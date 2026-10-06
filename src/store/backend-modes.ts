import { sameMode } from "@/lib/proxy-mode";
import type { RuntimeMode, RuntimeSnapshot } from "@/lib/generated/ipc";
import { normalizeError } from "@/lib/error-handler";
import { setRuntimeMode } from "@/lib/backend";
import { ipc } from "@/lib/ipc";
import type { BackendApi, RequestController } from "./backend-store-types";

export function modeActions(api: BackendApi, controller: RequestController) {
  let failedOperation: number | null = null;
  let queued: RuntimeMode | null = null;
  let flight: Promise<void> | null = null;
  let flightToken = 0;
  const applySnapshot = (snapshot: RuntimeSnapshot) => {
    controller.versions.snapshot++;
    if (!controller.active) return;
    const state = api.getState();
    let operationError = state.operationError;
    let selectedMode = state.selectedMode;
    if (
      failedOperation !== null &&
      snapshot.last_operation.id > failedOperation &&
      snapshot.last_operation.outcome === "succeeded"
    ) {
      failedOperation = null;
      operationError = null;
    }
    if (
      !flight &&
      failedOperation === null &&
      selectedMode !== null &&
      !sameMode(snapshot.desired_mode, selectedMode)
    )
      selectedMode = null;
    api.setState({
      snapshot,
      selectedMode,
      operationError,
      error: operationError ?? state.observationError,
    });
  };
  const beginTransition = () => {
    controller.transitions++;
    controller.versions.snapshot++;
    let finished = false;
    return () => {
      if (finished) return;
      finished = true;
      controller.transitions--;
      controller.versions.snapshot++;
    };
  };
  const reportFailure = (reason: unknown, snapshot?: RuntimeSnapshot) => {
    failedOperation = snapshot?.last_operation.id ?? Number.MAX_SAFE_INTEGER;
    const operationError = normalizeError(reason);
    api.setState({ selectedMode: queued, operationError, error: operationError });
  };
  const drain = async (token: number) => {
    const lifetime = controller.lifetime;
    const current = () => controller.active && controller.lifetime === lifetime;
    const finish = beginTransition();
    let succeeded = false;
    try {
      while (current() && queued !== null) {
        const mode = queued;
        queued = null;
        try {
          // 串行完成当前写操作，再应用最后一个排队选择。
          // oxlint-disable-next-line no-await-in-loop
          const snapshot = await setRuntimeMode(mode);
          if (current()) applySnapshot(snapshot);
          succeeded = true;
        } catch (reason) {
          succeeded = false;
          if (current()) {
            reportFailure(reason);
            try {
              // oxlint-disable-next-line no-await-in-loop
              const snapshot = await ipc("get_runtime_snapshot");
              if (current()) {
                applySnapshot(snapshot);
                reportFailure(reason, snapshot);
              }
            } catch {
              /* 保留最近一次权威快照。 */
            }
          }
        }
      }
      if (current() && succeeded) {
        failedOperation = null;
        api.setState({
          selectedMode: null,
          operationError: null,
          error: api.getState().observationError,
        });
      }
    } finally {
      finish();
      if (token === flightToken) flight = null;
      if (current()) api.setState({ pending: false });
    }
    if (current()) await api.getState().refreshConnections();
  };
  const switchMode = (mode: RuntimeMode): Promise<void> => {
    if (!controller.active) return Promise.resolve();
    queued = mode;
    failedOperation = null;
    api.setState({
      selectedMode: mode,
      operationError: null,
      error: api.getState().observationError,
    });
    if (flight) return flight;
    api.setState({ pending: true });
    flight = drain(++flightToken);
    return flight;
  };
  return {
    applySnapshot,
    beginTransition,
    switchMode,
    cancelQueued: () => {
      queued = null;
      flight = null;
      flightToken++;
      failedOperation = null;
      api.setState({ pending: false, selectedMode: null });
    },
  };
}
