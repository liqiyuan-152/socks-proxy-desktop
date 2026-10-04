import { ipc } from "@/lib/ipc";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  errorMessage,
  type ActiveConnectionsSnapshot,
  type BackendCapabilities,
  type ProxyProfile,
  type RuntimeSnapshot,
} from "@/lib/backend";
import { useRuntimePolling } from "@/lib/use-runtime-polling";

type Resource = "snapshot" | "connections" | "profiles" | "capabilities" | "refresh";

export function useBackendObservation() {
  const [capabilities, setCapabilities] = useState<BackendCapabilities | null>(null);
  const [snapshot, setSnapshot] = useState<RuntimeSnapshot | null>(null);
  const [profiles, setProfiles] = useState<ProxyProfile[]>([]);
  const [connections, setConnections] = useState<ActiveConnectionsSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const requests = useRef({
    snapshot: 0,
    connections: 0,
    profiles: 0,
    capabilities: 0,
    refresh: 0,
  });
  const active = useRef(true);
  const lifecycle = useRef(0);
  const transitions = useRef(0);

  // Reads compete only with newer reads of the same resource.
  const beginRequest = useCallback((resource: Resource) => {
    const version = ++requests.current[resource];
    const lifetime = lifecycle.current;
    return () =>
      active.current && lifetime === lifecycle.current && version === requests.current[resource];
  }, []);

  const applySnapshot = useCallback((next: RuntimeSnapshot) => {
    requests.current.snapshot += 1;
    if (active.current) setSnapshot(next);
  }, []);

  const beginTransition = useCallback(() => {
    transitions.current += 1;
    requests.current.snapshot += 1;
    return () => {
      transitions.current -= 1;
      requests.current.snapshot += 1;
    };
  }, []);

  const readSnapshot = useCallback(async () => {
    if (transitions.current > 0) return;
    const isCurrent = beginRequest("snapshot");
    const next = await ipc("get_runtime_snapshot");
    if (isCurrent()) setSnapshot(next);
  }, [beginRequest]);

  const refreshConnections = useCallback(async () => {
    const isCurrent = beginRequest("connections");
    try {
      const next = await ipc("get_active_connections");
      if (isCurrent()) setConnections(next);
    } catch {
      if (isCurrent())
        setConnections({
          status: "degraded",
          active_count: null,
          recent: [],
          diagnostic: "活跃连接不可用",
          history_available: false,
        });
    }
  }, [beginRequest]);

  const readProfiles = useCallback(async () => {
    const isCurrent = beginRequest("profiles");
    const next = await ipc("list_profiles");
    if (isCurrent()) setProfiles(next);
  }, [beginRequest]);

  const readCapabilities = useCallback(async () => {
    const isCurrent = beginRequest("capabilities");
    try {
      const next = await ipc("get_capabilities");
      if (isCurrent()) setCapabilities(next ?? null);
    } catch {
      if (isCurrent()) setCapabilities(null);
    }
  }, [beginRequest]);

  const refresh = useCallback(async () => {
    const isCurrent = beginRequest("refresh");
    try {
      await Promise.all([readSnapshot(), readProfiles(), refreshConnections(), readCapabilities()]);
      if (isCurrent()) setError(null);
    } catch (reason) {
      if (isCurrent()) setError(errorMessage(reason));
    } finally {
      if (isCurrent()) setLoading(false);
    }
  }, [beginRequest, readSnapshot, readProfiles, refreshConnections, readCapabilities]);

  const poll = useCallback(async () => {
    await Promise.allSettled([readSnapshot(), refreshConnections()]);
  }, [readSnapshot, refreshConnections]);

  useEffect(() => {
    active.current = true;
    const timer = window.setTimeout(() => void refresh(), 0);
    return () => {
      active.current = false;
      lifecycle.current += 1;
      window.clearTimeout(timer);
    };
  }, [refresh]);
  useRuntimePolling(poll);

  const reportError = useCallback((reason: unknown) => {
    if (active.current) setError(errorMessage(reason));
  }, []);

  return {
    capabilities,
    snapshot,
    profiles,
    connections,
    loading,
    error,
    refresh,
    refreshConnections,
    applySnapshot,
    beginTransition,
    reportError,
  };
}
