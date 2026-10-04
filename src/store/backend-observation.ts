import { normalizeError } from "@/lib/error-handler";
import { ipc } from "@/lib/ipc";
import type { BackendApi, RequestController } from "./backend-store-types";

export function observationActions(api: BackendApi, controller: RequestController) {
  const refreshSnapshot = async () => {
    if (controller.transitions > 0) return;
    const current = controller.begin("snapshot");
    const snapshot = await ipc("get_runtime_snapshot");
    if (current()) api.getState().applySnapshot(snapshot);
  };
  const refreshConnections = async () => {
    const current = controller.begin("connections");
    try {
      const connections = await ipc("get_active_connections");
      if (current()) api.setState({ connections });
    } catch {
      if (current())
        api.setState({
          connections: {
            status: "degraded",
            active_count: null,
            recent: [],
            diagnostic: "活跃连接不可用",
            history_available: false,
          },
        });
    }
  };
  const refreshProfiles = async () => {
    const current = controller.begin("profiles");
    const profiles = await ipc("list_profiles");
    if (current()) api.setState({ profiles });
  };
  const refreshRules = async () => {
    const current = controller.begin("rules");
    const rules = await ipc("list_rules");
    if (current()) api.setState({ rules: rules ?? [] });
  };
  const refreshCapabilities = async () => {
    const current = controller.begin("capabilities");
    try {
      const capabilities = await ipc("get_capabilities");
      if (current()) api.setState({ capabilities: capabilities ?? null });
    } catch {
      if (current()) api.setState({ capabilities: null });
    }
  };
  const refresh = async () => {
    const current = controller.begin("refresh");
    try {
      await Promise.all([
        refreshSnapshot(),
        refreshProfiles(),
        refreshConnections(),
        refreshCapabilities(),
      ]);
      if (current()) api.setState({ observationError: null, error: api.getState().operationError });
    } catch (reason) {
      if (current()) {
        const observationError = normalizeError(reason);
        api.setState({
          observationError,
          error: api.getState().operationError ?? observationError,
        });
      }
    } finally {
      if (current()) api.setState({ loading: false });
    }
  };
  return { refreshSnapshot, refreshConnections, refreshProfiles, refreshRules, refresh };
}
