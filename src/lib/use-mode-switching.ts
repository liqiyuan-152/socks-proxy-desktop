import { ipc } from "@/lib/ipc";
import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import { errorMessage, type ProxyMode, type RuntimeSnapshot } from "@/lib/backend";

type ModeCallbacks = {
  applySnapshot: (snapshot: RuntimeSnapshot) => void;
  beginTransition: () => () => void;
  refreshConnections: () => Promise<void>;
};

export function useModeSwitching({
  applySnapshot,
  beginTransition,
  refreshConnections,
}: ModeCallbacks) {
  const [pending, setPending] = useState(false);
  const [selectedMode, setSelectedMode] = useState<ProxyMode | null>(null);
  const [operationError, setOperationError] = useState<string | null>(null);
  const failedOperation = useRef<number | null>(null);
  const selected = useRef<ProxyMode | null>(null);
  const queued = useRef<ProxyMode | null>(null);
  const flight = useRef<Promise<void> | null>(null);
  const active = useRef(true);

  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
      queued.current = null;
    };
  }, []);

  const observeSnapshot = useCallback((next: RuntimeSnapshot) => {
    if (
      failedOperation.current !== null &&
      next.last_operation.id > failedOperation.current &&
      next.last_operation.outcome === "succeeded"
    ) {
      failedOperation.current = null;
      setOperationError(null);
    }
    if (
      !flight.current &&
      failedOperation.current === null &&
      selected.current !== null &&
      next.desired_mode !== selected.current
    ) {
      selected.current = null;
      setSelectedMode(null);
    }
  }, []);

  const reportFailure = useCallback((reason: unknown, next?: RuntimeSnapshot) => {
    if (!active.current) return;
    failedOperation.current = next?.last_operation.id ?? Number.MAX_SAFE_INTEGER;
    setOperationError(errorMessage(reason));
  }, []);

  const drain = useCallback(async () => {
    const endTransition = beginTransition();
    let succeeded = false;
    try {
      while (active.current && queued.current !== null) {
        const target = queued.current;
        queued.current = null;
        // Runtime mutations must finish serially before applying the latest queued mode.
        // oxlint-disable-next-line no-await-in-loop
        succeeded = await applyMode(target, applySnapshot, reportFailure, active);
      }
      if (active.current && succeeded) {
        selected.current = null;
        failedOperation.current = null;
        setOperationError(null);
        setSelectedMode(null);
      }
    } finally {
      endTransition();
      flight.current = null;
      if (active.current) setPending(false);
    }
    if (active.current) await refreshConnections();
    // The async loop forwards applySnapshot to applyMode, so its identity is a dependency.
    // oxlint-disable-next-line react/memo-dependencies
  }, [applySnapshot, beginTransition, refreshConnections, reportFailure]);

  const switchMode = useCallback(
    (mode: ProxyMode): Promise<void> => {
      if (!active.current) return Promise.resolve();
      selected.current = mode;
      setSelectedMode(mode);
      queued.current = mode;
      failedOperation.current = null;
      setOperationError(null);
      // All callers await the same drain; intermediate clicks are coalesced.
      if (flight.current) return flight.current;
      setPending(true);
      flight.current = drain();
      return flight.current;
    },
    [drain],
  );

  return { pending, selectedMode, operationError, switchMode, observeSnapshot };
}

async function applyMode(
  mode: ProxyMode,
  applySnapshot: (snapshot: RuntimeSnapshot) => void,
  reportFailure: (reason: unknown, next?: RuntimeSnapshot) => void,
  active: RefObject<boolean>,
): Promise<boolean> {
  try {
    const next = await ipc("set_runtime_mode", { mode });
    if (active.current) applySnapshot(next);
    return true;
  } catch (reason) {
    reportFailure(reason);
    if (active.current) {
      try {
        const next = await ipc("get_runtime_snapshot");
        if (active.current) {
          applySnapshot(next);
          reportFailure(reason, next);
        }
      } catch {
        // Keep the last authoritative snapshot if recovery cannot be read.
      }
    }
    return false;
  }
}
