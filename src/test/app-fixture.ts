import { beforeEach, vi } from "vitest";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn(async () => () => {}) }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));

export type Profile = {
  id: string;
  name: string;
  protocol: string;
  host: string;
  port: number;
  authentication_enabled: boolean;
  enabled: boolean;
};
export type Rule = {
  id: string;
  name: string;
  matcher: string;
  target: string;
  port_start: number | null;
  port_end: number | null;
  action: string;
  proxy_profile_id: string | null;
  enabled: boolean;
};

export const fixture = {} as {
  profiles: Profile[];
  rules: Rule[];
  defaultProfileId: string | null;
  mode: "global" | "rules" | "direct";
  settings: { launch_at_login: boolean; diagnostic_retention: string; latency_test_url: string };
  diagnostics: { id: string; created_at_ms: number; severity: string; summary: string }[];
  rejectMode: boolean;
  recoveryAvailable: boolean;
};

export function runtime() {
  return {
    revision: 1,
    selected_mode: fixture.mode,
    desired_mode: fixture.mode,
    applied_mode: fixture.mode,
    phase: fixture.mode === "direct" ? "stopped" : "running",
    active_profile_id: fixture.defaultProfileId,
    runtime_uptime_ms: fixture.mode === "direct" ? null : 1200,
    system_proxy_enabled: fixture.mode !== "direct",
    tun_enabled: false,
    coverage: fixture.mode === "direct" ? "none" : "system_proxy_apps",
    last_error: null,
  };
}

beforeEach(() => {
  window.history.pushState({}, "", "/");
  fixture.profiles = [
    {
      id: "primary",
      name: "Primary",
      protocol: "socks5",
      host: "proxy.example.org",
      port: 1080,
      authentication_enabled: true,
      enabled: true,
    },
  ];
  fixture.defaultProfileId = "primary";
  fixture.rules = [
    {
      id: "rule-1",
      name: "Example",
      matcher: "domain",
      target: "example.org",
      port_start: 443,
      port_end: null,
      action: "proxy",
      proxy_profile_id: "primary",
      enabled: true,
    },
  ];
  fixture.mode = "global";
  fixture.settings = {
    launch_at_login: false,
    diagnostic_retention: "days30",
    latency_test_url: "https://www.gstatic.com/generate_204",
  };
  fixture.diagnostics = [
    { id: "diagnostic-1", created_at_ms: Date.now(), severity: "info", summary: "代理模式已提交" },
  ];
  fixture.rejectMode = false;
  fixture.recoveryAvailable = false;
  mocks.invoke.mockReset();
  mocks.listen.mockClear();
  mocks.invoke.mockImplementation(async (command: string, args: Record<string, unknown> = {}) => {
    switch (command) {
      case "get_capabilities":
        return {
          platform: "test",
          proxy_runtime: true,
          proxy_latency: true,
          network_recovery: fixture.recoveryAvailable,
          startup: true,
        };
      case "get_runtime_snapshot":
        return runtime();
      case "list_profiles":
        return fixture.profiles;
      case "get_profile_credential":
        return { username: "alice", password: "stored-secret" };
      case "get_active_connections":
        return {
          status: "available",
          active_count: 1,
          history_available: false,
          diagnostic: null,
          recent: [
            {
              id: "active-1",
              started_at: "2026-09-23T01:00:00Z",
              target_host: "example.org",
              target_port: 443,
              matched_rule: "domain=example.org",
              outbound_chain: ["selected-proxy"],
            },
          ],
        };
      case "set_runtime_mode":
        if (fixture.rejectMode) throw { code: "unavailable", message: "内核启动失败", fields: [] };
        fixture.mode = args.mode as typeof fixture.mode;
        return runtime();
      case "recover_network":
        fixture.mode = "direct";
        return { completed_at_ms: Date.now(), snapshot: runtime() };
      case "save_profile": {
        const input = args.input as Profile;
        if (
          !input.enabled &&
          (input.id === fixture.defaultProfileId ||
            fixture.rules.some((rule) => rule.proxy_profile_id === input.id))
        ) {
          throw { code: "validation_error", message: "代理仍被默认出口或规则引用", fields: [] };
        }
        const profile = { ...input, id: input.id ?? "new-profile" };
        fixture.profiles = fixture.profiles
          .filter((item) => item.id !== profile.id)
          .concat(profile);
        return profile;
      }
      case "select_profile":
        fixture.defaultProfileId = args.id as string | null;
        return null;
      case "delete_profile":
        fixture.profiles = fixture.profiles.filter((item) => item.id !== args.id);
        return null;
      case "list_rules":
        return fixture.rules;
      case "replace_rules":
        fixture.rules = args.rules as Rule[];
        return null;
      case "reorder_rules":
        fixture.rules = (args.ids as string[]).map((id) =>
          fixture.rules.find((item) => item.id === id)!,
        );
        return null;
      case "get_settings":
        return fixture.settings;
      case "test_proxy_latency":
        return { latency_ms: 42 };
      case "update_settings":
        fixture.settings = args.settings as typeof fixture.settings;
        return fixture.settings;
      case "get_runtime_diagnostics":
        return { items: fixture.diagnostics, total: fixture.diagnostics.length, next_offset: null };
      case "clear_runtime_diagnostics":
        fixture.diagnostics = [];
        return 1;
      case "copy_active_connection_detail":
        return "目标: example.org:443\n出口链: selected-proxy";
      case "export_configuration":
        return '{"schema_version":1,"profiles":[],"rules":[]}';
      case "import_configuration":
        return null;
      default:
        throw new Error(`Unexpected command: ${command}`);
    }
  });
});

export { mocks };
