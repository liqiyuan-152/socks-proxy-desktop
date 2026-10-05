import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, readdir, copyFile, writeFile } from "node:fs/promises";
import { join, basename, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const releasePlatforms = {
  "windows-x64": { ".exe": "Windows_x64_setup.exe" },
  "macos-arm64": { ".dmg": "macOS_Apple-Silicon.dmg" },
  "macos-intel": { ".dmg": "macOS_Intel.dmg" },
  "macos-universal": { ".dmg": "macOS_Universal.dmg" },
  "linux-x64": {
    ".AppImage": "Linux_x64.AppImage",
    ".deb": null,
    ".rpm": "Linux_x64.rpm",
  },
};
const text = async (path) => (await readFile(path, "utf8")).replace(/^\uFEFF/u, "").trim();

export async function prepareRelease(input, output, tag, revision) {
  assert.ok(input && output && tag && revision, "需要产物目录、输出目录、tag 和源码提交号");
  assert.match(tag, /^v\d+\.\d+\.\d+(?:-[\w.-]+)?$/u);
  assert.match(revision, /^[a-f\d]{40}$/u);
  const version = tag.slice(1);
  const directories = await readdir(input, { withFileTypes: true });
  assert.equal(directories.length, Object.keys(releasePlatforms).length, "缺少目标平台产物");
  const seen = new Set();
  // Validate every platform before creating any uploadable files.
  const groups = await Promise.all(
    directories.map(async (directory) => {
      assert.ok(directory.isDirectory(), "产物必须为独立目录");
      const platform = Object.keys(releasePlatforms).find((name) =>
        directory.name.startsWith(`socks-proxy-${name}-`),
      );
      assert.ok(platform, "未知产物平台");
      assert.ok(!seen.has(platform), "重复目标平台");
      seen.add(platform);
      const expectedFormats = releasePlatforms[platform];
      const root = join(input, directory.name);
      assert.equal(await text(join(root, "SOURCE_REVISION.txt")), revision, "源码提交不匹配");
      assert.equal(await text(join(root, "VERSION.txt")), version, "版本不匹配");
      const checksums = (await text(join(root, "SHA256SUMS.txt"))).split(/\r?\n/u);
      assert.equal(checksums.length, Object.keys(expectedFormats).length, "安装包数量不匹配");
      const formats = new Set();
      const names = new Set();
      const packages = await Promise.all(
        checksums.map(async (line) => {
          const match = /^([a-f\d]{64}) {2}(.+)$/u.exec(line);
          assert.ok(match, "无效的校验清单");
          const [, expected, filename] = match;
          assert.equal(basename(filename), filename, "安装包必须位于产物根目录");
          assert.ok(!filename.includes("\\"), "安装包路径无效");
          assert.ok(!names.has(filename), "重复安装包");
          names.add(filename);
          const extension = Object.keys(expectedFormats).find((suffix) =>
            filename.endsWith(suffix),
          );
          assert.ok(extension, "安装包格式与平台不符");
          assert.ok(!formats.has(extension), "重复安装包格式");
          formats.add(extension);
          const source = join(root, filename);
          assert.equal(
            createHash("sha256")
              .update(await readFile(source))
              .digest("hex"),
            expected,
            "安装包校验失败",
          );
          return {
            source,
            expected,
            name:
              extension === ".deb"
                ? `socks-proxy_${version}_amd64.deb`
                : `Socks-Proxy_v${version}_${expectedFormats[extension]}`,
          };
        }),
      );
      const files = await readdir(root);
      assert.deepEqual(
        files.sort(),
        [...names, "SHA256SUMS.txt", "SOURCE_REVISION.txt", "VERSION.txt"].sort(),
        "产物含未核验文件",
      );
      return packages;
    }),
  );
  const packages = groups.flat();
  await mkdir(output, { recursive: true });
  assert.equal((await readdir(output)).length, 0, "输出目录必须为空，避免混入旧安装包");
  await Promise.all(packages.map((item) => copyFile(item.source, join(output, item.name))));
  await writeFile(
    join(output, "SHA256SUMS.txt"),
    packages
      .map((item) => `${item.expected}  ${item.name}`)
      .sort()
      .join("\n") + "\n",
  );
  await writeFile(join(output, "SOURCE_REVISION.txt"), `${revision}\n`);
  await writeFile(join(output, "VERSION.txt"), `${version}\n`);
  return packages.map((item) => item.name);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const names = await prepareRelease(...process.argv.slice(2));
  console.log(`已核验并准备 ${names.length} 个跨平台安装包`);
}
