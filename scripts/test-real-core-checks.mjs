import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { verifyCore } from "./test-real-core.mjs";

test("real-core gate rejects unsupported platform, missing input and alternate path", async () => {
  await assert.rejects(verifyCore(undefined, "darwin"), /Windows/u);
  await assert.rejects(verifyCore(undefined, "win32"), /SING_BOX_TEST_BIN/u);
  await assert.rejects(verifyCore("elsewhere/sing-box.exe", "win32"), /固定资源路径/u);
});

test("real-core gate rejects absent or substituted executable before running tests", async () => {
  const directory = await mkdtemp(join(tmpdir(), "core-gate-"));
  const previous = process.cwd();
  try {
    process.chdir(directory);
    const binary = join(process.cwd(), "src-tauri/resources/sing-box/windows-amd64/sing-box.exe");
    await assert.rejects(verifyCore(binary, "win32"), { code: "ENOENT" });
    await mkdir(join(directory, "src-tauri/resources/sing-box/windows-amd64"), { recursive: true });
    await writeFile(binary, "untrusted executable");
    await assert.rejects(verifyCore(binary, "win32"), /SHA-256/u);
  } finally {
    process.chdir(previous);
    await rm(directory, { recursive: true, force: true });
  }
});
