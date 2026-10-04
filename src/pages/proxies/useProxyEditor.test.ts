import { act, renderHook } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { useProxyEditor } from "./useProxyEditor";
import type { ProxyProfile } from "@/lib/backend";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke }));
const profile: ProxyProfile = {
  id: "a",
  name: "A",
  protocol: "socks5",
  host: "a.example",
  port: 1080,
  authentication_enabled: true,
  enabled: true,
  configuration_revision: 1,
};
function options() {
  return {
    busy: false,
    setBusy: vi.fn(),
    setError: vi.fn(),
    refresh: vi.fn().mockResolvedValue(undefined),
    clearLatency: vi.fn(),
  };
}
beforeEach(() => invoke.mockReset());

it("preserves unchanged credentials and serializes same-tick saves", async () => {
  invoke.mockResolvedValueOnce({ username: "alice", password: "secret" });
  const { result } = renderHook(() => useProxyEditor(options()));
  await act(async () => result.current.openDialog(profile));
  let resolve!: () => void;
  invoke.mockReturnValueOnce(
    new Promise<void>((done) => {
      resolve = done;
    }),
  );
  let saved!: Promise<void>;
  act(() => {
    saved = result.current.saveProfile();
    void result.current.saveProfile();
  });
  expect(invoke.mock.calls.filter(([name]) => name === "save_profile")).toHaveLength(1);
  expect(invoke.mock.calls[1][1].input.credential).toEqual({ action: "preserve" });
  await act(async () => {
    resolve();
    await saved;
  });
  expect(result.current.dialogOpen).toBe(false);
  expect(result.current.draft.password).toBe("");
});

it("does not close a new editor when an older save returns", async () => {
  const { result } = renderHook(() => useProxyEditor(options()));
  act(() => result.current.openDialog());
  let resolve!: () => void;
  invoke.mockReturnValueOnce(
    new Promise<void>((done) => {
      resolve = done;
    }),
  );
  let saved!: Promise<void>;
  act(() => {
    saved = result.current.saveProfile();
  });
  act(() => {
    result.current.closeDialog();
    result.current.openDialog();
  });
  await act(async () => {
    resolve();
    await saved;
  });
  expect(result.current.dialogOpen).toBe(true);
});

it("imported link credentials cannot be overwritten by an old credential read", async () => {
  let resolve!: (value: { username: string; password: string }) => void;
  invoke.mockReturnValue(
    new Promise((done) => {
      resolve = done;
    }),
  );
  const { result } = renderHook(() => useProxyEditor(options()));
  act(() => result.current.openDialog(profile));
  act(() => result.current.setProxyLink("socks5://new:replacement@b.example:1080"));
  act(() => result.current.applyProxyLink());
  await act(async () => resolve({ username: "old", password: "retired" }));
  expect(result.current.draft.username).toBe("new");
  expect(result.current.draft.password).toBe("replacement");
  expect(result.current.credentialLoading).toBe(false);
});
