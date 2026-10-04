import { act, renderHook } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import type { ProxyProfile } from "@/lib/backend";
import { useProxyLatency } from "./useProxyLatency";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  toast: { loading: vi.fn(), success: vi.fn(), error: vi.fn(), dismiss: vi.fn() },
}));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke: mocks.invoke }));
vi.mock("sonner", () => ({ toast: mocks.toast }));
const profiles: ProxyProfile[] = Array.from({ length: 5 }, (_, index) => ({
  id: String(index),
  name: `Proxy ${index}`,
  protocol: "socks5",
  host: "example.org",
  port: 1080,
  authentication_enabled: true,
  enabled: true,
  configuration_revision: 1,
}));
function subscription(id: string, latency_ms: number) {
  return {
    subscription_id: `subscription-${id}`,
    task: {
      task_id: `task-${id}`,
      profile_id: id,
      configuration_revision: 1,
      state: "succeeded",
      result: { latency_ms },
      error: null,
    },
  };
}
function deferred() {
  let resolve!: (value: { latency_ms: number }) => void;
  const promise = new Promise<{ latency_ms: number }>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}
beforeEach(() => {
  vi.clearAllMocks();
  mocks.invoke.mockImplementation(async () => ({ latency_test_url: "https://example.org/check" }));
});

it("stops queued batch work and suppresses results and toasts after unmount", async () => {
  const requests = Array.from({ length: 3 }, deferred);
  let count = 0;
  mocks.invoke.mockImplementation((name: string, args: { id: string }) =>
    name === "start_proxy_latency_task"
      ? requests[count++].promise.then((result) => subscription(args.id, result.latency_ms))
      : Promise.resolve({ latency_test_url: "https://example.org/check" }),
  );
  const { result, unmount } = renderHook(() => useProxyLatency(profiles));
  let batch!: Promise<void>;
  act(() => {
    batch = result.current.testAll();
  });
  expect(count).toBe(3);
  unmount();
  await act(async () => {
    requests.forEach((request) => request.resolve({ latency_ms: 42 }));
    await batch;
  });
  expect(count).toBe(3);
  expect(mocks.toast.success).not.toHaveBeenCalled();
  expect(mocks.toast.error).not.toHaveBeenCalled();
  expect(mocks.toast.dismiss).toHaveBeenCalledWith("proxy-latency-0");
  expect(
    mocks.invoke.mock.calls.filter(([name]) => name === "release_proxy_latency_task"),
  ).toHaveLength(3);
});

it("invalidates in-flight measurements on credential-only configuration revisions", async () => {
  const request = deferred();
  mocks.invoke.mockImplementation((name: string) =>
    name === "start_proxy_latency_task"
      ? request.promise.then((result) => subscription("0", result.latency_ms))
      : Promise.resolve({ latency_test_url: "https://example.org/check" }),
  );
  const { result, rerender } = renderHook(({ items }) => useProxyLatency(items), {
    initialProps: { items: profiles },
  });
  let test!: Promise<void>;
  act(() => {
    test = result.current.test("0");
  });
  rerender({ items: profiles.map((profile) => ({ ...profile, configuration_revision: 2 })) });
  await act(async () => {
    request.resolve({ latency_ms: 42 });
    await test;
  });
  expect(result.current.results["0"]).toBeUndefined();
  expect(mocks.toast.success).not.toHaveBeenCalled();
  expect(mocks.invoke).toHaveBeenCalledWith("release_proxy_latency_task", {
    subscriptionId: "subscription-0",
  });
});

it("releases the old page subscription and measures again after re-entry", async () => {
  const old = deferred();
  let requests = 0;
  mocks.invoke.mockImplementation((name: string) => {
    if (name !== "start_proxy_latency_task") return Promise.resolve();
    requests += 1;
    return requests === 1
      ? old.promise.then((result) => subscription("0", result.latency_ms))
      : Promise.resolve(subscription("0", 21));
  });
  const first = renderHook(() => useProxyLatency(profiles));
  let pending!: Promise<void>;
  act(() => {
    pending = first.result.current.test("0");
  });
  first.unmount();
  const second = renderHook(() => useProxyLatency(profiles));
  await act(() => second.result.current.test("0"));
  expect(second.result.current.results["0"].latency).toBe(21);
  await act(async () => {
    old.resolve({ latency_ms: 99 });
    await pending;
  });
  expect(second.result.current.results["0"].latency).toBe(21);
  expect(
    mocks.invoke.mock.calls.filter(([name]) => name === "release_proxy_latency_task"),
  ).toHaveLength(2);
});

it("clears completed results after configuration changes and refuses unsupported tests", async () => {
  mocks.invoke.mockImplementation(async (name: string) =>
    name === "start_proxy_latency_task"
      ? subscription("0", 42)
      : { latency_test_url: "https://example.org/check" },
  );
  const { result, rerender } = renderHook(
    ({ items, available }) => useProxyLatency(items, available),
    { initialProps: { items: profiles, available: true } },
  );
  await act(() => result.current.test("0"));
  expect(result.current.results["0"].latency).toBe(42);
  rerender({
    items: profiles.map((profile) => ({ ...profile, configuration_revision: 2 })),
    available: false,
  });
  expect(result.current.results["0"]).toBeUndefined();
  mocks.invoke.mockClear();
  await act(async () => {
    await result.current.test("0");
    await result.current.testAll();
  });
  expect(mocks.invoke).not.toHaveBeenCalledWith("start_proxy_latency_task", expect.anything());
});
