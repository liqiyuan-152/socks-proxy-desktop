import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { scheduleStartupReady } from "./startup-ready";

const mocks = vi.hoisted(() => ({ native: true, invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => mocks.native,
  invoke: mocks.invoke,
}));
let visibility: DocumentVisibilityState;
let sequence: number;
const frames = new Map<number, FrameRequestCallback>();
const cleanups: (() => void)[] = [];

function nextFrame() {
  const pending = [...frames.values()];
  frames.clear();
  for (const callback of pending) callback(0);
}

beforeEach(() => {
  visibility = "visible";
  sequence = 0;
  frames.clear();
  mocks.native = true;
  mocks.invoke.mockReset().mockResolvedValue(true);
  vi.spyOn(document, "visibilityState", "get").mockImplementation(() => visibility);
  vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
    frames.set(++sequence, callback);
    return sequence;
  });
  vi.stubGlobal("cancelAnimationFrame", (id: number) => frames.delete(id));
});

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it("acknowledges only after two visible frames without sending timing or user data", () => {
  cleanups.push(scheduleStartupReady());
  nextFrame();
  expect(mocks.invoke).not.toHaveBeenCalled();
  nextFrame();
  expect(mocks.invoke).toHaveBeenCalledExactlyOnceWith("acknowledge_frontend_ready", {});
  document.dispatchEvent(new Event("visibilitychange"));
  nextFrame();
  expect(mocks.invoke).toHaveBeenCalledTimes(1);
});

it("waits for visibility and cancels pending readiness when unmounted", () => {
  visibility = "hidden";
  const cleanup = scheduleStartupReady();
  cleanups.push(cleanup);
  nextFrame();
  expect(mocks.invoke).not.toHaveBeenCalled();
  visibility = "visible";
  document.dispatchEvent(new Event("visibilitychange"));
  nextFrame();
  cleanup();
  nextFrame();
  document.dispatchEvent(new Event("visibilitychange"));
  expect(frames.size).toBe(0);
  expect(mocks.invoke).not.toHaveBeenCalled();
});

it("restarts the visible-frame check after hiding between frames", () => {
  cleanups.push(scheduleStartupReady());
  nextFrame();
  visibility = "hidden";
  nextFrame();
  expect(mocks.invoke).not.toHaveBeenCalled();
  visibility = "visible";
  document.dispatchEvent(new Event("visibilitychange"));
  nextFrame();
  nextFrame();
  expect(mocks.invoke).toHaveBeenCalledTimes(1);
});

it("skips browser preview and absorbs telemetry failure", async () => {
  mocks.native = false;
  cleanups.push(scheduleStartupReady());
  expect(frames.size).toBe(0);
  mocks.native = true;
  mocks.invoke.mockRejectedValue(new Error("unavailable"));
  cleanups.push(scheduleStartupReady());
  nextFrame();
  nextFrame();
  await Promise.resolve();
  await Promise.resolve();
  expect(mocks.invoke).toHaveBeenCalledTimes(1);
});
