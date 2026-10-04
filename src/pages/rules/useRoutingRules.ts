import { normalizeError } from "@/lib/error-handler";
import type { AppError, RoutingRule } from "@/lib/generated/ipc";
import { useCallback, useEffect, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { useBackendStore } from "@/store/backend-store";

export type { RoutingRule } from "@/lib/generated/ipc";

/** 规则数据由全局 store 持有；这里只保存当前页面的表单操作反馈。 */
export function useRoutingRules() {
  const { routingRules, refreshRules, replaceRules, reorderRules } = useBackendStore(
    useShallow((state) => ({
      routingRules: state.rules,
      refreshRules: state.refreshRules,
      replaceRules: state.replaceRules,
      reorderRules: state.reorderRules,
    })),
  );
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  const mounted = useRef(true);
  const writing = useRef(false);
  const refresh = useCallback(async () => {
    try {
      await refreshRules();
      if (mounted.current) setError(null);
    } catch (reason) {
      if (mounted.current) setError(normalizeError(reason));
    } finally {
      if (mounted.current) setLoading(false);
    }
  }, [refreshRules]);
  useEffect(() => {
    mounted.current = true;
    const timer = window.setTimeout(() => void refresh(), 0);
    return () => {
      mounted.current = false;
      window.clearTimeout(timer);
    };
  }, [refresh]);
  async function mutate(action: () => Promise<void>): Promise<boolean> {
    if (writing.current || !mounted.current) return false;
    writing.current = true;
    setBusy(true);
    setError(null);
    try {
      await action();
      return mounted.current;
    } catch (reason) {
      if (mounted.current) setError(normalizeError(reason));
      return false;
    } finally {
      writing.current = false;
      if (mounted.current) setBusy(false);
    }
  }
  const replace = (next: RoutingRule[]) => mutate(() => replaceRules(next));
  async function moveRule(index: number, delta: number) {
    const next = [...routingRules];
    const target = index + delta;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    await mutate(() => reorderRules(next.map((rule) => rule.id)));
  }
  return { routingRules, loading, busy, error, setError, replace, moveRule };
}
