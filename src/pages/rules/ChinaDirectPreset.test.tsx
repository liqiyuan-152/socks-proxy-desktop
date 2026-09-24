import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { ChinaDirectPreset } from "./ChinaDirectPreset";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  refresh: vi.fn(async () => {}),
  defaultId: "primary" as string | null,
}));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke: mocks.invoke }));
vi.mock("@/lib/backend-context", () => ({
  useBackend: () => ({ snapshot: { active_profile_id: mocks.defaultId }, refresh: mocks.refresh }),
}));

beforeEach(() => {
  mocks.defaultId = "primary";
  mocks.refresh.mockClear();
  mocks.invoke.mockReset();
  mocks.invoke.mockResolvedValue({ enabled: false, available: true, data_date: "2026-09-23" });
});

it("shows the conservative scope and persists a successful toggle", async () => {
  render(<ChinaDirectPreset />);
  const toggle = await screen.findByRole("switch", { name: "国内直连" });
  await waitFor(() => expect(toggle).not.toBeDisabled());
  expect(screen.getByText(/域名集外的域名不按解析 IP 分流/)).toBeInTheDocument();
  expect(screen.getByText(/数据版本：2026-09-23/)).toBeInTheDocument();
  mocks.invoke.mockResolvedValue({ enabled: true, available: true, data_date: "2026-09-23" });
  fireEvent.click(toggle);
  await waitFor(() => expect(toggle).toBeChecked());
  expect(mocks.invoke).toHaveBeenCalledWith("set_china_direct_enabled", { enabled: true });
  expect(mocks.refresh).toHaveBeenCalled();
});

it("does not enable when the default exit is missing or the rule data is invalid", async () => {
  mocks.defaultId = null;
  mocks.invoke.mockResolvedValue({ enabled: false, available: false, data_date: null });
  render(<ChinaDirectPreset />);
  const toggle = await screen.findByRole("switch", { name: "国内直连" });
  expect(toggle).toBeDisabled();
  expect(screen.getByText(/请先设置默认代理/)).toBeInTheDocument();
  expect(screen.getByText(/本地规则集不可用/)).toBeInTheDocument();
});

it("keeps the switch off after a rejected revision", async () => {
  render(<ChinaDirectPreset />);
  const toggle = await screen.findByRole("switch", { name: "国内直连" });
  await waitFor(() => expect(toggle).not.toBeDisabled());
  mocks.invoke.mockRejectedValue(new Error("规则集损坏"));
  fireEvent.click(toggle);
  await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("规则集损坏"));
  expect(toggle).not.toBeChecked();
});
