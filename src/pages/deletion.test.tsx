import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import type { ProxyProfile } from "@/lib/backend";
import type { RoutingRule } from "@/lib/generated/ipc";
import { TooltipProvider } from "@/components/ui/tooltip";
import ProxyList from "./proxies";
import RoutingRuleList from "./rules";

const { invoke, refresh, refreshRules } = vi.hoisted(() => ({
  invoke: vi.fn(),
  refresh: vi.fn(),
  refreshRules: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));
vi.mock("@/store/backend-store", () => ({
  useBackendStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      profiles: [profile],
      snapshot: null,
      capabilities: null,
      loading: false,
      refresh,
      rules,
      refreshRules,
      replaceRules: async (next: RoutingRule[]) => {
        await invoke("replace_rules", { rules: next });
      },
      reorderRules: async (ids: string[]) => {
        await invoke("reorder_rules", { ids });
      },
      deleteProfile: async (id: string) => {
        await invoke("delete_profile", { id });
        await refresh();
      },
      selectProfile: async (id: string | null) => {
        await invoke("select_profile", { id });
        await refresh();
      },
      saveProfile: async (input: unknown) => invoke("save_profile", { input }),
    }),
}));

const profile: ProxyProfile = {
  id: "proxy-1",
  name: "测试代理",
  protocol: "socks5",
  host: "proxy.example.com",
  port: 1080,
  authentication_enabled: false,
  enabled: true,
  configuration_revision: 1,
};
const rule: RoutingRule = {
  id: "rule-1",
  name: "测试规则",
  matcher: "domain",
  target: "example.com",
  port_start: null,
  port_end: null,
  action: "proxy",
  proxy_profile_id: profile.id,
  enabled: true,
};
let rules: RoutingRule[];

beforeEach(() => {
  invoke.mockReset();
  refresh.mockReset();
  refresh.mockResolvedValue(undefined);
  rules = [rule];
  refreshRules.mockReset().mockImplementation(async () => {
    rules = await invoke("list_rules", {});
  });
  invoke.mockImplementation(async (name: string, args: { rules?: RoutingRule[] }) => {
    if (name === "list_rules") return rules;
    if (name === "get_china_direct_status")
      return { enabled: false, available: false, data_date: null };
    if (name === "replace_rules") rules = args.rules ?? [];
    return undefined;
  });
});

const cases = [
  {
    Page: ProxyList,
    resource: "代理",
    name: profile.name,
    command: "delete_profile",
    args: { id: profile.id },
  },
  {
    Page: RoutingRuleList,
    resource: "规则",
    name: rule.name,
    command: "replace_rules",
    args: { rules: [] },
  },
];

it.each(cases)(
  "$resource cancellation never calls the mutation",
  async ({ Page, resource, name, command }) => {
    render(
      <TooltipProvider>
        <Page />
      </TooltipProvider>,
    );
    fireEvent.click(await screen.findByRole("button", { name: `删除${name}` }));
    const dialog = screen.getByRole("alertdialog", { name: `确认删除${resource}` });
    expect(invoke.mock.calls.some(([called]) => called === command)).toBe(false);
    fireEvent.click(within(dialog).getByRole("button", { name: "取消" }));
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    expect(screen.getByText(name)).toBeInTheDocument();
    expect(invoke.mock.calls.some(([called]) => called === command)).toBe(false);
  },
);

it.each(cases)(
  "$resource confirmation calls the correct IPC command and closes",
  async ({ Page, resource, name, command, args }) => {
    render(
      <TooltipProvider>
        <Page />
      </TooltipProvider>,
    );
    fireEvent.click(await screen.findByRole("button", { name: `删除${name}` }));
    const dialog = screen.getByRole("alertdialog", { name: `确认删除${resource}` });
    fireEvent.click(within(dialog).getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
    expect(invoke).toHaveBeenCalledWith(command, args);
    expect(invoke.mock.calls.filter(([called]) => called === command)).toHaveLength(1);
    if (command === "delete_profile") expect(refresh).toHaveBeenCalledOnce();
    else expect(screen.queryByText(name)).not.toBeInTheDocument();
  },
);

it("a referenced proxy stays in the dialog and can be retried after rejection", async () => {
  let rejected = false;
  invoke.mockImplementation(async (name: string) => {
    if (name === "delete_profile" && !rejected) {
      rejected = true;
      throw { code: "validation_error", message: "规则仍引用此代理", fields: [] };
    }
    return undefined;
  });
  render(<ProxyList />);
  fireEvent.click(screen.getByRole("button", { name: `删除${profile.name}` }));
  let dialog = screen.getByRole("alertdialog");
  fireEvent.click(within(dialog).getByRole("button", { name: "确认删除" }));
  await waitFor(() =>
    expect(within(dialog).getByRole("alert")).toHaveTextContent("规则仍引用此代理"),
  );
  expect(refresh).not.toHaveBeenCalled();
  await waitFor(() =>
    expect(within(dialog).getByRole("button", { name: "确认删除" })).not.toBeDisabled(),
  );
  dialog = screen.getByRole("alertdialog");
  fireEvent.click(within(dialog).getByRole("button", { name: "确认删除" }));
  await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
  expect(refresh).toHaveBeenCalledOnce();
});

it.each(cases)(
  "$resource preserves structured error details inside confirmation",
  async ({ Page, name, command }) => {
    const original = invoke.getMockImplementation()!;
    invoke.mockImplementation(async (called: string, args: { rules?: RoutingRule[] }) => {
      if (called === command)
        throw {
          code: "proxy_in_use",
          message: "无法完成删除",
          fields: [{ field: "profile", message: "该代理仍被规则引用" }],
          context: {
            error_id: "error-delete-42",
            timestamp_ms: 1,
            domain: "proxy",
            kind: "in_use",
            recovery_suggestion: "请先修改引用规则",
          },
        };
      return original(called, args);
    });
    render(
      <TooltipProvider>
        <Page />
      </TooltipProvider>,
    );
    fireEvent.click(await screen.findByRole("button", { name: `删除${name}` }));
    const dialog = screen.getByRole("alertdialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "确认删除" }));
    const alert = await within(dialog).findByRole("alert");
    expect(alert).toHaveTextContent("profile: 该代理仍被规则引用");
    expect(alert).toHaveTextContent("请先修改引用规则");
    expect(alert).toHaveTextContent("error-delete-42");
    expect(screen.getAllByRole("alert")).toHaveLength(1);
    expect(invoke.mock.calls.filter(([called]) => called === command)).toHaveLength(1);
  },
);
