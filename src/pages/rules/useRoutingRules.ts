import { ipc } from "@/lib/ipc";
import { useCallback, useEffect, useRef, useState } from "react";
import { errorMessage, type BackendError } from "@/lib/backend";

import type { RoutingRule } from "@/lib/generated/ipc";
export type { RoutingRule } from "@/lib/generated/ipc";

export function useRoutingRules() {
  const [routingRules, setRoutingRules] = useState<RoutingRule[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const mounted = useRef(true);
  const readGeneration = useRef(0);
  const writing = useRef(false);
  const refresh = useCallback(async () => {
    const request = ++readGeneration.current;
    try {
      const rules = await ipc("list_rules");
      if (!mounted.current || request !== readGeneration.current) return;
      setRoutingRules(rules);
      setError(null);
    } catch (reason) {
      if (mounted.current && request === readGeneration.current) setError(errorMessage(reason));
    } finally {
      if (mounted.current && request === readGeneration.current) setLoading(false);
    }
  }, []);
  useEffect(() => {
    const generation = readGeneration;
    mounted.current = true;
    const timer = window.setTimeout(() => void refresh(), 0);
    return () => {
      mounted.current = false;
      generation.current++;
      window.clearTimeout(timer);
    };
  }, [refresh]);

  async function mutate(action: () => Promise<unknown>): Promise<boolean> {
    if (writing.current || !mounted.current) return false;
    writing.current = true;
    readGeneration.current++;
    setBusy(true);
    setError(null);
    try {
      await action();
      if (!mounted.current) return false;
      await refresh();
      return mounted.current;
    } catch (reason) {
      if (!mounted.current) return false;
      const typed = reason as Partial<BackendError>;
      setError(
        typed.fields?.map((field) => `${field.field}: ${field.message}`).join("；") ||
          errorMessage(reason),
      );
      return false;
    } finally {
      writing.current = false;
      if (mounted.current) setBusy(false);
    }
  }

  function replace(next: RoutingRule[]): Promise<boolean> {
    return mutate(() => ipc("replace_rules", { rules: next }));
  }

  async function moveRule(index: number, delta: number) {
    const next = [...routingRules];
    const target = index + delta;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    await mutate(() => ipc("reorder_rules", { ids: next.map((rule) => rule.id) }));
  }

  return { routingRules, loading, busy, error, setError, replace, moveRule };
}
