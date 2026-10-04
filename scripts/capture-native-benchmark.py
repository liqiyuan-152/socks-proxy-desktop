"""读取已由 CUA 启动的隔离 macOS 应用；不启动或操作任何界面。"""

import argparse
import json
import statistics
import subprocess
import time
from pathlib import Path


def processes():
    output = subprocess.check_output(
        ["ps", "-axo", "pid=,ppid=,rss=,command="], text=True
    )
    result = {}
    for line in output.splitlines():
        columns = line.strip().split(None, 3)
        if len(columns) == 4:
            pid, parent, rss, command = columns
            result[int(pid)] = {
                "pid": int(pid),
                "parent": int(parent),
                "rss_bytes": int(rss) * 1024,
                "command": command,
            }
    return result


def capture(arguments):
    report_path = Path(arguments.report)
    initial = json.loads(report_path.read_text())
    if len(initial.get("ipc_ms", [])) != 200:
        raise ValueError("就绪报告尚未完成 200 次真实 JavaScript IPC 往返采样")
    pid = initial["pid"]
    executable = str(Path(arguments.executable).resolve())
    samples = []
    for _ in range(arguments.samples):
        inventory = processes()
        root = inventory.get(pid)
        if root is None or str(Path(root["command"]).resolve()) != executable:
            raise ValueError("报告进程已退出或实际可执行路径不符合隔离应用")
        descendants = {pid}
        while True:
            added = {
                item["pid"]
                for item in inventory.values()
                if item["parent"] in descendants
            }
            expanded = descendants | added
            if expanded == descendants:
                break
            descendants = expanded
        members = [inventory[member] for member in sorted(descendants)]
        samples.append(
            {
                "root_rss_bytes": root["rss_bytes"],
                "process_tree_rss_bytes": sum(item["rss_bytes"] for item in members),
                "processes": members,
            }
        )
        time.sleep(arguments.interval)
    final = json.loads(report_path.read_text())
    if initial != final:
        raise ValueError("采样期间就绪报告发生变化")
    sorted_ipc = sorted(initial["ipc_ms"])
    captured = {
        "label": arguments.label,
        "platform": "macOS aarch64",
        "startup_ms": initial["startup_ms"],
        "ipc": {
            "samples": 200,
            "warmup": 20,
            "median_ms": statistics.median(sorted_ipc),
            "p95_ms": sorted_ipc[189],
            "minimum_ms": sorted_ipc[0],
            "maximum_ms": sorted_ipc[-1],
            "raw_ms": initial["ipc_ms"],
        },
        "memory": {
            "root_median_rss_bytes": statistics.median(
                sample["root_rss_bytes"] for sample in samples
            ),
            "tree_median_rss_bytes": statistics.median(
                sample["process_tree_rss_bytes"] for sample in samples
            ),
            "interval_seconds": arguments.interval,
            "samples": samples,
        },
        "limitations": [
            "启动为 Rust run 至初始数据可用、页面挂载并跨两个可见帧后的 IPC 确认",
            "不含 OS 创建进程耗时，不是像素首绘，也不是 Windows 原生性能",
            "内存为 RSS，不等同私有内存；共享页可能重复计入进程树总和",
            "只统计实际应用及当前父子进程树，OS 共享 WebKit 服务不一定在树内",
            "IPC 为无参数 bool 命令的真实 WebView 往返，不代表大数据序列化",
        ],
    }
    destination = Path(arguments.output)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(captured, ensure_ascii=False, indent=2) + "\n")
    print(
        json.dumps(
            {
                "label": captured["label"],
                "startup_ms": captured["startup_ms"],
                "ipc_median_ms": captured["ipc"]["median_ms"],
                "root_rss_bytes": captured["memory"]["root_median_rss_bytes"],
                "tree_rss_bytes": captured["memory"]["tree_median_rss_bytes"],
            }
        )
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--label", required=True)
    parser.add_argument("--report", required=True)
    parser.add_argument("--executable", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--samples", type=int, default=10)
    parser.add_argument("--interval", type=float, default=1.0)
    arguments = parser.parse_args()
    if not 1 <= arguments.samples <= 60 or not 0 <= arguments.interval <= 1:
        parser.error("samples 必须为 1–60，interval 必须为 0–1 秒")
    capture(arguments)
