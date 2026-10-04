import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { ConfirmDeletionDialog } from "./ConfirmDeletionDialog";

it("focuses cancellation and does not delete until explicitly confirmed", async () => {
  const onConfirm = vi.fn(async () => true);
  const onCancel = vi.fn();
  render(
    <ConfirmDeletionDialog
      name="测试代理"
      resource="代理"
      error={null}
      onConfirm={onConfirm}
      onCancel={onCancel}
    />,
  );
  expect(screen.getByRole("alertdialog", { name: "确认删除代理" })).toBeInTheDocument();
  await waitFor(() => expect(screen.getByRole("button", { name: "取消" })).toHaveFocus());
  expect(onConfirm).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "取消" }));
  expect(onCancel).toHaveBeenCalledOnce();
  expect(onConfirm).not.toHaveBeenCalled();
});

it("keeps a failed deletion open with the backend error", async () => {
  const onCancel = vi.fn();
  const onConfirm = vi.fn(async () => false);
  const view = render(
    <ConfirmDeletionDialog
      name="测试代理"
      resource="代理"
      error={null}
      onConfirm={onConfirm}
      onCancel={onCancel}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "确认删除" }));
  await waitFor(() => expect(onConfirm).toHaveBeenCalledOnce());
  view.rerender(
    <ConfirmDeletionDialog
      name="测试代理"
      resource="代理"
      error="规则仍引用此代理"
      onConfirm={onConfirm}
      onCancel={onCancel}
    />,
  );
  expect(screen.getByRole("alert")).toHaveTextContent("规则仍引用此代理");
  expect(onCancel).not.toHaveBeenCalled();
  await waitFor(() => expect(screen.getByRole("button", { name: "确认删除" })).not.toBeDisabled());
});

it("blocks duplicate confirmation and dismissal until the request finishes", async () => {
  let resolve!: (value: boolean) => void;
  const onConfirm = vi.fn(
    () =>
      new Promise<boolean>((done) => {
        resolve = done;
      }),
  );
  const onCancel = vi.fn();
  render(
    <ConfirmDeletionDialog
      name="测试规则"
      resource="规则"
      error={null}
      onConfirm={onConfirm}
      onCancel={onCancel}
    />,
  );
  const confirm = screen.getByRole("button", { name: "确认删除" });
  fireEvent.click(confirm);
  fireEvent.click(confirm);
  fireEvent.keyDown(screen.getByRole("alertdialog"), { key: "Escape" });
  expect(onConfirm).toHaveBeenCalledOnce();
  expect(onCancel).not.toHaveBeenCalled();
  expect(screen.getByRole("button", { name: "取消" })).toBeDisabled();
  await act(async () => resolve(true));
  expect(onCancel).toHaveBeenCalledOnce();
});

it("shows a rejected request and lets the user retry", async () => {
  const onConfirm = vi
    .fn()
    .mockRejectedValueOnce(new Error("配置暂不可用"))
    .mockResolvedValueOnce(true);
  const onCancel = vi.fn();
  render(
    <ConfirmDeletionDialog
      name="测试规则"
      resource="规则"
      error={null}
      onConfirm={onConfirm}
      onCancel={onCancel}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "确认删除" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("配置暂不可用");
  expect(onCancel).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "确认删除" }));
  await waitFor(() => expect(onCancel).toHaveBeenCalledOnce());
});
