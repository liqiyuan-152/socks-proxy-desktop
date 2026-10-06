import { defaultRulesMode } from "@/lib/proxy-mode";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { fixture, mocks, runtime } from "./test/app-fixture";
import App from "./App";

describe("backend-driven desktop UI", () => {
  it("requires a default for rules with proxy fallback and for global mode", async () => {
    fixture.profiles = [];
    fixture.mode = "direct";
    render(<App />);
    await waitFor(() => expect(screen.getByRole("radio", { name: "规则代理" })).not.toBeDisabled());

    fireEvent.click(screen.getByRole("radio", { name: "规则代理" }), {
      button: 0,
      ctrlKey: false,
    });
    expect(await screen.findByText(/尚未添加代理，请先添加代理/)).toBeInTheDocument();
    expect(mocks.invoke).not.toHaveBeenCalledWith("set_runtime_mode", { mode: defaultRulesMode });
    fireEvent.click(screen.getByRole("radio", { name: "全局代理" }), {
      button: 0,
      ctrlKey: false,
    });
    expect(await screen.findByText(/尚未添加代理，请先添加代理/)).toBeInTheDocument();
    expect(mocks.invoke).not.toHaveBeenCalledWith("set_runtime_mode", { mode: "global" });

    fireEvent.click(screen.getByRole("button", { name: "去添加" }));
    expect(await screen.findByRole("button", { name: "添加代理" })).toBeInTheDocument();
  });

  it.each(["direct", defaultRulesMode] as const)(
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
        expect(screen.getByRole("radio", { name: label })).toHaveAttribute("data-state", "checked"),
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
    expect(screen.getByText("最近操作").nextElementSibling).toHaveTextContent("成功");
    expect(screen.getAllByText("不可用").length).toBeGreaterThan(0);
  });

  it("selects immediately and rolls back to the applied mode after failure", async () => {
    render(<App />);
    await screen.findByText("example.org");
    await waitFor(() => expect(screen.getByRole("radio", { name: "规则代理" })).not.toBeDisabled());
    fixture.rejectMode = true;
    fireEvent.click(screen.getByRole("radio", { name: "规则代理" }), {
      button: 0,
      ctrlKey: false,
    });
    expect(screen.getByRole("radio", { name: "规则代理" })).toHaveAttribute(
      "data-state",
      "checked",
    );
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("set_runtime_mode", { mode: defaultRulesMode }),
    );
    await waitFor(() => expect(screen.queryByText("正在切换代理模式…")).not.toBeInTheDocument());
    expect(screen.getByRole("alert")).toHaveTextContent("内核启动失败");
    expect(screen.getByRole("radio", { name: "全局代理" })).toHaveAttribute(
      "data-state",
      "checked",
    );
    expect(screen.getByText("已应用模式").nextElementSibling).toHaveTextContent("全局代理");
    fireEvent.click(screen.getByRole("radio", { name: "规则代理" }));
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
    fireEvent.click(screen.getByRole("radio", { name: "规则代理" }), {
      button: 0,
      ctrlKey: false,
    });
    expect(await screen.findByText("正在切换代理模式…")).toBeInTheDocument();
    const statusContent = screen.getByRole("radiogroup").closest<HTMLElement>(".content-scroll")!;
    expect(within(statusContent).queryByText("正在切换代理模式…")).not.toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "规则代理" })).toHaveAttribute(
      "data-state",
      "checked",
    );
    expect(screen.getByRole("radio", { name: "全局直连" })).not.toBeDisabled();
    fireEvent.click(screen.getByRole("radio", { name: "全局直连" }), {
      button: 0,
      ctrlKey: false,
    });
    fireEvent.click(screen.getByRole("radio", { name: "全局代理" }), {
      button: 0,
      ctrlKey: false,
    });
    expect(screen.getByRole("radio", { name: "全局代理" })).toHaveAttribute(
      "data-state",
      "checked",
    );
    expect(screen.getAllByText("正在切换代理模式…")).toHaveLength(1);
    fixture.mode = defaultRulesMode;
    finish?.(runtime());
    await waitFor(() =>
      expect(
        mocks.invoke.mock.calls.filter(([command]) => command === "set_runtime_mode"),
      ).toHaveLength(2),
    );
    expect(mocks.invoke).toHaveBeenCalledWith("set_runtime_mode", { mode: "global" });
    expect(mocks.invoke).not.toHaveBeenCalledWith("set_runtime_mode", { mode: "direct" });
    await waitFor(() => expect(screen.queryByText("正在切换代理模式…")).not.toBeInTheDocument());
    expect(screen.getByRole("radio", { name: "全局代理" })).toHaveAttribute(
      "data-state",
      "checked",
    );
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
    fireEvent.click(screen.getByRole("radio", { name: "规则代理" }), {
      button: 0,
      ctrlKey: false,
    });
    fireEvent.click(screen.getByRole("radio", { name: "全局直连" }), {
      button: 0,
      ctrlKey: false,
    });
    fail?.({ code: "unavailable", message: "内核启动失败", fields: [] });
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("set_runtime_mode", { mode: "direct" }),
    );
    await waitFor(() => expect(screen.queryByText("正在切换代理模式…")).not.toBeInTheDocument());
    expect(screen.getByRole("radio", { name: "全局直连" })).toHaveAttribute(
      "data-state",
      "checked",
    );
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
    fireEvent.click(screen.getByRole("radio", { name: "规则代理" }), {
      button: 0,
      ctrlKey: false,
    });
    expect(await screen.findByText("正在切换代理模式…")).toBeInTheDocument();
    fireEvent.click(await screen.findByRole("link", { name: "代理" }));
    await waitFor(() => expect(screen.queryByText("正在切换代理模式…")).not.toBeInTheDocument());
    finish?.(runtime());
  });
});
