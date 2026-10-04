import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { BackendProvider } from "@/lib/backend-context";
import { useBackend } from "@/lib/backend-state";
import type { RuntimeSnapshot } from "@/lib/backend";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));

let uptime: number;
let count: number;

function runtimeSnapshot(): RuntimeSnapshot {
  return {
    revision: 1,
    configuration_revision: 0,
    runtime_plan_revision: 1,
    selected_mode: "global",
    desired_mode: "global",
    applied_mode: "global",
    phase: "running",
    active_profile_id: null,
    runtime_uptime_ms: uptime,
    system_proxy_enabled: true,
    tun_enabled: false,
    coverage: "system_proxy_apps",
    session_health: "healthy",
    last_operation: { id: 1, outcome: "succeeded", error: null },
    last_error: null,
  };
}

function Probe() {
  const { snapshot, connections } = useBackend();
  return (
    <p>
      {snapshot?.runtime_uptime_ms} ms / {connections?.active_count} connections
    </p>
  );
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.spyOn(document, "visibilityState", "get").mockReturnValue("visible");
  uptime = 1000;
  count = 1;
  mocks.listen.mockReset().mockResolvedValue(() => {});
  mocks.invoke.mockReset().mockImplementation(async (name: string) => {
    if (name === "get_runtime_snapshot") return runtimeSnapshot();
    if (name === "list_profiles") return [];
    if (name === "get_active_connections") {
      return {
        status: "available",
        active_count: count,
        recent: [],
        diagnostic: null,
        history_available: false,
      };
    }
  });
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

it("updates rendered backend data while mode and revision remain unchanged", async () => {
  render(
    <BackendProvider>
      <Probe />
    </BackendProvider>,
  );
  await act(() => vi.advanceTimersByTimeAsync(0));
  expect(screen.getByText("1000 ms / 1 connections")).toBeInTheDocument();
  uptime = 2000;
  count = 0;
  await act(() => vi.advanceTimersByTimeAsync(1000));
  expect(screen.getByText("2000 ms / 0 connections")).toBeInTheDocument();
});

it("does not let an old poll undo a runtime state event", async () => {
  render(
    <BackendProvider>
      <Probe />
    </BackendProvider>,
  );
  await act(() => vi.advanceTimersByTimeAsync(0));
  let resolve!: (value: RuntimeSnapshot) => void;
  const pending = new Promise<RuntimeSnapshot>((done) => {
    resolve = done;
  });
  mocks.invoke.mockImplementation((name: string) => {
    if (name === "get_runtime_snapshot") return pending;
    return Promise.resolve({
      status: "available",
      active_count: 0,
      recent: [],
      diagnostic: null,
      history_available: false,
    });
  });
  await act(() => vi.advanceTimersByTimeAsync(1000));
  const listener = mocks.listen.mock.calls[0][1] as (event: { payload: RuntimeSnapshot }) => void;
  await act(async () =>
    listener({
      payload: {
        ...runtimeSnapshot(),
        runtime_uptime_ms: null,
        phase: "stopped",
        applied_mode: null,
      },
    }),
  );
  await act(async () => resolve(runtimeSnapshot()));
  expect(screen.getByText("ms / 0 connections")).toBeInTheDocument();
});
