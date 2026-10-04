import { expect, it, vi, afterEach } from "vitest";
import { createLatencyRequest, type LatencyTask } from "./latencyTask";
const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke: mocks.invoke }));
const task: LatencyTask = {
  task_id: "task",
  profile_id: "profile",
  configuration_revision: 1,
  state: "running",
  result: null,
  error: null,
};
afterEach(() => {
  vi.useRealTimers();
  vi.clearAllMocks();
});
it("releases a subscription returned after cancellation without polling", async () => {
  let finish!: (value: { subscription_id: string; task: LatencyTask }) => void;
  mocks.invoke.mockImplementation((name: string) =>
    name === "start_proxy_latency_task"
      ? new Promise((resolve) => {
          finish = resolve;
        })
      : Promise.resolve(),
  );
  const request = createLatencyRequest("profile", 1);
  request.cancel();
  finish({ subscription_id: "subscription", task });
  expect(await request.result).toBeNull();
  expect(mocks.invoke).toHaveBeenCalledWith("release_proxy_latency_task", {
    subscriptionId: "subscription",
  });
  expect(mocks.invoke).not.toHaveBeenCalledWith("get_proxy_latency_task", expect.anything());
});
it("polls the task and releases after success", async () => {
  vi.useFakeTimers();
  mocks.invoke.mockImplementation(async (name: string) => {
    if (name === "start_proxy_latency_task") return { subscription_id: "subscription", task };
    if (name === "get_proxy_latency_task")
      return { ...task, state: "succeeded", result: { latency_ms: 42 } };
  });
  const request = createLatencyRequest("profile", 1);
  await vi.advanceTimersByTimeAsync(100);
  expect(await request.result).toEqual({ latency_ms: 42 });
  expect(mocks.invoke).toHaveBeenCalledWith("get_proxy_latency_task", {
    subscriptionId: "subscription",
  });
  expect(mocks.invoke).toHaveBeenCalledWith("release_proxy_latency_task", {
    subscriptionId: "subscription",
  });
});
it("rejects stale revision results and releases their subscriptions", async () => {
  mocks.invoke.mockResolvedValue({
    subscription_id: "subscription",
    task: { ...task, state: "succeeded", configuration_revision: 2, result: { latency_ms: 42 } },
  });
  expect(await createLatencyRequest("profile", 1).result).toBeNull();
  expect(mocks.invoke).toHaveBeenCalledWith("release_proxy_latency_task", {
    subscriptionId: "subscription",
  });
});
