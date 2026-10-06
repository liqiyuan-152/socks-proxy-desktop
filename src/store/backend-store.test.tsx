import { modeKey } from "@/lib/proxy-mode";
import { act, render, screen } from "@testing-library/react";
import { StrictMode } from "react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { BackendProvider } from "@/lib/backend-context";
import {
  createBackendStore,
  selectIsRunning,
  selectSelectedMode,
  useBackendStore,
} from "./backend-store";
import type { RuntimeSnapshot, ProfileView } from "@/lib/generated/ipc";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
const snapshot: RuntimeSnapshot = {
  revision: 1,
  configuration_revision: 0,
  runtime_plan_revision: 1,
  selected_mode: "global",
  desired_mode: "global",
  applied_mode: "global",
  phase: "running",
  session_health: "healthy",
  last_operation: { id: 1, outcome: "succeeded", error: null },
  active_profile_id: null,
  runtime_uptime_ms: 1000,
  system_proxy_enabled: true,
  tun_enabled: false,
  coverage: "system_proxy_apps",
  last_error: null,
};
const connections = {
  status: "available",
  active_count: 1,
  recent: [],
  diagnostic: null,
  history_available: false,
  trend: null,
};
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}
beforeEach(() => {
  vi.useFakeTimers();
  mocks.listen.mockReset().mockResolvedValue(() => {});
  mocks.invoke.mockReset().mockImplementation(async (name: string) => {
    if (name === "get_runtime_snapshot") return snapshot;
    if (name === "list_profiles" || name === "list_rules") return [];
    if (name === "get_active_connections") return connections;
    return null;
  });
});
afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

it("cleans up a subscription whose registration completes after disposal", async () => {
  const registration = deferred<() => void>();
  mocks.listen.mockReturnValue(registration.promise);
  const store = createBackendStore();
  await store.getState().initialize();
  store.getState().dispose();
  const unlisten = vi.fn();
  registration.resolve(unlisten);
  await Promise.resolve();
  await Promise.resolve();
  expect(unlisten).toHaveBeenCalledOnce();
});

it("isolates providers and selectors ignore unrelated connection polls", async () => {
  const rendered = vi.fn();
  function Probe() {
    const running = useBackendStore(selectIsRunning);
    rendered();
    return <p>{running ? "running" : "stopped"}</p>;
  }
  render(
    <BackendProvider>
      <Probe />
    </BackendProvider>,
  );
  await act(() => vi.advanceTimersByTimeAsync(0));
  expect(screen.getByText("running")).toBeInTheDocument();
  const count = rendered.mock.calls.length;
  await act(() => vi.advanceTimersByTimeAsync(3000));
  expect(rendered).toHaveBeenCalledTimes(count);
  const isolated = createBackendStore();
  expect(isolated.getState().snapshot).toBeNull();
  expect(selectSelectedMode(isolated.getState())).toBeNull();
});

it("survives StrictMode setup cleanup and only keeps the current subscription", async () => {
  const stops = [vi.fn(), vi.fn()];
  mocks.listen.mockResolvedValueOnce(stops[0]).mockResolvedValueOnce(stops[1]);
  function Probe() {
    const mode = useBackendStore(selectSelectedMode);
    return <p>{mode ? modeKey(mode) : ""}</p>;
  }
  const { unmount } = render(
    <StrictMode>
      <BackendProvider>
        <Probe />
      </BackendProvider>
    </StrictMode>,
  );
  await act(() => vi.advanceTimersByTimeAsync(0));
  expect(screen.getByText("global")).toBeInTheDocument();
  expect(stops[0]).toHaveBeenCalledOnce();
  expect(stops[1]).not.toHaveBeenCalled();
  unmount();
  expect(stops[1]).toHaveBeenCalledOnce();
});

it("profile mutation refreshes shared data without retaining credentials", async () => {
  const profile: ProfileView = {
    id: "proxy",
    name: "proxy",
    protocol: "socks5",
    host: "example.org",
    port: 1080,
    enabled: true,
    authentication_enabled: true,
    configuration_revision: 2,
  };
  mocks.invoke.mockImplementation(async (name: string) => {
    if (name === "save_profile") return profile;
    if (name === "list_profiles") return [profile];
    if (name === "get_runtime_snapshot") return snapshot;
    if (name === "get_active_connections") return connections;
    return null;
  });
  const store = createBackendStore();
  const result = await store.getState().saveProfile({
    ...profile,
    credential: { action: "replace", username: "private-user", password: "private-secret" },
  });
  expect(result).toEqual(profile);
  expect(store.getState().profiles).toEqual([profile]);
  expect(JSON.stringify(store.getState())).not.toContain("private-secret");
  expect(JSON.stringify(store.getState())).not.toContain("private-user");
});

it("mutation failures keep the rich error for the caller and leave shared data intact", async () => {
  const error = {
    code: "storage_error",
    message: "保存失败",
    fields: [{ field: "name", message: "重复" }],
  };
  mocks.invoke.mockRejectedValue(error);
  const store = createBackendStore();
  store.getState().applySnapshot(snapshot);
  await expect(store.getState().deleteProfile("proxy")).rejects.toEqual(error);
  expect(store.getState().snapshot).toEqual(snapshot);
  expect(store.getState().error).toBeNull();
});

it("runtime and configuration actions refresh shared resources", async () => {
  mocks.invoke.mockImplementation(async (name: string) => {
    if (name === "get_runtime_snapshot" || name === "stop_runtime") return snapshot;
    if (name === "recover_network") return { completed_at_ms: 42, snapshot };
    if (name === "list_profiles" || name === "list_rules") return [];
    if (name === "get_active_connections") return connections;
    return null;
  });
  const store = createBackendStore();
  await store.getState().selectProfile(null);
  await store.getState().replaceRules([]);
  await store.getState().reorderRules([]);
  await store.getState().importConfiguration({ json: "{}", updates: {} });
  await store.getState().stopRuntime();
  const recovery = await store.getState().recoverNetwork();
  expect(recovery.completed_at_ms).toBe(42);
  expect(store.getState().snapshot).toEqual(snapshot);
  expect(store.getState().rules).toEqual([]);
  expect(mocks.invoke.mock.calls.map(([name]) => name)).toEqual(
    expect.arrayContaining([
      "select_profile",
      "replace_rules",
      "reorder_rules",
      "import_configuration",
      "stop_runtime",
      "recover_network",
      "list_rules",
    ]),
  );
});

it("subscription failures retain normalized diagnostics and disposal suppresses later errors", async () => {
  const store = createBackendStore();
  mocks.listen.mockRejectedValue({ code: "unavailable", message: "订阅失败", fields: [] });
  await store.getState().initialize();
  await Promise.resolve();
  store.getState().reportError({ code: "unavailable", message: "读取失败", fields: [] });
  expect(store.getState().error?.message).toBe("读取失败");
  store.getState().dispose();
  store.getState().reportError(new Error("late error"));
  expect(store.getState().error?.message).toBe("读取失败");
});
