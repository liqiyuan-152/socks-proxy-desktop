import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, it } from "vitest";
import { mocks } from "@/test/app-fixture";
import App from "@/App";

async function openSettings() {
  window.history.pushState({}, "", "/settings");
  render(<App />);
  const button = await screen.findByRole("button", { name: "导出配置" });
  await waitFor(() => expect(button).not.toBeDisabled());
  return button;
}

it("exports only on click and prevents duplicate requests while the native chooser is open", async () => {
  const original = mocks.invoke.getMockImplementation()!;
  let finish!: (saved: boolean) => void;
  mocks.invoke.mockImplementation((name: string, args: Record<string, unknown>) =>
    name === "save_configuration"
      ? new Promise<boolean>((resolve) => {
          finish = resolve;
        })
      : original(name, args),
  );
  const button = await openSettings();
  expect(mocks.invoke.mock.calls.some(([name]) => name === "save_configuration")).toBe(false);
  fireEvent.click(button);
  fireEvent.click(button);
  expect(mocks.invoke.mock.calls.filter(([name]) => name === "save_configuration")).toHaveLength(1);
  expect(button).toBeDisabled();
  await act(async () => finish(true));
  expect(await screen.findByText("已保存不含密码的配置；请妥善保管文件。")).toBeInTheDocument();
});

it("does not report success or failure when the native chooser is cancelled", async () => {
  const original = mocks.invoke.getMockImplementation()!;
  mocks.invoke.mockImplementation((name: string, args: Record<string, unknown>) =>
    name === "save_configuration" ? Promise.resolve(false) : original(name, args),
  );
  const button = await openSettings();
  fireEvent.click(button);
  await waitFor(() => expect(button).not.toBeDisabled());
  expect(screen.queryByText("已保存不含密码的配置；请妥善保管文件。")).not.toBeInTheDocument();
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});

it("keeps save failures actionable and allows a fresh user retry", async () => {
  const original = mocks.invoke.getMockImplementation()!;
  let attempts = 0;
  mocks.invoke.mockImplementation((name: string, args: Record<string, unknown>) => {
    if (name !== "save_configuration") return original(name, args);
    if (++attempts === 1)
      return Promise.reject({
        code: "storage_error",
        message: "导出目录不可写",
        fields: [],
        context: {
          error_id: "export-config-42",
          timestamp_ms: 1,
          domain: "storage",
          kind: "write_failed",
          recovery_suggestion: "选择可写目录后重试",
        },
      });
    return Promise.resolve(true);
  });
  const button = await openSettings();
  fireEvent.click(button);
  expect(await screen.findByRole("alert")).toHaveTextContent("export-config-42");
  expect(screen.getByRole("alert")).toHaveTextContent("选择可写目录后重试");
  fireEvent.click(button);
  expect(await screen.findByText("已保存不含密码的配置；请妥善保管文件。")).toBeInTheDocument();
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});
