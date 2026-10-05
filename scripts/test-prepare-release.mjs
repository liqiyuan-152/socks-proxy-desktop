import assert from "node:assert/strict";
import test from "node:test";
import { createHash } from "node:crypto";
import { mkdtemp, mkdir, writeFile, readFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { prepareRelease, releasePlatforms } from "./prepare-release.mjs";

const revision = "a".repeat(40);
async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), "socks-release-test-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const input = join(root, "input");
  const output = join(root, "output");
  await mkdir(input);
  const dirs = await Promise.all(
    Object.entries(releasePlatforms).map(async ([platform, formats]) => {
      const path = join(input, `socks-proxy-${platform}-v0.2.2-${revision}`);
      await mkdir(path);
      const sums = await Promise.all(
        Object.keys(formats).map(async (format) => {
          const name = `installer${format}`;
          const bytes = Buffer.from(`test-${platform}-${format}`);
          await writeFile(join(path, name), bytes);
          return `${createHash("sha256").update(bytes).digest("hex")}  ${name}`;
        }),
      );
      await Promise.all([
        writeFile(join(path, "SHA256SUMS.txt"), sums.join("\n") + "\n"),
        writeFile(join(path, "VERSION.txt"), "\uFEFF0.2.2\r\n"),
        writeFile(join(path, "SOURCE_REVISION.txt"), revision + "\n"),
      ]);
      return path;
    }),
  );
  return { input, output, dirs };
}

test("完整平台矩阵生成七个具名产物并校验汇总 SHA256", async (t) => {
  const { input, output } = await fixture(t);
  const names = await prepareRelease(input, output, "v0.2.2", revision);
  assert.equal(names.length, 7);
  assert.ok(names.includes("Socks-Proxy_v0.2.2_Windows_x64_setup.exe"));
  assert.ok(names.includes("socks-proxy_0.2.2_amd64.deb"));
  const lines = (await readFile(join(output, "SHA256SUMS.txt"), "utf8")).trim().split("\n");
  await Promise.all(
    lines.map(async (line) => {
      const [sum, name] = line.split("  ");
      assert.equal(
        createHash("sha256")
          .update(await readFile(join(output, name)))
          .digest("hex"),
        sum,
      );
    }),
  );
});

for (const [name, mutate, expected] of [
  ["缺少平台", (dirs) => rm(dirs[0], { recursive: true }), /缺少目标平台/u],
  [
    "错误源码",
    (dirs) => writeFile(join(dirs[0], "SOURCE_REVISION.txt"), "b".repeat(40)),
    /源码提交不匹配/u,
  ],
  ["错误版本", (dirs) => writeFile(join(dirs[0], "VERSION.txt"), "0.1.0"), /版本不匹配/u],
  [
    "篡改安装包",
    (dirs) => writeFile(join(dirs[0], "installer.exe"), "tampered"),
    /安装包校验失败/u,
  ],
  ["未核验文件", (dirs) => writeFile(join(dirs[0], "unverified.exe"), "extra"), /未核验文件/u],
  [
    "错误格式",
    (dirs) => writeFile(join(dirs[0], "SHA256SUMS.txt"), `${"a".repeat(64)}  installer.msi`),
    /格式与平台不符/u,
  ],
  [
    "路径穿越",
    (dirs) => writeFile(join(dirs[0], "SHA256SUMS.txt"), `${"a".repeat(64)}  ../installer.exe`),
    /产物根目录/u,
  ],
]) {
  test(`${name}拒绝发布`, async (t) => {
    const { input, output, dirs } = await fixture(t);
    await mutate(dirs);
    await assert.rejects(prepareRelease(input, output, "v0.2.2", revision), expected);
  });
}

test("已有输出文件拒绝混合发布", async (t) => {
  const { input, output } = await fixture(t);
  await mkdir(output);
  await writeFile(join(output, "old.exe"), "old");
  await assert.rejects(prepareRelease(input, output, "v0.2.2", revision), /输出目录必须为空/u);
});
