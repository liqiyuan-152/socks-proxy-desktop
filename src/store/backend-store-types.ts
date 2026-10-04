import type { StoreApi } from "zustand";
import type {
  ActiveConnectionsSnapshot,
  AppError,
  AppSettings,
  Capabilities,
  CommandMap,
  NetworkRecoveryResult,
  ProfileView,
  RoutingRule,
  RuntimeMode,
  RuntimeSnapshot,
} from "@/lib/generated/ipc";

export type BackendStore = {
  capabilities: Capabilities | null;
  snapshot: RuntimeSnapshot | null;
  profiles: ProfileView[];
  rules: RoutingRule[];
  connections: ActiveConnectionsSnapshot | null;
  loading: boolean;
  pending: boolean;
  selectedMode: RuntimeMode | null;
  error: AppError | null;
  observationError: AppError | null;
  operationError: AppError | null;
  initialize: () => Promise<void>;
  dispose: () => void;
  refresh: () => Promise<void>;
  refreshSnapshot: () => Promise<void>;
  refreshConnections: () => Promise<void>;
  refreshProfiles: () => Promise<void>;
  refreshRules: () => Promise<void>;
  switchMode: (mode: RuntimeMode) => Promise<void>;
  applySnapshot: (snapshot: RuntimeSnapshot) => void;
  beginTransition: () => () => void;
  reportError: (reason: unknown) => void;
  saveProfile: (input: CommandMap["save_profile"][0]["input"]) => Promise<ProfileView>;
  deleteProfile: (id: string) => Promise<void>;
  selectProfile: (id: string | null) => Promise<void>;
  replaceRules: (rules: RoutingRule[]) => Promise<void>;
  reorderRules: (ids: string[]) => Promise<void>;
  updateSettings: (settings: AppSettings) => Promise<AppSettings>;
  importConfiguration: (input: CommandMap["import_configuration"][0]) => Promise<void>;
  stopRuntime: () => Promise<void>;
  recoverNetwork: () => Promise<NetworkRecoveryResult>;
};
export type BackendApi = StoreApi<BackendStore>;
export type Resource =
  | "snapshot"
  | "connections"
  | "profiles"
  | "rules"
  | "capabilities"
  | "refresh";

/** 每个 Provider 生命周期独立；请求只与同资源的新请求竞争。 */
export class RequestController {
  active = true;
  lifetime = 0;
  transitions = 0;
  versions: Record<Resource, number> = {
    snapshot: 0,
    connections: 0,
    profiles: 0,
    rules: 0,
    capabilities: 0,
    refresh: 0,
  };
  begin(resource: Resource) {
    const version = ++this.versions[resource];
    const lifetime = this.lifetime;
    return () => this.active && lifetime === this.lifetime && version === this.versions[resource];
  }
  invalidate() {
    this.active = false;
    this.lifetime++;
  }
}
