import { defaultRulesMode } from "@/lib/proxy-mode";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { fixture, mocks, runtime } from "./test/app-fixture";
import App from "./App";

it("shows failed operation and retained healthy mode in status and sidebar", async () => {
  const original = mocks.invoke.getMockImplementation()!;
  const failed = {
    ...runtime(),
    desired_mode: defaultRulesMode,
    phase: "failed",
    session_health: "healthy",
    last_operation: { id: 2, outcome: "failed", error: "内核健康检查失败" },
    last_error: "内核健康检查失败",
  };
  mocks.invoke.mockImplementation((name: string, args: Record<string, unknown>) =>
    name === "get_runtime_snapshot" ? Promise.resolve(failed) : original(name, args),
  );
  render(<App />);
  expect(await screen.findByRole("alert")).toHaveTextContent("内核健康检查失败");
  expect(screen.getByText(/当前仍生效的模式/)).toHaveTextContent("全局代理，原内核仍在运行");
  expect(screen.getByText("会话健康").nextElementSibling).toHaveTextContent("健康");
  expect(screen.getByText("最近操作").nextElementSibling).toHaveTextContent("失败");
  expect(screen.getByRole("link", { name: /最近操作失败 · 全局代理/ })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "重试模式切换" }));
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("set_runtime_mode", { mode: defaultRulesMode }),
  );
  expect(screen.queryByText(/stored-secret/)).not.toBeInTheDocument();
});

it("explains unfinished restoration and retries recovery only when supported", async () => {
  fixture.recoveryAvailable = true;
  const original = mocks.invoke.getMockImplementation()!;
  let recovering = true;
  mocks.invoke.mockImplementation((name: string, args: Record<string, unknown>) => {
    if (name === "get_runtime_snapshot" && recovering)
      return Promise.resolve({
        ...runtime(),
        applied_mode: null,
        phase: "failed",
        session_health: "recovery_required",
        last_error: "系统代理已被外部修改",
      });
    if (name === "recover_network") recovering = false;
    return original(name, args);
  });
  render(<App />);
  expect(await screen.findByText(/应用不会覆盖外部修改/)).toHaveAttribute("role", "alert");
  expect(screen.queryByRole("button", { name: "重试模式切换" })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "恢复系统代理" }));
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("recover_network", { confirmed: true }),
  );
  await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
});

it("retains typed failure context when snapshot repeats the same message", async () => {
  const original = mocks.invoke.getMockImplementation()!;
  mocks.invoke.mockImplementation((name: string, args: Record<string, unknown>) => {
    if (name === "set_runtime_mode")
      return Promise.reject({
        code: "runtime_failed",
        message: "内核健康检查失败",
        fields: [],
        context: {
          error_id: "runtime-error-42",
          timestamp_ms: 1,
          domain: "runtime",
          kind: "health_check",
          recovery_suggestion: "检查本地内核后重试",
        },
      });
    if (name === "get_runtime_snapshot")
      return Promise.resolve({
        ...runtime(),
        last_operation: { id: 2, outcome: "failed", error: "内核健康检查失败" },
        last_error: "内核健康检查失败",
      });
    return original(name, args);
  });
  render(<App />);
  await screen.findByRole("button", { name: "重试模式切换" });
  fireEvent.click(screen.getByRole("button", { name: "重试模式切换" }));
  expect(await screen.findByText("错误编号：runtime-error-42")).toBeInTheDocument();
  expect(screen.getByText("检查本地内核后重试")).toBeInTheDocument();
  expect(screen.getAllByRole("alert")).toHaveLength(1);
});
