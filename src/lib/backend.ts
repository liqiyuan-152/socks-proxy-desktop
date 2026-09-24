import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type ProxyMode = "rules" | "global" | "direct";
export type RuntimePhase =
  | "stopped"
  | "starting"
  | "running"
  | "switching"
  | "recovering"
  | "failed";
export type RuntimeSnapshot = {
  revision: number;
  desired_mode: ProxyMode;
  applied_mode: ProxyMode | null;
  phase: RuntimePhase;
  active_profile_id: string | null;
  runtime_uptime_ms: number | null;
  system_proxy_enabled: boolean;
  tun_enabled: boolean;
  coverage: "none" | "system_proxy_apps";
  last_error: string | null;
};

export type ProxyProfile = {
  id: string;
  name: string;
  protocol: "socks5" | "http";
  host: string;
  port: number;
  authentication_enabled: boolean;
  enabled: boolean;
};

export type ActiveConnection = {
  id: string;
  started_at: string;
  target_host: string;
  target_port: number;
  matched_rule: string | null;
  outbound_chain: string[];
};

export type ActiveConnectionsSnapshot = {
  status: "available" | "degraded";
  active_count: number | null;
  recent: ActiveConnection[];
  diagnostic: string | null;
  history_available: false;
};

export type BackendError = {
  code: string;
  message: string;
  fields: { field: string; message: string }[];
};

export function errorMessage(error: unknown): string {
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    return error.message;
  }
  return "操作失败，请检查运行时状态。";
}

export async function command<T>(
  name: string,
  arguments_: Record<string, unknown> = {},
): Promise<T> {
  if (!isTauri()) {
    throw {
      code: "unavailable",
      message: "仅在桌面应用中可使用代理后端。",
      fields: [],
    } satisfies BackendError;
  }
  return invoke<T>(name, arguments_);
}

export function onRuntimeSnapshot(callback: (snapshot: RuntimeSnapshot) => void) {
  if (!isTauri()) return Promise.resolve(() => {});
  return listen<RuntimeSnapshot>("runtime://snapshot", (event) => callback(event.payload));
}
