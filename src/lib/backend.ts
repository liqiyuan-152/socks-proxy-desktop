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
import { ipc } from "./ipc";
import type { RuntimeMode, RuntimeSnapshot } from "./generated/ipc";

export { errorMessage } from "./error-handler";

export function onRuntimeSnapshot(callback: (snapshot: RuntimeSnapshot) => void) {
  if (!isTauri()) return Promise.resolve(() => {});
  return listen<RuntimeSnapshot>("runtime://snapshot", (event) => callback(event.payload));
}

export function setRuntimeMode(mode: RuntimeMode): Promise<RuntimeSnapshot> {
  return ipc("set_runtime_mode", { mode });
}
