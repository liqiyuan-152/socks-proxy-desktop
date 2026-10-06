import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { fixture, mocks, runtime } from "@/test/app-fixture";
import App from "@/App";

beforeEach(() => {
  fixture.mode = { rules: { use_china_direct: false, default_action: "proxy" } };
});

it("debounces parameter changes and sends the complete Rules mode", async () => {
  render(<App />);
  const toggle = await screen.findByRole("switch", { name: "国内直连" });
  fireEvent.click(toggle);
  fireEvent.click(toggle);
  fireEvent.click(toggle);
  expect(mocks.invoke).not.toHaveBeenCalledWith("set_runtime_mode", expect.anything());
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("set_runtime_mode", {
      mode: { rules: { use_china_direct: true, default_action: "proxy" } },
    }),
  );
  expect(mocks.invoke.mock.calls.filter(([name]) => name === "set_runtime_mode")).toHaveLength(1);
  await waitFor(() => expect(screen.getByRole("switch", { name: "国内直连" })).toBeChecked());
  expect(screen.getByText(/默认走代理/)).toBeInTheDocument();
});

it("saves the selected default action and rolls back rejected parameters", async () => {
  render(<App />);
  const select = await screen.findByRole("combobox", { name: "未匹配流量" });
  fireEvent.click(select);
  fireEvent.click(await screen.findByRole("option", { name: "直连" }));
  await waitFor(() =>
    expect(fixture.mode).toEqual({ rules: { use_china_direct: false, default_action: "direct" } }),
  );
  await waitFor(() => expect(screen.getByText("配置已同步")).toBeInTheDocument());
  fixture.rejectMode = true;
  fireEvent.click(screen.getByRole("switch", { name: "国内直连" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("内核启动失败");
  await waitFor(() => expect(screen.getByRole("switch", { name: "国内直连" })).not.toBeChecked());
  expect(screen.getByRole("combobox", { name: "未匹配流量" })).toHaveTextContent("直连");
});

it("cancels unsaved parameter changes when leaving Rules mode", async () => {
  render(<App />);
  fireEvent.click(await screen.findByRole("switch", { name: "国内直连" }));
  fireEvent.click(screen.getByRole("radio", { name: "全局直连" }));
  await waitFor(() => expect(fixture.mode).toBe("direct"));
  await new Promise((resolve) => setTimeout(resolve, 450));
  expect(
    mocks.invoke.mock.calls
      .filter(([name]) => name === "set_runtime_mode")
      .map(([, args]) => args.mode),
  ).toEqual(["direct"]);
});

it("renders editable saved parameters on macOS without claiming an applied proxy", async () => {
  const original = mocks.invoke.getMockImplementation()!;
  mocks.invoke.mockImplementation((name: string, args: Record<string, unknown>) => {
    if (name === "get_capabilities")
      return Promise.resolve({
        platform: "macos",
        proxy_runtime: false,
        proxy_latency: false,
        network_recovery: false,
        startup: false,
      });
    if (name === "get_runtime_snapshot")
      return Promise.resolve({
        ...runtime(),
        applied_mode: null,
        phase: "stopped",
        session_health: "inactive",
        coverage: "none",
      });
    return original(name, args);
  });
  render(<App />);
  expect(await screen.findByRole("switch", { name: "国内直连" })).not.toBeDisabled();
  expect(screen.getByText(/此平台仅保存模式配置/)).toBeInTheDocument();
  expect(screen.getByText("已应用模式").nextElementSibling).toHaveTextContent("未应用");
});
