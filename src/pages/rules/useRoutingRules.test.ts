import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { createElement, type ReactNode } from "react";
import { BackendStoreContext, createBackendStore } from "@/store/backend-store";
import { useRoutingRules, type RoutingRule } from "./useRoutingRules";
const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke }));
const rule: RoutingRule = {
  id: "a",
  name: "A",
  matcher: "domain",
  target: "a.example",
  port_start: null,
  port_end: null,
  action: "direct",
  proxy_profile_id: null,
  enabled: true,
};
beforeEach(() => invoke.mockReset());

it("serializes same-tick writes and ignores an older list response", async () => {
  let resolveOld!: (rules: RoutingRule[]) => void;
  invoke.mockReturnValueOnce(
    new Promise((done) => {
      resolveOld = done;
    }),
  );
  const store = createBackendStore();
  const { result } = renderHook(() => useRoutingRules(), {
    wrapper: ({ children }: { children: ReactNode }) =>
      createElement(BackendStoreContext.Provider, { value: store }, children),
  });
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("list_rules", {}));
  let resolveWrite!: () => void;
  invoke.mockReturnValueOnce(
    new Promise<void>((done) => {
      resolveWrite = done;
    }),
  );
  invoke.mockImplementation(async (name: string) => (name === "list_rules" ? [rule] : null));
  let saved!: Promise<boolean>;
  let duplicate!: Promise<boolean>;
  act(() => {
    saved = result.current.replace([rule]);
    duplicate = result.current.replace([]);
  });
  expect(await duplicate).toBe(false);
  expect(invoke.mock.calls.filter(([name]) => name === "replace_rules")).toHaveLength(1);
  await act(async () => {
    resolveWrite();
    await saved;
  });
  await act(async () => resolveOld([]));
  expect(result.current.routingRules).toEqual([rule]);
  expect(result.current.busy).toBe(false);
});

it("refreshes shared rules after leaving the page without publishing local feedback", async () => {
  invoke.mockResolvedValue([]);
  const store = createBackendStore();
  const { result, unmount } = renderHook(() => useRoutingRules(), {
    wrapper: ({ children }: { children: ReactNode }) =>
      createElement(BackendStoreContext.Provider, { value: store }, children),
  });
  await waitFor(() => expect(result.current.loading).toBe(false));
  let resolve!: () => void;
  invoke.mockReturnValueOnce(
    new Promise<void>((done) => {
      resolve = done;
    }),
  );
  let saved!: Promise<boolean>;
  act(() => {
    saved = result.current.replace([rule]);
  });
  unmount();
  resolve();
  expect(await saved).toBe(false);
  expect(invoke.mock.calls.filter(([name]) => name === "list_rules")).toHaveLength(2);
  expect(store.getState().rules).toEqual([]);
});
