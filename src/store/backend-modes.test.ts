import { beforeEach, expect, it, vi } from "vitest";
import type { RuntimeSnapshot } from "@/lib/backend";
import { createBackendStore } from "@/store/backend-store";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));

const snapshot: RuntimeSnapshot = {
  revision: 1,
  configuration_revision: 0,
  runtime_plan_revision: 1,
  selected_mode: "global",
  desired_mode: "global",
  applied_mode: "global",
  phase: "running",
  active_profile_id: null,
  runtime_uptime_ms: 1000,
  system_proxy_enabled: true,
  tun_enabled: false,
  coverage: "system_proxy_apps",
  session_health: "healthy",
  last_operation: { id: 1, outcome: "succeeded", error: null },
  last_error: null,
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

beforeEach(() => {
  invoke.mockReset();
});

it("coalesces queued modes and shares one completed drain", async () => {
  const pending = deferred<RuntimeSnapshot>();
  invoke.mockReturnValueOnce(pending.promise).mockResolvedValue(snapshot);
  const store = createBackendStore();
  const first = store.getState().switchMode("rules");
  void store.getState().switchMode("direct");
  const second = store.getState().switchMode("global");
  expect(first).toBe(second);
  expect(store.getState()).toMatchObject({ selectedMode: "global", pending: true });
  expect(invoke).toHaveBeenCalledTimes(1);
  pending.resolve(snapshot);
  await second;
  expect(
    invoke.mock.calls.filter(([name]) => name === "set_runtime_mode").map(([, args]) => args.mode),
  ).toEqual(["rules", "global"]);
  expect(store.getState()).toMatchObject({ selectedMode: null, pending: false });
});

it("keeps failed selection and authoritative snapshot when recovery fails", async () => {
  invoke.mockRejectedValue(new Error("backend unavailable"));
  const store = createBackendStore();
  store.getState().applySnapshot(snapshot);
  await store.getState().switchMode("rules");
  expect(store.getState()).toMatchObject({
    selectedMode: "rules",
    pending: false,
    snapshot,
    operationError: { message: "backend unavailable" },
  });
});

it("preserves rich command error across recovery and clears it after retry", async () => {
  const error = {
    code: "unavailable",
    message: "内核启动失败",
    fields: [],
    context: {
      error_id: "id",
      timestamp_ms: 1,
      domain: "runtime",
      kind: "unavailable",
      operation: "mode",
      recovery_suggestion: "重试",
    },
  };
  invoke.mockRejectedValueOnce(error).mockResolvedValue(snapshot);
  const store = createBackendStore();
  await store.getState().switchMode("rules");
  expect(store.getState().snapshot).toEqual(snapshot);
  expect(store.getState().operationError).toMatchObject(error);
  store.getState().applySnapshot(snapshot);
  expect(store.getState().operationError).toMatchObject(error);
  await store.getState().switchMode("rules");
  expect(store.getState().operationError).toBeNull();
});

it("clears command failure after a newer successful backend operation", async () => {
  invoke.mockRejectedValueOnce({ message: "failed" }).mockResolvedValue(snapshot);
  const store = createBackendStore();
  await store.getState().switchMode("rules");
  store
    .getState()
    .applySnapshot({ ...snapshot, last_operation: { id: 2, outcome: "succeeded", error: null } });
  expect(store.getState().operationError).toBeNull();
});

it("ignores queued mutations and late snapshots after disposal", async () => {
  const pending = deferred<RuntimeSnapshot>();
  invoke.mockReturnValue(pending.promise);
  const store = createBackendStore();
  const flight = store.getState().switchMode("rules");
  void store.getState().switchMode("global");
  store.getState().dispose();
  pending.resolve(snapshot);
  await flight;
  expect(invoke).toHaveBeenCalledTimes(1);
  expect(store.getState().snapshot).toBeNull();
});
