import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { resolve, join } from "node:path";
import { createConnection } from "node:net";

const execute = promisify(execFile);

/** 读取本次原生进程树；不读取业务 SQLite、不调用 IPC、不输出原用户代理地址。 */
export async function inspectNative(reportDirectory, terminate = false) {
  const args = [
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    resolve("scripts/inspect-native-e2e.ps1"),
    "-Application",
    resolve("src-tauri/target/debug/socks-proxy.exe"),
    "-ProxyBeforePath",
    join(reportDirectory, "proxy-before.json"),
  ];
  if (terminate) args.push("-TerminateOwnedCore");
  const { stdout } = await execute(
    `${process.env.SystemRoot}/System32/WindowsPowerShell/v1.0/powershell.exe`,
    args,
    { timeout: 20_000, windowsHide: true, maxBuffer: 1024 * 1024 },
  );
  return JSON.parse(stdout.replace(/^\uFEFF/u, ""));
}

/** 经实际系统代理监听端口验证合成 HTTP CONNECT 隧道双向数据。 */
export async function probeTunnel(port) {
  assert.ok(Number.isInteger(port) && port > 0 && port < 65536, "缺少本次内核代理端口");
  await new Promise((done, reject) => {
    const socket = createConnection({ host: "127.0.0.1", port });
    let tunnel = false;
    let received = "";
    const fail = (error) => {
      socket.destroy();
      reject(error);
    };
    socket.setTimeout(5000, () => fail(new Error("实际代理隧道超时")));
    socket.on("error", fail);
    socket.on("end", () => {
      if (!received.endsWith("E2E")) reject(new Error("隧道未返回完整合成响应"));
    });
    socket.on("connect", () =>
      socket.write("CONNECT soak.invalid:80 HTTP/1.1\r\nHost: soak.invalid:80\r\n\r\n"),
    );
    socket.on("data", (chunk) => {
      received += chunk.toString();
      if (!tunnel && received.includes("\r\n\r\n")) {
        if (!/^HTTP\/1\.[01] 200 /u.test(received)) return fail(new Error("CONNECT 失败"));
        tunnel = true;
        received = "";
        socket.write("GET / HTTP/1.1\r\nHost: soak.invalid\r\nConnection: close\r\n\r\n");
      } else if (tunnel && received.endsWith("E2E")) {
        socket.destroy();
        done();
      }
    });
  });
}
