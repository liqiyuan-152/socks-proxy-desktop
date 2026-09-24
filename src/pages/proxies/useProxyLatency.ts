import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { command, errorMessage, type ProxyProfile } from "@/lib/backend";

type LatencyState = { latency?: number; error?: string; at?: number; pending?: boolean };
type TestSettings = { latency_test_url: string };

export function useProxyLatency(profiles: ProxyProfile[]) {
  const [results, setResults] = useState<Record<string, LatencyState>>({});
  const [batchPending, setBatchPending] = useState(false);
  const tokens = useRef(new Map<string, number>());
  const signatures = useRef(new Map<string, string>());
  const testUrl = useRef<string | null>(null);
  const running = useRef(new Set<string>());
  const batchRunning = useRef(false);

  const clear = useCallback((id: string) => {
    tokens.current.set(id, (tokens.current.get(id) ?? 0) + 1);
    toast.dismiss(`proxy-latency-${id}`);
    setResults((current) => {
      const next = { ...current };
      delete next[id];
      return next;
    });
  }, []);

  useEffect(() => {
    const next = new Map(
      profiles.map((profile) => [
        profile.id,
        JSON.stringify([
          profile.protocol,
          profile.host,
          profile.port,
          profile.authentication_enabled,
          profile.enabled,
        ]),
      ]),
    );
    for (const [id, signature] of signatures.current) {
      if (next.get(id) !== signature) clear(id);
    }
    signatures.current = next;
  }, [profiles, clear]);

  useEffect(() => {
    let active = true;
    async function checkUrl() {
      try {
        const settings = await command<TestSettings>("get_settings");
        if (!active) return;
        if (testUrl.current !== null && testUrl.current !== settings.latency_test_url) {
          for (const id of tokens.current.keys()) clear(id);
        }
        testUrl.current = settings.latency_test_url;
      } catch {
        // A failed settings refresh does not invalidate completed tests.
      }
    }
    void checkUrl();
    window.addEventListener("focus", checkUrl);
    return () => {
      active = false;
      window.removeEventListener("focus", checkUrl);
    };
  }, [clear]);

  const test = useCallback(
    async (id: string) => {
      if (running.current.has(id)) return;
      running.current.add(id);
      const name = profiles.find((profile) => profile.id === id)?.name ?? "代理";
      const toastId = `proxy-latency-${id}`;
      const token = (tokens.current.get(id) ?? 0) + 1;
      tokens.current.set(id, token);
      setResults((current) => ({ ...current, [id]: { pending: true } }));
      toast.loading(`正在测试 ${name} 的延迟…`, { id: toastId });
      try {
        const result = await command<{ latency_ms: number }>("test_proxy_latency", { id });
        if (tokens.current.get(id) === token) {
          setResults((current) => ({
            ...current,
            [id]: { latency: result.latency_ms, at: Date.now() },
          }));
          toast.success(`${name}：${result.latency_ms} ms`, { id: toastId });
        }
      } catch (reason) {
        if (tokens.current.get(id) === token) {
          const message = errorMessage(reason);
          setResults((current) => ({
            ...current,
            [id]: { error: message, at: Date.now() },
          }));
          toast.error(`${name} 延迟测试失败：${message}`, { id: toastId });
        }
      } finally {
        running.current.delete(id);
      }
    },
    [profiles],
  );

  async function testAll() {
    if (batchRunning.current) return;
    batchRunning.current = true;
    setBatchPending(true);
    const queue = profiles.filter((profile) => profile.enabled).map((profile) => profile.id);
    try {
      async function work(): Promise<void> {
        const id = queue.shift();
        if (!id) return;
        await test(id);
        return work();
      }
      await Promise.all(Array.from({ length: Math.min(3, queue.length) }, work));
    } finally {
      batchRunning.current = false;
      setBatchPending(false);
    }
  }

  return { results, batchPending, test, testAll, clear };
}
