import { normalizeError } from "@/lib/error-handler";
import type { AppError } from "@/lib/generated/ipc";
import { ipc } from "@/lib/ipc";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { type ProxyProfile } from "@/lib/backend";

import { createLatencyRequest } from "./latencyTask";

type LatencyState = { latency?: number; error?: AppError; at?: number; pending?: boolean };

export function useProxyLatency(profiles: ProxyProfile[], available = true) {
  const [results, setResults] = useState<Record<string, LatencyState>>({});
  const [batchPending, setBatchPending] = useState(false);
  const active = useRef(false);
  const generation = useRef(0);
  const tokens = useRef(new Map<string, number>());
  const signatures = useRef(new Map<string, string>());
  const testUrl = useRef<string | null>(null);
  const running = useRef(new Set<string>());
  const requests = useRef(new Map<string, ReturnType<typeof createLatencyRequest>>());
  const batchRunning = useRef(false);

  const clear = useCallback((id: string) => {
    requests.current.get(id)?.cancel();
    requests.current.delete(id);
    tokens.current.set(id, (tokens.current.get(id) ?? 0) + 1);
    toast.dismiss(`proxy-latency-${id}`);
    setResults((current) => {
      const next = { ...current };
      delete next[id];
      return next;
    });
  }, []);

  useEffect(() => {
    active.current = true;
    const currentTokens = tokens.current;
    const currentRequests = requests.current;
    return () => {
      active.current = false;
      for (const request of currentRequests.values()) request.cancel();
      currentRequests.clear();
      generation.current += 1;
      for (const id of currentTokens.keys()) {
        currentTokens.set(id, (currentTokens.get(id) ?? 0) + 1);
        toast.dismiss(`proxy-latency-${id}`);
      }
    };
  }, []);

  useEffect(() => {
    if (!available) {
      generation.current += 1;
      for (const id of tokens.current.keys()) clear(id);
    }
  }, [available, clear]);

  useEffect(() => {
    const next = new Map(
      profiles.map((profile) => [
        profile.id,
        JSON.stringify([
          profile.configuration_revision,
          profile.protocol,
          profile.host,
          profile.port,
          profile.authentication_enabled,
          profile.enabled,
        ]),
      ]),
    );
    for (const [id, signature] of signatures.current) {
      if (next.get(id) !== signature) {
        generation.current += 1;
        clear(id);
      }
    }
    signatures.current = next;
  }, [profiles, clear]);

  useEffect(() => {
    let settingsActive = true;
    let request = 0;
    async function checkUrl() {
      const version = ++request;
      try {
        const settings = await ipc("get_settings");
        if (!settingsActive || request !== version) return;
        if (testUrl.current !== null && testUrl.current !== settings.latency_test_url) {
          generation.current += 1;
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
      settingsActive = false;
      window.removeEventListener("focus", checkUrl);
    };
  }, [clear]);

  const test = useCallback(
    async (id: string) => {
      if (
        !active.current ||
        !available ||
        running.current.has(id) ||
        !profiles.some((profile) => profile.id === id && profile.enabled)
      )
        return;
      running.current.add(id);
      const name = profiles.find((profile) => profile.id === id)?.name ?? "代理";
      const toastId = `proxy-latency-${id}`;
      const token = (tokens.current.get(id) ?? 0) + 1;
      tokens.current.set(id, token);
      setResults((current) => ({ ...current, [id]: { pending: true } }));
      toast.loading(`正在测试 ${name} 的延迟…`, { id: toastId });
      try {
        const revision = profiles.find((profile) => profile.id === id)!.configuration_revision;
        const request = createLatencyRequest(id, revision);
        requests.current.set(id, request);
        const result = await request.result;
        if (!result && active.current && tokens.current.get(id) === token) clear(id);
        if (result && active.current && tokens.current.get(id) === token) {
          setResults((current) => ({
            ...current,
            [id]: { latency: result.latency_ms, at: Date.now() },
          }));
          toast.success(`${name}：${result.latency_ms} ms`, { id: toastId });
        }
      } catch (reason) {
        if (active.current && tokens.current.get(id) === token) {
          const error = normalizeError(reason);
          setResults((current) => ({
            ...current,
            [id]: { error, at: Date.now() },
          }));
          toast.error(`${name} 延迟测试失败：${error.message}`, { id: toastId });
        }
      } finally {
        if (tokens.current.get(id) === token) requests.current.delete(id);
        running.current.delete(id);
      }
    },
    [profiles, available, clear],
  );

  async function testAll() {
    if (!active.current || !available || batchRunning.current) return;
    const version = generation.current;
    batchRunning.current = true;
    setBatchPending(true);
    const queue = profiles.filter((profile) => profile.enabled).map((profile) => profile.id);
    try {
      async function work(): Promise<void> {
        if (!active.current || version !== generation.current) return;
        const id = queue.shift();
        if (!id) return;
        await test(id);
        return work();
      }
      await Promise.all(Array.from({ length: Math.min(3, queue.length) }, work));
    } finally {
      batchRunning.current = false;
      if (active.current) setBatchPending(false);
    }
  }

  return { results, batchPending, test, testAll, clear };
}
