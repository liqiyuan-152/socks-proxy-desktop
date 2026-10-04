import { normalizeError } from "@/lib/error-handler";
import { ipc } from "@/lib/ipc";
import type { AppSettings, CommandMap, RoutingRule } from "@/lib/generated/ipc";
import type { BackendApi, RequestController } from "./backend-store-types";

/** 凭据只在调用参数中存在，永不保存到 store 或开发工具状态。 */
export function businessActions(api: BackendApi, controller: RequestController) {
  const mutate = async <T>(action: () => Promise<T>, rules = false): Promise<T> => {
    const lifetime = controller.lifetime;
    controller.versions.profiles++;
    controller.versions.rules++;
    const finish = api.getState().beginTransition();
    try {
      const result = await action();
      if (controller.active && lifetime === controller.lifetime) {
        finish();
        await api.getState().refresh();
        if (rules) await api.getState().refreshRules();
      }
      return result;
    } catch (reason) {
      throw normalizeError(reason);
    } finally {
      finish();
    }
  };
  return {
    saveProfile: (input: CommandMap["save_profile"][0]["input"]) =>
      mutate(() => ipc("save_profile", { input })),
    deleteProfile: async (id: string) => {
      await mutate(() => ipc("delete_profile", { id }));
    },
    selectProfile: async (id: string | null) => {
      await mutate(() => ipc("select_profile", { id }));
    },
    replaceRules: async (rules: RoutingRule[]) => {
      await mutate(() => ipc("replace_rules", { rules }), true);
    },
    reorderRules: async (ids: string[]) => {
      await mutate(() => ipc("reorder_rules", { ids }), true);
    },
    updateSettings: (settings: AppSettings) => mutate(() => ipc("update_settings", { settings })),
    importConfiguration: async (input: CommandMap["import_configuration"][0]) => {
      await mutate(() => ipc("import_configuration", input), true);
    },
    stopRuntime: async () => {
      await mutate(async () => {
        const lifetime = controller.lifetime;
        const snapshot = await ipc("stop_runtime");
        if (controller.active && lifetime === controller.lifetime)
          api.getState().applySnapshot(snapshot);
      });
    },
    recoverNetwork: async () => {
      return mutate(async () => {
        const lifetime = controller.lifetime;
        const result = await ipc("recover_network", { confirmed: true });
        if (controller.active && lifetime === controller.lifetime)
          api.getState().applySnapshot(result.snapshot);
        return result;
      });
    },
  };
}
