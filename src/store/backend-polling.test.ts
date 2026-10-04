import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { useEffect } from "react";
import { RUNTIME_POLL_INTERVAL_MS, startRuntimePolling } from "@/store/backend-polling";
function usePollingFixture(refresh: () => Promise<void>) {
  useEffect(() => startRuntimePolling(refresh), [refresh]);
}

async function tick(ms = RUNTIME_POLL_INTERVAL_MS) {
  await act(() => vi.advanceTimersByTimeAsync(ms));
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.spyOn(document, "visibilityState", "get").mockReturnValue("visible");
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

it("polls without runtime transition events", async () => {
  const refresh = vi.fn(async () => {});
  renderHook(() => usePollingFixture(refresh));
  await tick(3000);
  expect(refresh).toHaveBeenCalledTimes(3);
});

it("does not overlap slow refreshes", async () => {
  let finish!: () => void;
  const pending = new Promise<void>((resolve) => {
    finish = resolve;
  });
  const refresh = vi.fn(() => pending);
  renderHook(() => usePollingFixture(refresh));
  await tick(5000);
  expect(refresh).toHaveBeenCalledTimes(1);
  await act(async () => finish());
  await tick();
  expect(refresh).toHaveBeenCalledTimes(2);
});

it("pauses when hidden and immediately refreshes when visible again", async () => {
  const visibility = vi.spyOn(document, "visibilityState", "get");
  const refresh = vi.fn(async () => {});
  renderHook(() => usePollingFixture(refresh));
  visibility.mockReturnValue("hidden");
  act(() => document.dispatchEvent(new Event("visibilitychange")));
  await tick(5000);
  expect(refresh).not.toHaveBeenCalled();
  visibility.mockReturnValue("visible");
  act(() => document.dispatchEvent(new Event("visibilitychange")));
  await tick(0);
  expect(refresh).toHaveBeenCalledTimes(1);
});

it("coalesces focus events while a request is pending", async () => {
  let finish!: () => void;
  const pending = new Promise<void>((resolve) => {
    finish = resolve;
  });
  const refresh = vi.fn(() => pending);
  renderHook(() => usePollingFixture(refresh));
  await tick();
  act(() => window.dispatchEvent(new Event("focus")));
  await tick(0);
  expect(refresh).toHaveBeenCalledTimes(1);
  await act(async () => finish());
  await tick(0);
  expect(refresh).toHaveBeenCalledTimes(2);
});

it("cleans up timers and listeners on unmount even with a pending refresh", async () => {
  let finish!: () => void;
  const pending = new Promise<void>((resolve) => {
    finish = resolve;
  });
  const refresh = vi.fn(() => pending);
  const { unmount } = renderHook(() => usePollingFixture(refresh));
  await tick();
  unmount();
  await act(async () => finish());
  act(() => window.dispatchEvent(new Event("focus")));
  act(() => document.dispatchEvent(new Event("visibilitychange")));
  await tick(5000);
  expect(refresh).toHaveBeenCalledTimes(1);
});
