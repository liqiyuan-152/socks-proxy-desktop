import { act, renderHook } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import type { RuntimeSnapshot } from "@/lib/backend";
import { useModeSwitching } from "@/lib/use-mode-switching";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke }));

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

function setup() {
  const applySnapshot = vi.fn();
  const endTransition = vi.fn();
  const beginTransition = vi.fn(() => endTransition);
  const refreshConnections = vi.fn(async () => {});
  return {
    ...renderHook(() => useModeSwitching({ applySnapshot, beginTransition, refreshConnections })),
    applySnapshot,
    endTransition,
    beginTransition,
    refreshConnections,
  };
}

beforeEach(() => {
  invoke.mockReset();
});

it("coalesces queued modes and lets every caller await the completed drain", async () => {
  const pending = deferred<RuntimeSnapshot>();
  invoke.mockReturnValueOnce(pending.promise).mockResolvedValue(snapshot);
  const { result, beginTransition, endTransition } = setup();
  let first!: Promise<void>;
  let second!: Promise<void>;
  act(() => {
    first = result.current.switchMode("rules");
    result.current.switchMode("direct");
    second = result.current.switchMode("global");
  });
  expect(first).toBe(second);
  expect(result.current.selectedMode).toBe("global");
  expect(result.current.pending).toBe(true);
  expect(invoke).toHaveBeenCalledTimes(1);
  await act(async () => {
    pending.resolve(snapshot);
    await second;
  });
  expect(invoke.mock.calls.map(([, args]) => args.mode)).toEqual(["rules", "global"]);
  expect(result.current.pending).toBe(false);
  expect(result.current.selectedMode).toBeNull();
  expect(beginTransition).toHaveBeenCalledTimes(1);
  expect(endTransition).toHaveBeenCalledTimes(1);
});

it("preserves a failed selection and last snapshot when recovery also fails", async () => {
  invoke.mockRejectedValue(new Error("backend unavailable"));
  const { result, applySnapshot, endTransition } = setup();
  await act(() => result.current.switchMode("rules"));
  expect(result.current.selectedMode).toBe("rules");
  expect(result.current.pending).toBe(false);
  expect(applySnapshot).not.toHaveBeenCalled();
  expect(endTransition).toHaveBeenCalledTimes(1);
  expect(result.current.operationError).toBe("backend unavailable");
});

it("preserves the command error when the recovery snapshot succeeds and clears it after retry", async () => {
  invoke.mockRejectedValueOnce({ message: "内核启动失败" }).mockResolvedValue(snapshot);
  const { result, applySnapshot } = setup();
  await act(() => result.current.switchMode("rules"));
  expect(applySnapshot).toHaveBeenCalledWith(snapshot);
  expect(result.current.operationError).toBe("内核启动失败");
  act(() => result.current.observeSnapshot(snapshot));
  expect(result.current.operationError).toBe("内核启动失败");
  expect(result.current.selectedMode).toBe("rules");
  await act(() => result.current.switchMode("rules"));
  expect(result.current.operationError).toBeNull();
});

it("clears a prior command failure after a newer successful backend operation", async () => {
  invoke.mockRejectedValueOnce({ message: "failed" }).mockResolvedValue(snapshot);
  const { result } = setup();
  await act(() => result.current.switchMode("rules"));
  act(() =>
    result.current.observeSnapshot({
      ...snapshot,
      last_operation: { id: 2, outcome: "succeeded", error: null },
    }),
  );
  expect(result.current.operationError).toBeNull();
});

it("does not execute queued mutations or publish snapshots after unmount", async () => {
  const pending = deferred<RuntimeSnapshot>();
  invoke.mockReturnValue(pending.promise);
  const { result, unmount, applySnapshot, endTransition, refreshConnections } = setup();
  let flight!: Promise<void>;
  act(() => {
    flight = result.current.switchMode("rules");
    result.current.switchMode("global");
  });
  unmount();
  await act(async () => {
    pending.resolve(snapshot);
    await flight;
  });
  expect(invoke).toHaveBeenCalledTimes(1);
  expect(applySnapshot).not.toHaveBeenCalled();
  expect(refreshConnections).not.toHaveBeenCalled();
  expect(endTransition).toHaveBeenCalledTimes(1);
});

it("uses updated snapshot callbacks after rerender", async () => {
  invoke.mockResolvedValue(snapshot);
  const initial = vi.fn();
  const updated = vi.fn();
  const beginTransition = () => () => {};
  const refreshConnections = async () => {};
  const { result, rerender } = renderHook(
    ({ publish }) =>
      useModeSwitching({ applySnapshot: publish, beginTransition, refreshConnections }),
    { initialProps: { publish: initial } },
  );
  await act(() => result.current.switchMode("rules"));
  rerender({ publish: updated });
  await act(() => result.current.switchMode("global"));
  expect(initial).toHaveBeenCalledTimes(1);
  expect(updated).toHaveBeenCalledTimes(1);
});
