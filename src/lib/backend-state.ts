import { createContext, useContext } from "react";
import type {
  ActiveConnectionsSnapshot,
  BackendCapabilities,
  ProxyMode,
  ProxyProfile,
  RuntimeSnapshot,
} from "@/lib/backend";

type BackendState = {
  capabilities: BackendCapabilities | null;
  snapshot: RuntimeSnapshot | null;
  profiles: ProxyProfile[];
  connections: ActiveConnectionsSnapshot | null;
  loading: boolean;
  pending: boolean;
  selectedMode: ProxyMode | null;
  error: string | null;
  refresh: () => Promise<void>;
  switchMode: (mode: ProxyMode) => Promise<void>;
};

export const BackendContext = createContext<BackendState | null>(null);

export function useBackend(): BackendState {
  const value = useContext(BackendContext);
  if (!value) throw new Error("BackendProvider is missing");
  return value;
}
