import { createContext, useContext } from "react";
import { createStore, useStore } from "zustand";
import { devtools } from "zustand/middleware";
import { onRuntimeSnapshot } from "@/lib/backend";
import { normalizeError } from "@/lib/error-handler";
import { businessActions } from "./backend-business";
import { modeActions } from "./backend-modes";
import { observationActions } from "./backend-observation";
import { startRuntimePolling } from "./backend-polling";
import { RequestController, type BackendApi, type BackendStore } from "./backend-store-types";

export type { BackendStore } from "./backend-store-types";
export const BackendStoreContext = createContext<BackendApi | null>(null);

/** 每个应用 Provider 只有一个 store，避免跨窗口及测试实例共享生命周期。 */
export function createBackendStore(): BackendApi {
  const controller = new RequestController();
  let initialized = false;
  let stopPolling: (() => void) | undefined;
  let unlisten: (() => void) | undefined;
  return createStore<BackendStore>()(
    devtools(
      (_set, _get, api) => {
        const observation = observationActions(api, controller);
        const modes = modeActions(api, controller);
        const business = businessActions(api, controller);
        const reportError = (reason: unknown) => {
          if (!controller.active) return;
          const observationError = normalizeError(reason);
          api.setState({
            observationError,
            error: api.getState().operationError ?? observationError,
          });
        };
        const initialize = async () => {
          if (initialized) return;
          initialized = true;
          controller.active = true;
          const lifetime = controller.lifetime;
          const current = () => controller.active && controller.lifetime === lifetime;
          stopPolling = startRuntimePolling(async () => {
            await Promise.allSettled([
              observation.refreshSnapshot(),
              observation.refreshConnections(),
            ]);
          });
          void onRuntimeSnapshot((snapshot) => {
            if (!current()) return;
            modes.applySnapshot(snapshot);
            void observation.refreshConnections();
          })
            .then((stop) => {
              if (current()) unlisten = stop;
              else stop();
            })
            .catch((reason: unknown) => {
              if (current()) reportError(reason);
            });
          await Promise.all([
            observation.refresh(),
            observation.refreshRules().catch((reason: unknown) => {
              if (current()) reportError(reason);
            }),
          ]);
        };
        const dispose = () => {
          initialized = false;
          controller.invalidate();
          modes.cancelQueued();
          stopPolling?.();
          unlisten?.();
          stopPolling = undefined;
          unlisten = undefined;
        };
        return {
          capabilities: null,
          snapshot: null,
          profiles: [],
          rules: [],
          connections: null,
          loading: true,
          pending: false,
          selectedMode: null,
          error: null,
          operationError: null,
          observationError: null,
          ...observation,
          ...business,
          applySnapshot: modes.applySnapshot,
          beginTransition: modes.beginTransition,
          switchMode: modes.switchMode,
          reportError,
          initialize,
          dispose,
        };
      },
      {
        name: "socks-proxy/backend",
        enabled: import.meta.env.DEV,
        anonymousActionType: "backend/update",
      },
    ),
  );
}

/** 组件使用选择器订阅所需字段，业务写操作共享同一个 store。 */
export function useBackendStore(): BackendStore;
export function useBackendStore<T>(selector: (state: BackendStore) => T): T;
export function useBackendStore(
  selector: (state: BackendStore) => unknown = (state) => state,
): unknown {
  const store = useContext(BackendStoreContext);
  if (!store) throw new Error("BackendProvider is missing");
  return useStore(store, selector);
}

export const selectSelectedMode = (state: BackendStore) =>
  state.selectedMode ?? state.snapshot?.selected_mode ?? null;
export const selectIsRunning = (state: BackendStore) => state.snapshot?.phase === "running";
export const selectAppliedMode = (state: BackendStore) => state.snapshot?.applied_mode ?? null;
