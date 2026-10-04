import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, readdir, copyFile, writeFile } from "node:fs/promises";
import { join, basename } from "node:path";

const [input, output, tag, revision] = process.argv.slice(2);
assert.ok(input && output && tag && revision, "需要产物目录、输出目录、tag 和源码提交号");
const version = tag.replace(/^v/u, "");
assert.match(tag, /^v\d+\.\d+\.\d+(?:-[\w.-]+)?$/u);
assert.match(revision, /^[a-f\d]{40}$/u);
const directories = await readdir(input, { withFileTypes: true });
assert.equal(directories.length, 2, "需要 Windows 和 macOS 两份产物");
const text = async (path) => (await readFile(path, "utf8")).replace(/^\uFEFF/u, "").trim();
// 两个平台独立核验，全部成功后才生成可上传文件。
const packages = await Promise.all(
  directories.map(async (directory) => {
    assert.ok(directory.isDirectory(), "产物必须为独立目录");
    const root = join(input, directory.name);
    assert.equal(await text(join(root, "SOURCE_REVISION.txt")), revision);
    assert.equal(await text(join(root, "VERSION.txt")), version);
    const checksums = (await text(join(root, "SHA256SUMS.txt"))).split(/\r?\n/u);
    assert.equal(checksums.length, 1, "每个平台需要一个安装包");
    const match = /^([a-f\d]{64}) {2}(.+)$/u.exec(checksums[0]);
    assert.ok(match, "无效的校验清单");
    const [, expected, filename] = match;
    assert.equal(basename(filename), filename, "安装包必须位于产物根目录");
    const platform = directory.name.startsWith("socks-proxy-windows-x64-")
      ? "windows-x64"
      : directory.name.startsWith("socks-proxy-macos-universal-")
        ? "macos-universal"
        : null;
    assert.ok(platform, "未知产物平台");
    const extension = platform === "windows-x64" ? ".exe" : ".dmg";
    assert.ok(filename.endsWith(extension), "安装包格式与平台不符");
    const source = join(root, filename);
    assert.equal(
      createHash("sha256")
        .update(await readFile(source))
        .digest("hex"),
      expected,
    );
    return {
      source,
      expected,
      platform,
      name: `Socks-Proxy_${version}_${platform}${extension}`,
    };
  }),
);
assert.equal(new Set(packages.map((item) => item.platform)).size, 2, "缺少一个目标平台");
await mkdir(output, { recursive: true });
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
console.log(`已核验并准备 ${tag} 的 Windows 和 macOS 安装包`);
