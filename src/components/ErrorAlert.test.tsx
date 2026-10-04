import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { ErrorAlert } from "./ErrorAlert";

it("shows the backend message, fields, recovery suggestion and identity", () => {
  render(
    <ErrorAlert
      error={{
        code: "proxy_in_use",
        message: "代理被引用",
        fields: [{ field: "rules[0]", message: "规则仍引用" }],
        context: {
          error_id: "failure-1",
          timestamp_ms: 42,
          domain: "proxy",
          kind: "in_use",
          recovery_suggestion: "请先修改规则",
        },
      }}
    />,
  );
  expect(screen.getByRole("alert")).toHaveTextContent("代理被引用");
  expect(screen.getByText("rules[0]: 规则仍引用")).toBeInTheDocument();
  expect(screen.getByText("请先修改规则")).toBeInTheDocument();
  expect(screen.getByText("错误编号：failure-1")).toBeInTheDocument();
});

it("does not invoke recovery actions before explicit clicks", () => {
  const onRetry = vi.fn();
  const onOpenDiagnostics = vi.fn();
  render(
    <ErrorAlert
      error={new Error("配置不可用")}
      onRetry={onRetry}
      onOpenDiagnostics={onOpenDiagnostics}
    />,
  );
  expect(onRetry).not.toHaveBeenCalled();
  expect(onOpenDiagnostics).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "查看诊断" }));
  expect(onOpenDiagnostics).toHaveBeenCalledOnce();
  expect(onRetry).not.toHaveBeenCalled();
});

it("prevents duplicate retries and offers another retry after a failure", async () => {
  let reject!: (reason: Error) => void;
  const onRetry = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise<void>((_, fail) => {
          reject = fail;
        }),
    )
    .mockResolvedValue(undefined);
  render(<ErrorAlert error={{ code: "storage_error", message: "存储不可用" }} onRetry={onRetry} />);
  const retry = screen.getByRole("button", { name: "重试" });
  fireEvent.click(retry);
  fireEvent.click(retry);
  expect(onRetry).toHaveBeenCalledOnce();
  expect(screen.getByRole("button", { name: "正在重试…" })).toBeDisabled();
  await act(async () => reject(new Error("仍无法读取配置")));
  expect(screen.getByRole("alert")).toHaveTextContent("仍无法读取配置");
  fireEvent.click(screen.getByRole("button", { name: "重试" }));
  await waitFor(() => expect(onRetry).toHaveBeenCalledTimes(2));
});

it("renders field text safely instead of interpreting markup", () => {
  render(
    <ErrorAlert
      error={{
        message: "<script>secret</script>",
        fields: [{ field: "port", message: "<img src=x>" }],
      }}
    />,
  );
  expect(screen.getByText("<script>secret</script>")).toBeInTheDocument();
  expect(screen.getByText("port: <img src=x>")).toBeInTheDocument();
  expect(screen.getByRole("alert").querySelector("script, img")).toBeNull();
});

it("handles an unknown error and removes duplicate field details", () => {
  const view = render(<ErrorAlert error={undefined} />);
  expect(screen.getByRole("alert")).toHaveTextContent("操作失败，请检查运行时状态。");
  view.rerender(
    <ErrorAlert
      error={{
        message: "配置无效",
        fields: [
          { field: "port", message: "端口无效" },
          { field: "port", message: "端口无效" },
        ],
      }}
    />,
  );
  expect(screen.getAllByText("port: 端口无效")).toHaveLength(1);
});

it("replaces a stale retry failure when a new backend error arrives", async () => {
  const onRetry = vi.fn().mockRejectedValue(new Error("上次重试失败"));
  const view = render(<ErrorAlert error={new Error("原错误")} onRetry={onRetry} />);
  fireEvent.click(screen.getByRole("button", { name: "重试" }));
  expect(await screen.findByText("上次重试失败")).toBeInTheDocument();
  view.rerender(<ErrorAlert error={new Error("新错误")} onRetry={onRetry} />);
  expect(screen.getByRole("alert")).toHaveTextContent("新错误");
  expect(screen.queryByText("上次重试失败")).not.toBeInTheDocument();
});
