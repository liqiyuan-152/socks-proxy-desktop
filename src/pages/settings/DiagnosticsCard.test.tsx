import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { DiagnosticsCard } from "./DiagnosticsCard";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke }));

beforeEach(() => {
  invoke.mockReset();
  invoke.mockImplementation(async (name: string) => {
    if (name === "get_diagnostic_groups")
      return [
        {
          error_type: "proxy.not_found",
          operation: "delete_profile",
          severity: "warning",
          occurrences: 3,
          first_seen_ms: 1,
          last_seen_ms: 3,
        },
      ];
    if (name === "get_runtime_diagnostics")
      return {
        total: 105,
        next_offset: 100,
        items: [
          {
            id: "failure-42",
            created_at_ms: 3,
            severity: "warning",
            summary: JSON.stringify({ summary: "应用操作失败" }),
            error_type: "proxy.not_found",
            operation: "delete_profile",
          },
        ],
      };
    return true;
  });
});

it("shows error types, identifiers and aggregates without exporting automatically", async () => {
  render(<DiagnosticsCard />);
  expect(
    await screen.findByText(/proxy.not_found · delete_profile · 警告 · 3 次/),
  ).toBeInTheDocument();
  expect(screen.getByText(/编号：failure-42/)).toHaveTextContent("类型：proxy.not_found");
  expect(screen.getByText(/共 105 条记录/)).toHaveTextContent("导出包含当前筛选的全部记录");
  expect(screen.getByText(/警告 · 应用操作失败/)).toBeInTheDocument();
  expect(invoke.mock.calls.some(([name]) => name === "save_runtime_diagnostics")).toBe(false);
});

it("saves through the native chooser on explicit click and prevents duplicate export", async () => {
  let resolveExport!: (saved: boolean) => void;
  const original = invoke.getMockImplementation()!;
  invoke.mockImplementation((name: string, args: unknown) =>
    name === "save_runtime_diagnostics"
      ? new Promise<boolean>((resolve) => {
          resolveExport = resolve;
        })
      : original(name, args),
  );
  render(<DiagnosticsCard />);
  await screen.findByText(/共 105 条记录/);
  fireEvent.click(screen.getByRole("button", { name: "导出诊断 JSON Lines" }));
  fireEvent.click(screen.getByRole("button", { name: "正在导出…" }));
  expect(screen.getByRole("combobox", { name: "设置诊断级别" })).toBeDisabled();
  expect(invoke.mock.calls.filter(([name]) => name === "save_runtime_diagnostics")).toHaveLength(1);
  expect(invoke).toHaveBeenCalledWith("save_runtime_diagnostics", {
    filter: { from_ms: null, until_ms: null, severity: null },
  });
  await act(async () => resolveExport(true));
  expect(await screen.findByText("已保存当前筛选的全部诊断记录。")).toBeInTheDocument();
});

it("cancelling the native chooser does not report a saved file or an error", async () => {
  const original = invoke.getMockImplementation()!;
  invoke.mockImplementation((name: string, args: unknown) =>
    name === "save_runtime_diagnostics" ? Promise.resolve(false) : original(name, args),
  );
  render(<DiagnosticsCard />);
  await screen.findByText(/共 105 条记录/);
  fireEvent.click(screen.getByRole("button", { name: "导出诊断 JSON Lines" }));
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "导出诊断 JSON Lines" })).not.toBeDisabled(),
  );
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  expect(screen.queryByText("已保存当前筛选的全部诊断记录。")).not.toBeInTheDocument();
});

it("keeps export failures visible with context and supports a user retry", async () => {
  const original = invoke.getMockImplementation()!;
  invoke.mockImplementation((name: string, args: unknown) =>
    name === "save_runtime_diagnostics"
      ? Promise.reject({
          code: "storage_error",
          message: "诊断存储不可读",
          fields: [],
          context: {
            error_id: "export-42",
            timestamp_ms: 1,
            domain: "storage",
            kind: "read_failed",
            recovery_suggestion: "检查存储后重新导出",
          },
        })
      : original(name, args),
  );
  render(<DiagnosticsCard />);
  await screen.findByText(/共 105 条记录/);
  fireEvent.click(screen.getByRole("button", { name: "导出诊断 JSON Lines" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("export-42");
  expect(screen.getByRole("alert")).toHaveTextContent("检查存储后重新导出");
  fireEvent.click(screen.getByRole("button", { name: "导出诊断 JSON Lines" }));
  await waitFor(() =>
    expect(invoke.mock.calls.filter(([name]) => name === "save_runtime_diagnostics")).toHaveLength(
      2,
    ),
  );
});

it("uses the same literal search for records, aggregates and exported diagnostics", async () => {
  render(<DiagnosticsCard />);
  await screen.findByText(/共 105 条记录/);
  fireEvent.change(screen.getByRole("textbox", { name: "搜索诊断" }), {
    target: { value: "failure-42%" },
  });
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("get_runtime_diagnostics", {
      filter: { from_ms: null, until_ms: null, severity: null, search: "failure-42%" },
      offset: 0,
      limit: 100,
    }),
  );
  expect(invoke).toHaveBeenCalledWith("get_diagnostic_groups", {
    filter: { from_ms: null, until_ms: null, severity: null, search: "failure-42%" },
  });
  fireEvent.click(screen.getByRole("button", { name: "导出诊断 JSON Lines" }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("save_runtime_diagnostics", {
      filter: { from_ms: null, until_ms: null, severity: null, search: "failure-42%" },
    }),
  );
});
