import { useState, type FormEvent } from "react";
import { Search } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { command, errorMessage } from "@/lib/backend";

type RouteTestResult = {
  stage: "user_rule" | "china_domain" | "private_ip" | "china_ip" | "final";
  action: "proxy" | "direct";
  proxy_profile_id: string | null;
  proxy_name: string | null;
  matched_rule_id: string | null;
  matched_rule_name: string | null;
  reason: string;
  data_date: string | null;
};

const stageLabels: Record<RouteTestResult["stage"], string> = {
  user_rule: "用户规则",
  china_domain: "中国域名集",
  private_ip: "私有或本地 IP",
  china_ip: "中国 IP 集",
  final: "最终出口",
};

export function RouteTest() {
  const [target, setTarget] = useState("");
  const [port, setPort] = useState("443");
  const [result, setResult] = useState<RouteTestResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function test(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setResult(null);
    setError(null);
    const parsedPort = Number(port);
    if (!target.trim() || !Number.isInteger(parsedPort) || parsedPort < 1 || parsedPort > 65535) {
      setError("请输入有效的目标域名或 IP，以及 1–65535 的端口。");
      return;
    }
    setBusy(true);
    try {
      setResult(
        await command<RouteTestResult>("test_route", { target: target.trim(), port: parsedPort }),
      );
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="border-b border-border pb-5" aria-labelledby="route-test-title">
      <h2 id="route-test-title" className="text-base font-semibold">
        规则测试
      </h2>
      <form onSubmit={(event) => void test(event)} className="mt-3 flex flex-wrap items-end gap-3">
        <label className="min-w-48 flex-1 text-sm">
          目标域名或 IP
          <Input
            className="mt-1"
            value={target}
            onChange={(event) => setTarget(event.target.value)}
            placeholder="example.com 或 1.0.1.1"
          />
        </label>
        <label className="w-28 text-sm">
          端口
          <Input
            className="mt-1"
            type="number"
            min={1}
            max={65535}
            value={port}
            onChange={(event) => setPort(event.target.value)}
          />
        </label>
        <Button type="submit" disabled={busy}>
          <Search className="size-4" aria-hidden="true" />
          {busy ? "测试中…" : "测试"}
        </Button>
      </form>
      {error && (
        <p role="alert" className="mt-3 text-sm text-destructive">
          {error}
        </p>
      )}
      {result && (
        <div role="status" className="mt-3 space-y-1 text-sm">
          <p className="font-medium">
            {result.action === "direct" ? "直连" : `代理：${result.proxy_name ?? "未知出口"}`}
            {result.matched_rule_name ? ` · 规则：${result.matched_rule_name}` : ""}
          </p>
          <p className="text-muted-foreground">命中阶段：{stageLabels[result.stage]}</p>
          <p className="text-muted-foreground">{result.reason}</p>
          <p className="text-xs text-muted-foreground">
            按已保存配置预测规则模式的出口，不代表已建立连接或实际拨号地址。
            {result.data_date ? ` 规则集日期：${result.data_date}。` : ""}
          </p>
        </div>
      )}
    </section>
  );
}
