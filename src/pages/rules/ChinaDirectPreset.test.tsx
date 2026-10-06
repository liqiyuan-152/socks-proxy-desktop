import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, expect, it, vi } from "vitest";
import { ChinaDirectPreset } from "./ChinaDirectPreset";
const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke }));
beforeEach(() => {
  invoke.mockReset();
  invoke.mockResolvedValue({ enabled: false, available: true, data_date: "2026-09-23" });
});
it("links to Rules parameters instead of exposing a global switch", async () => {
  render(
    <MemoryRouter>
      <ChinaDirectPreset />
    </MemoryRouter>,
  );
  expect(await screen.findByText(/数据版本：2026-09-23/)).toBeInTheDocument();
  expect(screen.getByRole("link", { name: "在状态页配置规则代理" })).toHaveAttribute("href", "/");
  expect(screen.queryByRole("switch")).not.toBeInTheDocument();
  expect(invoke).not.toHaveBeenCalledWith("set_china_direct_enabled", expect.anything());
});
it("reports unavailable rule data", async () => {
  invoke.mockResolvedValue({ enabled: false, available: false, data_date: null });
  render(
    <MemoryRouter>
      <ChinaDirectPreset />
    </MemoryRouter>,
  );
  expect(await screen.findByText(/本地规则集不可用/)).toBeInTheDocument();
});
it("preserves a rule status error", async () => {
  invoke.mockRejectedValue(new Error("规则集损坏"));
  render(
    <MemoryRouter>
      <ChinaDirectPreset />
    </MemoryRouter>,
  );
  expect(await screen.findByRole("alert")).toHaveTextContent("规则集损坏");
});
