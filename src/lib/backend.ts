import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type {
  RuntimeMode as ProxyMode,
  RuntimePhase,
  RuntimeSnapshot,
  Capabilities as BackendCapabilities,
  ProfileView as ProxyProfile,
  ActiveConnection,
  ActiveConnectionsSnapshot,
  AppError as BackendError,
} from "./generated/ipc";
export type { ProfileCredentialView as ProfileCredential } from "./generated/credentials";
import type { RuntimeSnapshot } from "./generated/ipc";

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

export function onRuntimeSnapshot(callback: (snapshot: RuntimeSnapshot) => void) {
  if (!isTauri()) return Promise.resolve(() => {});
  return listen<RuntimeSnapshot>("runtime://snapshot", (event) => callback(event.payload));
}
