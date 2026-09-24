import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import {
  command,
  errorMessage,
  onRuntimeSnapshot,
  type ActiveConnectionsSnapshot,
  type ProxyMode,
  type ProxyProfile,
  type RuntimeSnapshot,
} from "@/lib/backend";

type BackendState = {
  snapshot: RuntimeSnapshot | null;
  profiles: ProxyProfile[];
  connections: ActiveConnectionsSnapshot | null;
  loading: boolean;
  pending: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  switchMode: (mode: ProxyMode) => Promise<void>;
};

const BackendContext = createContext<BackendState | null>(null);

export function BackendProvider({ children }: { children: ReactNode }) {
  const [snapshot, setSnapshot] = useState<RuntimeSnapshot | null>(null);
  const [profiles, setProfiles] = useState<ProxyProfile[]>([]);
  const [connections, setConnections] = useState<ActiveConnectionsSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refreshConnections = useCallback(async () => {
    try {
      setConnections(await command<ActiveConnectionsSnapshot>("get_active_connections"));
    } catch {
      setConnections({
        status: "degraded",
        active_count: null,
        recent: [],
        diagnostic: "活跃连接不可用",
        history_available: false,
      });
    }
  }, []);

  const refresh = useCallback(async () => {
    try {
      const [nextSnapshot, nextProfiles] = await Promise.all([
        command<RuntimeSnapshot>("get_runtime_snapshot"),
        command<ProxyProfile[]>("list_profiles"),
      ]);
      setSnapshot(nextSnapshot);
      setProfiles(nextProfiles);
      setError(null);
      await refreshConnections();
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setLoading(false);
    }
  }, [refreshConnections]);

  useEffect(() => {
    const timer = window.setTimeout(() => void refresh(), 0);
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void onRuntimeSnapshot((next) => {
      setSnapshot(next);
      void refreshConnections();
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => {
      disposed = true;
      window.clearTimeout(timer);
      unlisten?.();
    };
  }, [refresh, refreshConnections]);

  const switchMode = useCallback(
    async (mode: ProxyMode) => {
      if (pending) return;
      setPending(true);
      setError(null);
      try {
        const next = await command<RuntimeSnapshot>("set_runtime_mode", { mode });
        setSnapshot(next);
        await refreshConnections();
      } catch (reason) {
        setError(errorMessage(reason));
        try {
          setSnapshot(await command<RuntimeSnapshot>("get_runtime_snapshot"));
        } catch {
          /* Preserve last known state. */
        }
      } finally {
        setPending(false);
      }
    },
    [pending, refreshConnections],
  );

  const value = useMemo(
    () => ({ snapshot, profiles, connections, loading, pending, error, refresh, switchMode }),
    [snapshot, profiles, connections, loading, pending, error, refresh, switchMode],
  );
  return <BackendContext.Provider value={value}>{children}</BackendContext.Provider>;
}

export function useBackend(): BackendState {
  const value = useContext(BackendContext);
  if (!value) throw new Error("BackendProvider is missing");
  return value;
}
