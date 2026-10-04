import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ActiveConnectionsSnapshot, ProxyProfile, RuntimeSnapshot } from "@/lib/backend";
import { useBackendObservation } from "@/lib/use-backend-observation";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke }));

const running: RuntimeSnapshot = {
  revision: 1,
  selected_mode: "global",
  desired_mode: "global",
  applied_mode: "global",
  phase: "running",
  active_profile_id: "primary",
  runtime_uptime_ms: 1000,
  system_proxy_enabled: true,
  tun_enabled: false,
  coverage: "system_proxy_apps",
  last_error: null,
};
const connections: ActiveConnectionsSnapshot = {
  status: "available",
  active_count: 1,
  recent: [],
  diagnostic: null,
  history_available: false,
};
const primary: ProxyProfile = {
  id: "primary",
  name: "Primary",
  protocol: "socks5",
  host: "example.org",
  port: 1080,
  enabled: true,
  authentication_enabled: false,
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

async function tick(ms = 1000) {
  await act(() => vi.advanceTimersByTimeAsync(ms));
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.spyOn(document, "visibilityState", "get").mockReturnValue("visible");
  invoke.mockReset().mockImplementation(async (name: string) => {
    if (name === "get_runtime_snapshot") return running;
    if (name === "list_profiles") return [primary];
    return connections;
  });
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

it("connection refreshes cannot cancel an independent runtime read", async () => {
  const { result } = renderHook(() => useBackendObservation());
  await tick(0);
  const pending = deferred<RuntimeSnapshot>();
  invoke.mockImplementation((name: string) =>
    name === "get_runtime_snapshot" ? pending.promise : Promise.resolve(connections),
  );
  await tick();
  await act(() => result.current.refreshConnections());
  await act(async () => pending.resolve({ ...running, runtime_uptime_ms: 3000 }));
  expect(result.current.snapshot?.runtime_uptime_ms).toBe(3000);
  expect(result.current.connections?.active_count).toBe(1);
});

it("keeps newer profiles when refresh responses arrive out of order", async () => {
  const { result } = renderHook(() => useBackendObservation());
  await tick(0);
  const pending = deferred<ProxyProfile[]>();
  invoke.mockImplementation(async (name: string) => {
    if (name === "get_runtime_snapshot") return running;
    if (name === "list_profiles") return pending.promise;
    return connections;
  });
  let first!: Promise<void>;
  act(() => {
    first = result.current.refresh();
  });
  const renamed = { ...primary, name: "Renamed" };
  invoke.mockImplementation(async (name: string) => {
    if (name === "get_runtime_snapshot") return running;
    if (name === "list_profiles") return [renamed];
    return connections;
  });
  await act(() => result.current.refresh());
  await act(async () => {
    pending.resolve([primary]);
    await first;
  });
  expect(result.current.profiles).toEqual([renamed]);
});

it("retains runtime state while failed connections degrade, then recovers", async () => {
  const { result } = renderHook(() => useBackendObservation());
  await tick(0);
  invoke.mockRejectedValue(new Error("unavailable"));
  await tick();
  expect(result.current.snapshot).toEqual(running);
  expect(result.current.connections).toMatchObject({
    status: "degraded",
    active_count: null,
    recent: [],
  });
  invoke.mockImplementation(async (name: string) =>
    name === "get_runtime_snapshot" ? running : connections,
  );
  await tick();
  expect(result.current.connections).toEqual(connections);
});

it("invalidates old reads during a mode transition and resumes afterwards", async () => {
  const { result } = renderHook(() => useBackendObservation());
  await tick(0);
  const pending = deferred<RuntimeSnapshot>();
  invoke.mockImplementation((name: string) =>
    name === "get_runtime_snapshot" ? pending.promise : Promise.resolve(connections),
  );
  await tick();
  let finish!: () => void;
  const stopped = { ...running, phase: "stopped" as const, runtime_uptime_ms: null };
  act(() => {
    finish = result.current.beginTransition();
    result.current.applySnapshot(stopped);
  });
  await act(async () => pending.resolve(running));
  await tick();
  expect(result.current.snapshot).toEqual(stopped);
  expect(invoke.mock.calls.filter(([name]) => name === "get_runtime_snapshot")).toHaveLength(2);
  act(() => finish());
  await tick();
  expect(result.current.snapshot).toEqual(running);
});

it("ignores pending reads from an unmounted observation", async () => {
  const { result, unmount } = renderHook(() => useBackendObservation());
  await tick(0);
  const pending = deferred<RuntimeSnapshot>();
  invoke.mockImplementation((name: string) =>
    name === "get_runtime_snapshot" ? pending.promise : Promise.resolve(connections),
  );
  await tick();
  unmount();
  await act(async () => pending.resolve({ ...running, runtime_uptime_ms: 9000 }));
  expect(result.current.snapshot).toEqual(running);
});
