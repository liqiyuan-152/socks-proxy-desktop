import { act, renderHook } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { ipc } from "@/lib/ipc";
import type { CommandMap } from "@/lib/generated/ipc";
import { useConfigurationImport } from "./useConfigurationImport";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke }));
const json = JSON.stringify({
  schema_version: 2,
  profiles: [{ id: "proxy", name: "Proxy", authentication_enabled: true }],
});
function file(text: () => Promise<string>) {
  return { text } as File;
}
function deferred() {
  let resolve!: (text: string) => void;
  const promise = new Promise<string>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}
beforeEach(() => {
  invoke.mockReset().mockResolvedValue(null);
});

it("keeps the newest file and ignores reads completed after closing or unmount", async () => {
  const onImported = vi.fn();
  const { result, unmount } = renderHook(() =>
    useConfigurationImport(onImported, (input: CommandMap["import_configuration"][0]) =>
      ipc("import_configuration", input),
    ),
  );
  const first = deferred();
  let read!: Promise<void>;
  act(() => {
    read = result.current.read(file(() => first.promise));
  });
  await act(() => result.current.read(file(async () => json)));
  act(() => result.current.updateCredential("proxy", "password", "secret"));
  await act(async () => {
    first.resolve("{}");
    await read;
  });
  expect(result.current.credentials.proxy.password).toBe("secret");
  expect(result.current.error).toBeNull();
  const late = deferred();
  act(() => {
    read = result.current.read(file(() => late.promise));
    result.current.close();
  });
  await act(async () => {
    late.resolve(json);
    await read;
  });
  expect(result.current.open).toBe(false);
  expect(result.current.credentials).toEqual({});
  const afterUnmount = deferred();
  act(() => {
    read = result.current.read(file(() => afterUnmount.promise));
  });
  unmount();
  await act(async () => {
    afterUnmount.resolve(json);
    await read;
  });
  expect(onImported).not.toHaveBeenCalled();
});

it.each([
  [() => Promise.reject(new Error("read")), "无法读取"],
  [async () => "{", "有效的 JSON"],
  [async () => "{}", "结构无效"],
  [async () => JSON.stringify({ schema_version: 2, profiles: [{ id: "bad" }] }), "结构无效"],
])("distinguishes file, JSON and structure failures", async (text, message) => {
  const { result } = renderHook(() =>
    useConfigurationImport(vi.fn(), (input) => ipc("import_configuration", input)),
  );
  await act(() => result.current.read(file(text)));
  expect(result.current.error?.message).toContain(message);
  expect(result.current.open).toBe(false);
});

it("guards duplicate submissions and clears credentials and input after success", async () => {
  let finish!: () => void;
  invoke.mockImplementation(
    () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
  );
  const onImported = vi.fn(async () => {});
  const { result } = renderHook(() =>
    useConfigurationImport(onImported, (input: CommandMap["import_configuration"][0]) =>
      ipc("import_configuration", input),
    ),
  );
  await act(() => result.current.read(file(async () => json)));
  act(() => {
    result.current.updateCredential("proxy", "username", "alice");
    result.current.updateCredential("proxy", "password", "secret");
  });
  let submit!: Promise<void>;
  act(() => {
    submit = result.current.submit();
    void result.current.submit();
  });
  expect(invoke).toHaveBeenCalledTimes(1);
  await act(async () => {
    finish();
    await submit;
  });
  expect(result.current.open).toBe(false);
  expect(result.current.credentials).toEqual({});
  expect(result.current.profiles).toEqual([]);
  expect(onImported).toHaveBeenCalledTimes(1);
  await act(() => result.current.read(file(async () => json)));
  expect(result.current.credentials).toEqual({});
});
