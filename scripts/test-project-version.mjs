import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import test from "node:test";

const root = new URL("../", import.meta.url);
test("frontend, Tauri and Rust package versions agree", async () => {
  const [packageText, tauriText, cargoText, lockText] = await Promise.all(
    [
      "package.json",
      "src-tauri/tauri.conf.json",
      "src-tauri/Cargo.toml",
      "src-tauri/Cargo.lock",
    ].map((path) => readFile(new URL(path, root), "utf8")),
  );
  const version = JSON.parse(packageText).version;
  assert.match(version, /^\d+\.\d+\.\d+$/);
  assert.equal(JSON.parse(tauriText).version, version);
  const packageSection = cargoText.split("[package]")[1]?.split(/\n\[/)[0];
  assert.equal(packageSection?.match(/^version = "([^"]+)"/m)?.[1], version);
  assert.equal(lockText.match(/name = "socks-proxy"\nversion = "([^"]+)"/)?.[1], version);
});
