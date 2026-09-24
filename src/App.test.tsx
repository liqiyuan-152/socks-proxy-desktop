import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
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
  proxy_profile_id: string | null;
  enabled: boolean;
};

let profiles: Profile[];
let rules: Rule[];
let defaultProfileId: string | null;
let mode: "global" | "rules" | "direct";
let settings: { launch_at_login: boolean; diagnostic_retention: string; latency_test_url: string };
let diagnostics: { id: string; created_at_ms: number; severity: string; summary: string }[];
let rejectMode: boolean;

function runtime() {
  return {
    revision: 1,
    selected_mode: mode,
    desired_mode: mode,
    applied_mode: mode,
    phase: mode === "direct" ? "stopped" : "running",
    active_profile_id: defaultProfileId,
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
  defaultProfileId = "primary";
  rules = [
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
  mode = "global";
  settings = {
    launch_at_login: false,
    diagnostic_retention: "days30",
    latency_test_url: "https://www.gstatic.com/generate_204",
  };
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
        if (rejectMode) throw { code: "unavailable", message: "内核启动失败", fields: [] };
        mode = args.mode as typeof mode;
        return runtime();
      case "recover_network":
        mode = "direct";
        return { completed_at_ms: Date.now(), snapshot: runtime() };
      case "save_profile": {
        const input = args.input as Profile;
        if (
          !input.enabled &&
          (input.id === defaultProfileId ||
            rules.some((rule) => rule.proxy_profile_id === input.id))
        ) {
          throw { code: "validation_error", message: "代理仍被默认出口或规则引用", fields: [] };
        }
        const profile = { ...input, id: input.id ?? "new-profile" };
        profiles = profiles.filter((item) => item.id !== profile.id).concat(profile);
        return profile;
      }
      case "select_profile":
        defaultProfileId = args.id as string | null;
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
      case "test_proxy_latency":
        return { latency_ms: 42 };
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
  it("allows rules without a default but requires one for global mode", async () => {
    profiles = [];
    mode = "direct";
    render(<App />);
    await waitFor(() => expect(screen.getByRole("tab", { name: "规则代理" })).not.toBeDisabled());

    fireEvent.mouseDown(screen.getByRole("tab", { name: "规则代理" }), {
      button: 0,
      ctrlKey: false,
    });
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("set_runtime_mode", { mode: "rules" }),
    );
    await waitFor(() => expect(screen.queryByText("正在切换代理模式…")).not.toBeInTheDocument());
    fireEvent.mouseDown(screen.getByRole("tab", { name: "全局代理" }), {
      button: 0,
      ctrlKey: false,
    });
    expect(await screen.findByText(/尚未添加代理，请先添加代理/)).toBeInTheDocument();
    expect(mocks.invoke).not.toHaveBeenCalledWith("set_runtime_mode", { mode: "global" });

    fireEvent.click(screen.getByRole("button", { name: "去添加" }));
    expect(await screen.findByRole("button", { name: "添加代理" })).toBeInTheDocument();
  });

  it.each(["direct", "rules"] as const)(
    "selects the persisted %s mode while the runtime is stopped",
    async (selectedMode) => {
      const original = mocks.invoke.getMockImplementation()!;
      mocks.invoke.mockImplementation((command: string, args: Record<string, unknown>) =>
        command === "get_runtime_snapshot"
          ? Promise.resolve({
              ...runtime(),
              selected_mode: selectedMode,
              desired_mode: "direct",
              applied_mode: null,
              phase: "stopped",
            })
          : original(command, args),
      );
      render(<App />);
      await screen.findByText("example.org");
      const label = selectedMode === "direct" ? "全局直连" : "规则代理";
      await waitFor(() =>
        expect(screen.getByRole("tab", { name: label })).toHaveAttribute("data-state", "active"),
      );
      expect(screen.getByText("已应用模式").nextElementSibling).toHaveTextContent("未应用");
      expect(mocks.invoke).not.toHaveBeenCalledWith("set_runtime_mode", expect.anything());
    },
  );

  it("loads the real runtime, coverage and active snapshot without sample outcomes", async () => {
    render(<App />);
    expect(screen.getByText("正在加载运行时状态…")).toBeInTheDocument();
    await screen.findByText("example.org");
    expect(screen.getByText("仅遵循 Windows 系统代理设置的应用流量")).toBeInTheDocument();
    expect(screen.getByText("未启用")).toBeInTheDocument();
    expect(screen.queryByText("成功")).not.toBeInTheDocument();
    expect(screen.getAllByText("不可用").length).toBeGreaterThan(0);
  });

  it("selects immediately and keeps the target selected after a logged failure", async () => {
    render(<App />);
    await screen.findByText("example.org");
    await waitFor(() => expect(screen.getByRole("tab", { name: "规则代理" })).not.toBeDisabled());
    rejectMode = true;
    fireEvent.mouseDown(screen.getByRole("tab", { name: "规则代理" }), {
      button: 0,
      ctrlKey: false,
    });
    expect(screen.getByRole("tab", { name: "规则代理" })).toHaveAttribute("data-state", "active");
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("set_runtime_mode", { mode: "rules" }),
    );
    await waitFor(() => expect(screen.queryByText("正在切换代理模式…")).not.toBeInTheDocument());
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "规则代理" })).toHaveAttribute("data-state", "active");
    expect(screen.getByText("已应用模式").nextElementSibling).toHaveTextContent("全局代理");
    fireEvent.click(screen.getByRole("tab", { name: "规则代理" }));
    await waitFor(() =>
      expect(
        mocks.invoke.mock.calls.filter(([command]) => command === "set_runtime_mode"),
      ).toHaveLength(2),
    );
  });

  it("keeps controls responsive and applies only the latest queued mode", async () => {
    render(<App />);
    await screen.findByText("example.org");
    const original = mocks.invoke.getMockImplementation()!;
    let finish: ((snapshot: ReturnType<typeof runtime>) => void) | undefined;
    const pending = new Promise<ReturnType<typeof runtime>>((resolve) => {
      finish = resolve;
    });
    let firstSwitch = true;
    mocks.invoke.mockImplementation((command: string, args: Record<string, unknown>) => {
      if (command === "set_runtime_mode" && firstSwitch) {
        firstSwitch = false;
        return pending;
      }
      return original(command, args);
    });
    fireEvent.mouseDown(screen.getByRole("tab", { name: "规则代理" }), {
      button: 0,
      ctrlKey: false,
    });
    expect(await screen.findByText("正在切换代理模式…")).toBeInTheDocument();
    const statusContent = screen.getByRole("tablist").closest<HTMLElement>(".content-scroll")!;
    expect(within(statusContent).queryByText("正在切换代理模式…")).not.toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "规则代理" })).toHaveAttribute("data-state", "active");
    expect(screen.getByRole("tab", { name: "全局直连" })).not.toBeDisabled();
    fireEvent.mouseDown(screen.getByRole("tab", { name: "全局直连" }), {
      button: 0,
      ctrlKey: false,
    });
    fireEvent.mouseDown(screen.getByRole("tab", { name: "全局代理" }), {
      button: 0,
      ctrlKey: false,
    });
    expect(screen.getByRole("tab", { name: "全局代理" })).toHaveAttribute("data-state", "active");
    expect(screen.getAllByText("正在切换代理模式…")).toHaveLength(1);
    mode = "rules";
    finish?.(runtime());
    await waitFor(() =>
      expect(
        mocks.invoke.mock.calls.filter(([command]) => command === "set_runtime_mode"),
      ).toHaveLength(2),
    );
    expect(mocks.invoke).toHaveBeenCalledWith("set_runtime_mode", { mode: "global" });
    expect(mocks.invoke).not.toHaveBeenCalledWith("set_runtime_mode", { mode: "direct" });
    await waitFor(() => expect(screen.queryByText("正在切换代理模式…")).not.toBeInTheDocument());
    expect(screen.getByRole("tab", { name: "全局代理" })).toHaveAttribute("data-state", "active");
  });

  it("continues to the latest selection when an earlier switch fails", async () => {
    render(<App />);
    await screen.findByText("example.org");
    const original = mocks.invoke.getMockImplementation()!;
    let fail: ((reason: unknown) => void) | undefined;
    const pending = new Promise<ReturnType<typeof runtime>>((_, reject) => {
      fail = reject;
    });
    let firstSwitch = true;
    mocks.invoke.mockImplementation((command: string, args: Record<string, unknown>) => {
      if (command === "set_runtime_mode" && firstSwitch) {
        firstSwitch = false;
        return pending;
      }
      return original(command, args);
    });
    fireEvent.mouseDown(screen.getByRole("tab", { name: "规则代理" }), {
      button: 0,
      ctrlKey: false,
    });
    fireEvent.mouseDown(screen.getByRole("tab", { name: "全局直连" }), {
      button: 0,
      ctrlKey: false,
    });
    fail?.({ code: "unavailable", message: "内核启动失败", fields: [] });
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("set_runtime_mode", { mode: "direct" }),
    );
    await waitFor(() => expect(screen.queryByText("正在切换代理模式…")).not.toBeInTheDocument());
    expect(screen.getByRole("tab", { name: "全局直连" })).toHaveAttribute("data-state", "active");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("dismisses the mode switch toast when leaving the status page", async () => {
    render(<App />);
    await screen.findByText("example.org");
    const original = mocks.invoke.getMockImplementation()!;
    let finish: ((snapshot: ReturnType<typeof runtime>) => void) | undefined;
    mocks.invoke.mockImplementation((command: string, args: Record<string, unknown>) =>
      command === "set_runtime_mode"
        ? new Promise<ReturnType<typeof runtime>>((resolve) => {
            finish = resolve;
          })
        : original(command, args),
    );
    fireEvent.mouseDown(screen.getByRole("tab", { name: "规则代理" }), {
      button: 0,
      ctrlKey: false,
    });
    expect(await screen.findByText("正在切换代理模式…")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    await waitFor(() => expect(screen.queryByText("正在切换代理模式…")).not.toBeInTheDocument());
    finish?.(runtime());
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
    await waitFor(() => expect(screen.getByLabelText("用户名")).toHaveValue("alice"));
    expect(screen.getByLabelText("密码")).toHaveValue("stored-secret");
    expect(screen.getByLabelText("密码")).toHaveAttribute("type", "password");
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

  it("replaces edited credentials and clears them when the dialog closes", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    fireEvent.click(await screen.findByRole("button", { name: "编辑Primary" }));
    await waitFor(() => expect(screen.getByLabelText("密码")).toHaveValue("stored-secret"));
    fireEvent.change(screen.getByLabelText("用户名"), { target: { value: "bob" } });
    fireEvent.change(screen.getByLabelText("密码"), { target: { value: "changed-secret" } });
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith(
        "save_profile",
        expect.objectContaining({
          input: expect.objectContaining({
            credential: { action: "replace", username: "bob", password: "changed-secret" },
          }),
        }),
      ),
    );
    fireEvent.click(screen.getByRole("button", { name: "添加代理" }));
    fireEvent.click(screen.getByRole("switch", { name: "启用认证" }));
    expect(screen.getByLabelText("用户名")).toHaveValue("");
    expect(screen.getByLabelText("密码")).toHaveValue("");
  });

  it("does not fetch credentials for unauthenticated profiles", async () => {
    profiles[0].authentication_enabled = false;
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    fireEvent.click(await screen.findByRole("button", { name: "编辑Primary" }));
    expect(mocks.invoke).not.toHaveBeenCalledWith("get_profile_credential", expect.anything());
  });

  it("blocks saving when credential loading fails and retries successfully", async () => {
    const original = mocks.invoke.getMockImplementation()!;
    let attempt = 0;
    mocks.invoke.mockImplementation((name: string, args: Record<string, unknown>) => {
      if (name === "get_profile_credential" && attempt++ === 0) {
        return Promise.reject({ message: "凭据库不可用" });
      }
      return original(name, args);
    });
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    fireEvent.click(await screen.findByRole("button", { name: "编辑Primary" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("凭据库不可用");
    expect(screen.getByRole("button", { name: "保存" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "重试" }));
    await waitFor(() => expect(screen.getByLabelText("密码")).toHaveValue("stored-secret"));
    expect(screen.getByRole("button", { name: "保存" })).not.toBeDisabled();
  });

  it("ignores credentials returned after closing the edit dialog", async () => {
    const original = mocks.invoke.getMockImplementation()!;
    let finish: ((credential: { username: string; password: string }) => void) | undefined;
    mocks.invoke.mockImplementation((name: string, args: Record<string, unknown>) =>
      name === "get_profile_credential"
        ? new Promise((resolve) => {
            finish = resolve;
          })
        : original(name, args),
    );
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    fireEvent.click(await screen.findByRole("button", { name: "编辑Primary" }));
    expect(screen.getByRole("button", { name: "保存" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    fireEvent.click(screen.getByRole("button", { name: "添加代理" }));
    fireEvent.click(screen.getByRole("switch", { name: "启用认证" }));
    finish?.({ username: "alice", password: "stored-secret" });
    await waitFor(() => expect(screen.getByLabelText("密码")).toHaveValue(""));
  });

  it("distinguishes selected, enabled and disabled profile statuses on all screen sizes", async () => {
    profiles.push(
      { ...profiles[0], id: "secondary", name: "Secondary" },
      { ...profiles[0], id: "disabled", name: "Disabled", enabled: false },
    );
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    await screen.findByText("Secondary");

    for (const [name, label, color] of [
      ["Primary", "默认代理", "text-emerald-700"],
      ["Secondary", "已启用", "text-blue-700"],
      ["Disabled", "已停用", "text-destructive"],
    ]) {
      const row = within(screen.getByRole("table")).getByText(name).closest("tr")!;
      const labels = within(row).getAllByText(label);
      expect(labels).toHaveLength(2);
      for (const item of labels) expect(item).toHaveClass(color);
    }
  });

  it("puts the select action first for an inactive proxy", async () => {
    profiles.push({ ...profiles[0], id: "secondary", name: "Secondary" });
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    const select = await screen.findByRole("button", { name: "设Secondary为默认代理" });
    const actions = select.parentElement;
    expect(actions).toHaveClass("grid-cols-2");
    expect(
      Array.from(actions?.querySelectorAll("button") ?? [], (button) =>
        button.getAttribute("aria-label"),
      ),
    ).toEqual(["设Secondary为默认代理", "测试Secondary延迟", "编辑Secondary", "删除Secondary"]);
  });

  it("sets and clears the default proxy without disabling other exits", async () => {
    profiles.push({ ...profiles[0], id: "secondary", name: "Secondary" });
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    fireEvent.click(await screen.findByRole("button", { name: "设Secondary为默认代理" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "设Primary为默认代理" })).toBeInTheDocument(),
    );
    expect(mocks.invoke).toHaveBeenCalledWith("select_profile", { id: "secondary" });
    expect(profiles.every((profile) => profile.enabled)).toBe(true);

    fireEvent.click(screen.getByRole("button", { name: "取消默认代理" }));
    await waitFor(() =>
      expect(screen.queryByRole("button", { name: "取消默认代理" })).not.toBeInTheDocument(),
    );
    expect(mocks.invoke).toHaveBeenCalledWith("select_profile", { id: null });
  });

  it("keeps a referenced proxy enabled when disabling is rejected", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    const switches = await screen.findAllByRole("switch", { name: "Primary启用状态" });
    fireEvent.click(switches[0]);
    expect(await screen.findByRole("alert")).toHaveTextContent("代理仍被默认出口或规则引用");
    expect(profiles[0].enabled).toBe(true);
    expect(switches[0]).toBeChecked();
  });

  it("tests one profile on demand and clears its result after editing", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    const test = await screen.findByRole("button", { name: "测试Primary延迟" });
    expect(test.closest("td")?.cellIndex).toBe(7);
    expect(screen.queryByText("42 ms")).not.toBeInTheDocument();
    fireEvent.click(test);
    await screen.findByText("42 ms");
    expect(await screen.findByText("Primary：42 ms")).toBeInTheDocument();
    expect(mocks.invoke).toHaveBeenCalledWith("test_proxy_latency", { id: "primary" });
    fireEvent.click(screen.getByRole("button", { name: "编辑Primary" }));
    await waitFor(() => expect(screen.getByLabelText("密码")).toHaveValue("stored-secret"));
    fireEvent.change(screen.getByLabelText("服务器*"), { target: { value: "other.example.org" } });
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(screen.queryByText("42 ms")).not.toBeInTheDocument());
  });

  it("shows test failures without fabricating a latency", async () => {
    const original = mocks.invoke.getMockImplementation()!;
    mocks.invoke.mockImplementation((command: string, args: Record<string, unknown>) =>
      command === "test_proxy_latency"
        ? Promise.reject({ code: "unavailable", message: "代理测试超时（5 秒）", fields: [] })
        : original(command, args),
    );
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    fireEvent.click(await screen.findByRole("button", { name: "测试Primary延迟" }));
    await screen.findByText("失败");
    expect(screen.getByText("失败")).toHaveAttribute(
      "title",
      expect.stringContaining("代理测试超时（5 秒）"),
    );
    expect(
      await screen.findByText("Primary 延迟测试失败：代理测试超时（5 秒）"),
    ).toBeInTheDocument();
  });

  it("shows test progress in a toast and disables retesting until completion", async () => {
    const original = mocks.invoke.getMockImplementation()!;
    let finish: ((result: { latency_ms: number }) => void) | undefined;
    mocks.invoke.mockImplementation((command: string, args: Record<string, unknown>) =>
      command === "test_proxy_latency"
        ? new Promise<{ latency_ms: number }>((resolve) => {
            finish = resolve;
          })
        : original(command, args),
    );
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    const test = await screen.findByRole("button", { name: "测试Primary延迟" });
    fireEvent.click(test);
    expect(await screen.findByText("正在测试 Primary 的延迟…")).toBeInTheDocument();
    expect(test).toBeDisabled();

    finish?.({ latency_ms: 250 });
    expect(await screen.findByText("Primary：250 ms")).toBeInTheDocument();
    await waitFor(() => expect(test).not.toBeDisabled());
    expect(screen.getByText("250 ms")).toHaveClass("text-amber-700");
  });

  it("limits batch testing to three concurrent profiles", async () => {
    profiles = Array.from({ length: 5 }, (_, index) => ({
      ...profiles[0],
      id: `proxy-${index}`,
      name: `Proxy${index}`,
    }));
    const original = mocks.invoke.getMockImplementation()!;
    let active = 0;
    let maximum = 0;
    const pending: Array<() => void> = [];
    mocks.invoke.mockImplementation((command: string, args: Record<string, unknown>) => {
      if (command !== "test_proxy_latency") return original(command, args);
      active++;
      maximum = Math.max(maximum, active);
      return new Promise((resolve) =>
        pending.push(() => {
          active--;
          resolve({ latency_ms: 30 });
        }),
      );
    });
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    const batch = await screen.findByRole("button", { name: "测试全部" });
    await waitFor(() => expect(batch).not.toBeDisabled());
    fireEvent.click(batch);
    await waitFor(() => expect(pending).toHaveLength(3));
    expect(maximum).toBe(3);
    pending[0]();
    await waitFor(() => expect(pending).toHaveLength(4));
    pending[1]();
    await waitFor(() => expect(pending).toHaveLength(5));
    pending.slice(2).forEach((finish) => finish());
    await waitFor(() => expect(screen.getAllByText("30 ms")).toHaveLength(5));
    expect(maximum).toBe(3);
  });

  it("fills a new profile from a link and saves only after confirmation", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "添加代理" })).not.toBeDisabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "添加代理" }));
    fireEvent.change(await screen.findByLabelText("代理链接"), {
      target: { value: "【socks5://repeatlink:abc0123456789\\@210.48.231.68:1080】" },
    });
    fireEvent.click(screen.getByRole("button", { name: "解析" }));
    expect(screen.getByLabelText("代理链接")).toHaveValue("");
    expect(screen.getByLabelText("名称*")).toHaveValue("210.48.231.68:1080");
    expect(screen.getByLabelText("服务器*")).toHaveValue("210.48.231.68");
    expect(screen.getByLabelText("端口*")).toHaveValue("1080");
    expect(screen.getByRole("switch", { name: "启用认证" })).toBeChecked();
    expect(screen.getByLabelText("用户名")).toHaveValue("repeatlink");
    expect(screen.getByLabelText("密码")).toHaveValue("abc0123456789");
    expect(mocks.invoke.mock.calls.filter(([command]) => command === "save_profile")).toHaveLength(
      0,
    );

    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("save_profile", {
        input: expect.objectContaining({
          id: null,
          name: "210.48.231.68:1080",
          protocol: "socks5",
          host: "210.48.231.68",
          port: 1080,
          authentication_enabled: true,
          credential: { action: "replace", username: "repeatlink", password: "abc0123456789" },
        }),
      }),
    );
  });

  it("keeps manual fields on parse failure and hides the link field when editing", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "添加代理" })).not.toBeDisabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "添加代理" }));
    await screen.findByLabelText("代理链接");
    fireEvent.change(screen.getByLabelText("名称*"), { target: { value: "Manual" } });
    fireEvent.change(screen.getByLabelText("代理链接"), {
      target: { value: "socks5://host:65536" },
    });
    fireEvent.click(screen.getByRole("button", { name: "解析" }));
    expect(screen.getByRole("alert")).toHaveTextContent("端口");
    expect(screen.getByLabelText("名称*")).toHaveValue("Manual");
    expect(mocks.invoke.mock.calls.filter(([command]) => command === "save_profile")).toHaveLength(
      0,
    );
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    fireEvent.click(screen.getByRole("button", { name: "编辑Primary" }));
    expect(screen.queryByLabelText("代理链接")).not.toBeInTheDocument();
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

  it("saves a rule with its selected exit and port range", async () => {
    profiles.push({ ...profiles[0], id: "secondary", name: "Secondary" });
    rules[0].proxy_profile_id = "secondary";
    rules[0].port_end = 445;
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "分流规则" }));
    await screen.findByText("443–445");
    expect(screen.getByText("Secondary")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "编辑Example" }));
    expect(screen.getByRole("textbox", { name: "结束端口" })).toHaveValue("445");
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("replace_rules", {
        rules: [
          expect.objectContaining({
            id: "rule-1",
            proxy_profile_id: "secondary",
            port_start: 443,
            port_end: 445,
          }),
        ],
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

  it("persists settings and hides update checks while network recovery is unavailable", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "设置" }));
    const startup = await screen.findByRole("switch", { name: "开机启动" });
    await waitFor(() => expect(startup).not.toBeDisabled());
    fireEvent.click(startup);
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("update_settings", {
        settings: {
          launch_at_login: true,
          diagnostic_retention: "days30",
          latency_test_url: "https://www.gstatic.com/generate_204",
        },
      }),
    );
    expect(screen.queryByRole("button", { name: "检查更新" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "恢复网络设置" })).toBeDisabled();
    expect(screen.queryByText("尚未配置发布渠道，无法检查更新。")).not.toBeInTheDocument();
  });

  it("saves the configured latency test URL", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("link", { name: "设置" }));
    const input = await screen.findByRole("textbox", { name: "延迟测试地址" });
    fireEvent.change(input, { target: { value: "https://example.org/check" } });
    fireEvent.click(screen.getByRole("button", { name: "保存测试地址" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("update_settings", {
        settings: { ...settings, latency_test_url: "https://example.org/check" },
      }),
    );
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
    const fileInput = screen.getByLabelText("选择配置文件");
    expect(fileInput).toHaveClass("sr-only");
    expect(fileInput).not.toHaveClass("w-full");
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
