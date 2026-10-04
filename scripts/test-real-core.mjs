import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

const version = "1.14.1";
const expectedHash = "b838de45bd0b2e6ddbed1977e4745622f7dffab3b293807ff4c6b1b640fed909";

export async function verifyCore(binary, platform = process.platform) {
  if (platform !== "win32") throw new Error("真实内核门禁必须在 Windows 执行");
  if (!binary) throw new Error("缺少 SING_BOX_TEST_BIN");
  const fixedPath = resolve("src-tauri/resources/sing-box/windows-amd64/sing-box.exe");
  if (resolve(binary) !== fixedPath) throw new Error("测试内核必须使用固定资源路径");
  const hash = createHash("sha256")
    .update(await readFile(binary))
    .digest("hex");
  if (hash !== expectedHash) throw new Error("测试内核 SHA-256 不匹配");
  const result = spawnSync(binary, ["version"], { encoding: "utf8", timeout: 5000 });
  if (
    result.status !== 0 ||
    !result.stdout.split(/\r?\n/u).includes(`sing-box version ${version}`)
  ) {
    throw new Error("测试内核版本不匹配或无法执行");
  }
  return { version, hash };
}

if (process.argv[1] && resolve(process.argv[1]) === import.meta.filename) {
  try {
    const binary = process.env.SING_BOX_TEST_BIN;
    const verified = await verifyCore(binary);
    console.log(`真实内核前置检查通过: ${verified.version} / ${verified.hash}`);
    const result = spawnSync(
      "cargo",
      ["test", "--manifest-path", "src-tauri/Cargo.toml", "--locked"],
      {
        stdio: "inherit",
        env: {
          ...process.env,
          SING_BOX_TEST_BIN: binary,
          REQUIRE_SING_BOX_TEST_BIN: "1",
          RUST_TEST_THREADS: "2",
        },
      },
    );
    if (result.error) throw result.error;
    process.exitCode = result.status ?? 1;
  } catch (error) {
    console.error(`真实内核验证失败: ${error.message}`);
    process.exitCode = 1;
  }
}
