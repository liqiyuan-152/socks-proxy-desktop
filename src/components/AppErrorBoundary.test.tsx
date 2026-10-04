import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { AppErrorBoundary } from "./AppErrorBoundary";

it("contains a render failure and can recreate the UI after retry", () => {
  const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
  let failed = true;
  function Page() {
    if (failed) throw new Error("private technical details");
    return <p>代理列表</p>;
  }
  try {
    render(
      <AppErrorBoundary>
        <Page />
      </AppErrorBoundary>,
    );
    expect(screen.getByRole("alert")).toHaveTextContent("界面出现异常");
    expect(screen.queryByText("private technical details")).not.toBeInTheDocument();
    failed = false;
    fireEvent.click(screen.getByRole("button", { name: "重试" }));
    expect(screen.getByText("代理列表")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  } finally {
    consoleError.mockRestore();
  }
});

it("renders a healthy child without a fallback", () => {
  render(
    <AppErrorBoundary>
      <p>正常界面</p>
    </AppErrorBoundary>,
  );
  expect(screen.getByText("正常界面")).toBeInTheDocument();
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});
