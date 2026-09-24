import { afterEach, expect, it, vi } from "vitest";
import { syncSystemTheme } from "./system-theme";

afterEach(() => {
  vi.unstubAllGlobals();
  document.documentElement.classList.remove("dark");
});

it("follows the system color scheme at startup and when it changes", () => {
  const listeners = new Set<EventListenerOrEventListenerObject>();
  const preference = {
    matches: false,
    addEventListener: vi.fn((_type: string, listener: EventListenerOrEventListenerObject) => {
      listeners.add(listener);
    }),
    removeEventListener: vi.fn((_type: string, listener: EventListenerOrEventListenerObject) => {
      listeners.delete(listener);
    }),
  };
  vi.stubGlobal(
    "matchMedia",
    vi.fn(() => preference),
  );

  document.documentElement.classList.add("dark");
  const stop = syncSystemTheme();
  expect(document.documentElement).not.toHaveClass("dark");

  preference.matches = true;
  for (const listener of listeners) {
    if (typeof listener === "function") listener(new Event("change"));
    else listener.handleEvent(new Event("change"));
  }
  expect(document.documentElement).toHaveClass("dark");

  preference.matches = false;
  for (const listener of listeners) {
    if (typeof listener === "function") listener(new Event("change"));
    else listener.handleEvent(new Event("change"));
  }
  expect(document.documentElement).not.toHaveClass("dark");

  stop();
  expect(preference.removeEventListener).toHaveBeenCalledWith("change", expect.any(Function));
  expect(listeners.size).toBe(0);
});
