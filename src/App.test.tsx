import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, vi } from "vitest";
import App from "./App";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn(async () => () => {}) }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));

type Profile = {
  id: string;
  name: string;
  protocol: string;
  host: string;
  port: number;
  authentication_enabled: boolean;
  enabled: boolean;
};
type Rule = {
  id: string;
  name: string;
  matcher: string;
  target: string;
  port_start: number | null;
  port_end: number | null;
  action: string;
  enabled: boolean;
};

let profiles: Profile[];
let rules: Rule[];
let mode: "global" | "rules" | "direct";
let settings: { launch_at_login: boolean; diagnostic_retention: string };
let diagnostics: { id: string; created_at_ms: number; severity: string; summary: string }[];
let rejectMode: boolean;

function runtime() {
  return {
    revision: 1,
    desired_mode: mode,
    applied_mode: mode,
    phase: mode === "direct" ? "stopped" : "running",
    active_profile_id: profiles[0]?.id ?? null,
    runtime_uptime_ms: mode === "direct" ? null : 1200,
    system_proxy_enabled: mode !== "direct",
    tun_enabled: false,
    coverage: mode === "direct" ? "none" : "system_proxy_apps",
    last_error: null,
  };
}

beforeEach(() => {
  window.history.pushState({}, "", "/");
  profiles = [
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
  rules = [
    {
      id: "rule-1",
      name: "Example",
      matcher: "domain",
      target: "example.org",
      port_start: 443,
      port_end: null,
      action: "proxy",
      enabled: true,
    },
  ];
  mode = "global";
  settings = { launch_at_login: false, diagnostic_retention: "days30" };
  diagnostics = [
    { id: "diagnostic-1", created_at_ms: Date.now(), severity: "info", summary: "代理模式已提交" },
  ];
  rejectMode = false;
  mocks.invoke.mockReset();
  mocks.listen.mockClear();
  mocks.invoke.mockImplementation(async (command: string, args: Record<string, unknown> = {}) => {
    switch (command) {
      case "get_runtime_snapshot":
        return runtime();
      case "list_profiles":
        return profiles;
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
        if (rejectMode) throw { code: "unavailable", message: "内核启动失败", fields: [] };
        mode = args.mode as typeof mode;
        return runtime();
      case "recover_network":
        mode = "direct";
        return { completed_at_ms: Date.now(), snapshot: runtime() };
      case "save_profile": {
        const input = args.input as Profile;
        const profile = { ...input, id: input.id ?? "new-profile" };
        profiles = profiles.filter((item) => item.id !== profile.id).concat(profile);
        return profile;
      }
      case "select_profile":
        return null;
      case "delete_profile":
        profiles = profiles.filter((item) => item.id !== args.id);
        return null;
      case "list_rules":
        return rules;
      case "replace_rules":
        rules = args.rules as Rule[];
        return null;
      case "reorder_rules":
        rules = (args.ids as string[]).map((id) => rules.find((item) => item.id === id)!);
        return null;
      case "get_settings":
        return settings;
      case "update_settings":
        settings = args.settings as typeof settings;
        return settings;
      case "get_runtime_diagnostics":
        return { items: diagnostics, total: diagnostics.length, next_offset: null };
      case "clear_runtime_diagnostics":
        diagnostics = [];
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

describe("backend-driven desktop UI", () => {
  it("loads the real runtime, coverage and active snapshot without sample outcomes", async () => {
    render(<App />);
    expect(screen.getByText("正在加载运行时状态…")).toBeInTheDocument();
    await screen.findByText("example.org");
    expect(screen.getByText("仅遵循 Windows 系统代理设置的应用流量")).toBeInTheDocument();
    expect(screen.getByText("未启用")).toBeInTheDocument();
    expect(screen.queryByText("成功")).not.toBeInTheDocument();
    expect(screen.getAllByText("不可用").length).toBeGreaterThan(0);
  });

  it("keeps the previously applied mode and shows an error when backend switching fails", async () => {
    render(<App />);
    await screen.findByText("example.org");
    await waitFor(() => expect(screen.getByRole("tab", { name: "规则代理" })).not.toBeDisabled());
    rejectMode = true;
    fireEvent.mouseDown(screen.getByRole("tab", { name: "规则代理" }), {
      button: 0,
      ctrlKey: false,
    });
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("set_runtime_mode", { mode: "rules" }),
    );
    await screen.findByRole("alert");
    expect(screen.getByRole("alert")).toHaveTextContent("内核启动失败");
    expect(screen.getByRole("tab", { name: "全局代理" })).toHaveAttribute("data-state", "active");
  });

  it("disables conflicting mode switches until the backend commits", async () => {
    render(<App />);
    await screen.findByText("example.org");
    const original = mocks.invoke.getMockImplementation()!;
    let finish: ((snapshot: ReturnType<typeof runtime>) => void) | undefined;
    const pending = new Promise<ReturnType<typeof runtime>>((resolve) => {
      finish = resolve;
    });
    mocks.invoke.mockImplementation((command: string, args: Record<string, unknown>) =>
      command === "set_runtime_mode" ? pending : original(command, args),
    );
    fireEvent.mouseDown(screen.getByRole("tab", { name: "规则代理" }), {
      button: 0,
      ctrlKey: false,
    });
    await waitFor(() => expect(screen.getByRole("tab", { name: "全局直连" })).toBeDisabled());
    expect(screen.getByRole("tab", { name: "全局代理" })).toHaveAttribute("data-state", "active");
    expect(screen.getByText(/正在提交模式/)).toBeInTheDocument();
    mode = "rules";
    finish?.(runtime());
    await waitFor(() =>
      expect(screen.getByRole("tab", { name: "规则代理" })).toHaveAttribute("data-state", "active"),
    );
  });

  it("searches real profiles and preserves credentials when editing non-secret fields", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    await screen.findByRole("button", { name: "编辑Primary" });
    fireEvent.change(screen.getByPlaceholderText("搜索代理名称、服务器地址..."), {
      target: { value: "absent" },
    });
    expect(screen.getByText("暂无匹配的代理档案")).toBeInTheDocument();
    fireEvent.change(screen.getByPlaceholderText("搜索代理名称、服务器地址..."), {
      target: { value: "Primary" },
    });
    fireEvent.click(screen.getByRole("button", { name: "编辑Primary" }));
    expect(screen.getByLabelText("密码")).toHaveValue("");
    fireEvent.change(screen.getByLabelText("服务器*"), { target: { value: "new.example.org" } });
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith(
        "save_profile",
        expect.objectContaining({
          input: expect.objectContaining({
            host: "new.example.org",
            credential: { action: "preserve" },
          }),
        }),
      ),
    );
  });

  it("loads real rules and persists enable toggles and ordering", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "分流规则" }));
    await screen.findByText("共 1 条规则");
    fireEvent.click(screen.getByRole("switch", { name: "Example已启用" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("replace_rules", {
        rules: [expect.objectContaining({ id: "rule-1", enabled: false })],
      }),
    );
  });

  it("shows unavailable history, active details, and confirms diagnostic cleanup", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "连接日志" }));
    await screen.findByText("代理模式已提交");
    expect(screen.getByText(/已完成连接历史、成功率与失败详情暂不可用/)).toBeInTheDocument();
    expect(mocks.invoke).toHaveBeenCalledWith(
      "get_runtime_diagnostics",
      expect.objectContaining({
        filter: expect.objectContaining({
          severity: null,
          until_ms: null,
          from_ms: expect.any(Number),
        }),
        offset: 0,
        limit: 100,
      }),
    );
    fireEvent.change(screen.getByPlaceholderText("搜索目标、规则或诊断..."), {
      target: { value: "absent" },
    });
    expect(screen.getByText("暂无运行时诊断")).toBeInTheDocument();
    expect(screen.getByText("暂无活跃连接")).toBeInTheDocument();
    fireEvent.change(screen.getByPlaceholderText("搜索目标、规则或诊断..."), {
      target: { value: "" },
    });
    fireEvent.click(screen.getByRole("button", { name: "清理诊断" }));
    expect(screen.getByRole("dialog")).toHaveTextContent("活跃连接不受影响");
    fireEvent.click(screen.getByRole("button", { name: "确认清理" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith(
        "clear_runtime_diagnostics",
        expect.objectContaining({ confirmed: true }),
      ),
    );
    expect(screen.getByText("example.org:443")).toBeInTheDocument();
  });

  it("opens more details and copies only the backend-provided active connection summary", async () => {
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    render(<App />);
    await screen.findByText("example.org");
    fireEvent.click(screen.getByRole("button", { name: /查看更多/ }));
    await screen.findByText("代理模式已提交");
    expect(screen.getByText(/已完成连接历史、成功率与失败详情暂不可用/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "查看example.org详情" }));
    expect(screen.getByRole("dialog")).toHaveTextContent("最终结果不可用");
    fireEvent.click(screen.getByRole("button", { name: "复制脱敏详情" }));
    await waitFor(() =>
      expect(writeText).toHaveBeenCalledWith("目标: example.org:443\n出口链: selected-proxy"),
    );
    expect(mocks.invoke).toHaveBeenCalledWith("copy_active_connection_detail", { id: "active-1" });
  });

  it("renders empty backend snapshots without invented connections or results", async () => {
    const original = mocks.invoke.getMockImplementation()!;
    mocks.invoke.mockImplementation((command: string, args: Record<string, unknown>) => {
      if (command === "get_active_connections")
        return Promise.resolve({
          status: "available",
          active_count: 0,
          history_available: false,
          diagnostic: null,
          recent: [],
        });
      return original(command, args);
    });
    render(<App />);
    await screen.findByText("暂无活跃连接");
    expect(screen.getByRole("heading", { name: "当前活跃连接" })).toBeInTheDocument();
    expect(screen.queryByText("example.org")).not.toBeInTheDocument();
  });

  it("persists settings and keeps unconfigured update and network recovery unavailable", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "设置" }));
    const startup = await screen.findByRole("switch", { name: "开机启动" });
    await waitFor(() => expect(startup).not.toBeDisabled());
    fireEvent.click(startup);
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("update_settings", {
        settings: { launch_at_login: true, diagnostic_retention: "days30" },
      }),
    );
    expect(screen.getByRole("button", { name: "检查更新" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "恢复网络设置" })).toBeDisabled();
    expect(screen.getByText("尚未配置发布渠道，无法检查更新。")).toBeInTheDocument();
  });

  it("requires confirmation before clearing settings diagnostics", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "设置" }));
    fireEvent.click(await screen.findByRole("button", { name: "清理运行时诊断" }));
    expect(screen.getByRole("dialog")).toHaveTextContent("不会清理或伪造活跃连接");
    expect(mocks.invoke).not.toHaveBeenCalledWith("clear_runtime_diagnostics", expect.anything());
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(mocks.invoke).not.toHaveBeenCalledWith("clear_runtime_diagnostics", expect.anything());
    fireEvent.click(screen.getByRole("button", { name: "清理运行时诊断" }));
    fireEvent.click(screen.getByRole("button", { name: "确认清空" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("clear_runtime_diagnostics", {
        filter: { from_ms: null, until_ms: null, severity: null },
        confirmed: true,
      }),
    );
  });

  it("requires fresh credentials when confirming an imported authenticated profile", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "设置" }));
    await screen.findByRole("switch", { name: "开机启动" });
    const json = JSON.stringify({
      schema_version: 1,
      profiles: [
        {
          id: "imported",
          name: "Imported",
          authentication_enabled: true,
        },
      ],
      rules: [],
    });
    const file = new File([json], "settings.json", { type: "application/json" });
    Object.defineProperty(file, "text", { value: async () => json });
    fireEvent.change(screen.getByLabelText("选择配置文件"), { target: { files: [file] } });
    await screen.findByRole("dialog");
    fireEvent.click(screen.getByRole("button", { name: "确认导入" }));
    expect(screen.getByRole("alert")).toHaveTextContent("重新输入认证密码");
    expect(mocks.invoke).not.toHaveBeenCalledWith("import_configuration", expect.anything());
    fireEvent.change(screen.getByRole("textbox", { name: "Imported用户名" }), {
      target: { value: "alice" },
    });
    fireEvent.change(screen.getByLabelText("Imported密码"), { target: { value: "fresh-secret" } });
    fireEvent.click(screen.getByRole("button", { name: "确认导入" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("import_configuration", {
        json,
        updates: { imported: { action: "replace", username: "alice", password: "fresh-secret" } },
      }),
    );
    expect(screen.queryByText("fresh-secret")).not.toBeInTheDocument();
  });

  it("confirms Windows network recovery before invoking the backend", async () => {
    const originalAgent = navigator.userAgent;
    Object.defineProperty(navigator, "userAgent", { configurable: true, value: "Windows NT 10.0" });
    try {
      render(<App />);
      fireEvent.click(screen.getByRole("link", { name: "设置" }));
      const restore = await screen.findByRole("button", { name: "恢复网络设置" });
      expect(restore).not.toBeDisabled();
      fireEvent.click(restore);
      expect(screen.getByRole("dialog")).toHaveTextContent("外部修改不会被覆盖");
      expect(mocks.invoke).not.toHaveBeenCalledWith("recover_network", expect.anything());
      fireEvent.click(screen.getByRole("button", { name: "确认恢复" }));
      await waitFor(() =>
        expect(mocks.invoke).toHaveBeenCalledWith("recover_network", { confirmed: true }),
      );
      await screen.findByText(/网络恢复检查完成/);
    } finally {
      Object.defineProperty(navigator, "userAgent", { configurable: true, value: originalAgent });
    }
  });
});
