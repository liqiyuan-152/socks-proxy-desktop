import { execFileSync } from "node:child_process";
import {
  mkdtempSync,
  readFileSync,
  writeFileSync,
  copyFileSync,
  readdirSync,
  rmSync,
  mkdirSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

// Tauri renders the original SVGs; package those renders at their intended sizes.
const root = new URL("../", import.meta.url);
const icons = new URL("src-tauri/icons/", root);
const scratch = mkdtempSync(join(tmpdir(), "socks-brand-"));
const pngs = new Map();
const require = createRequire(import.meta.url);
const cli = join(require.resolve("@tauri-apps/cli/package.json"), "..", "tauri.js");
const run = (args) =>
  execFileSync(process.execPath, [cli, "icon", ...args], { cwd: root, stdio: "inherit" });
try {
  const defaults = join(scratch, "default");
  run([fileURLToPath(new URL("source/full.svg", icons)), "--output", defaults]);
  for (const name of readdirSync(defaults)) {
    if (name.startsWith("Square") || name === "StoreLogo.png") {
      copyFileSync(join(defaults, name), new URL(name, icons));
    }
  }
  for (const [variant, sizes] of [
    ["minimal", [16, 24]],
    ["small", [30, 32, 44, 48, 50, 64]],
    ["standard", [128, 256]],
    ["full", [512, 1024]],
  ]) {
    const output = join(scratch, variant);
    run([
      fileURLToPath(new URL(`source/${variant}.svg`, icons)),
      "--output",
      output,
      ...sizes.flatMap((size) => ["--png", String(size)]),
    ]);
    for (const size of sizes) {
      const data = readFileSync(join(output, `${size}x${size}.png`));
      pngs.set(size, data);
      writeFileSync(new URL(`${size}x${size}.png`, icons), data);
    }
  }
  for (const [name, size] of [
    ["Square30x30Logo.png", 30],
    ["Square44x44Logo.png", 44],
    ["StoreLogo.png", 50],
  ]) {
    writeFileSync(new URL(name, icons), pngs.get(size));
  }
  for (const [name, size] of [
    ["icon.png", 1024],
    ["128x128@2x.png", 256],
    ["32x32@2x.png", 64],
    ["512x512@2x.png", 1024],
  ]) {
    writeFileSync(new URL(name, icons), pngs.get(size));
  }
  // PNG-compressed ICO entries are supported by Windows Vista and later.
  const sizes = [16, 24, 32, 48, 64, 128, 256];
  const directory = Buffer.alloc(6 + sizes.length * 16);
  directory.writeUInt16LE(1, 2);
  directory.writeUInt16LE(sizes.length, 4);
  let offset = directory.length;
  sizes.forEach((size, index) => {
    const start = 6 + index * 16;
    directory[start] = size === 256 ? 0 : size;
    directory[start + 1] = directory[start];
    directory.writeUInt16LE(1, start + 4);
    directory.writeUInt16LE(32, start + 6);
    directory.writeUInt32LE(pngs.get(size).length, start + 8);
    directory.writeUInt32LE(offset, start + 12);
    offset += pngs.get(size).length;
  });
  writeFileSync(
    new URL("icon.ico", icons),
    Buffer.concat([directory, ...sizes.map((size) => pngs.get(size))]),
  );
  const chunks = [
    ["icp4", 16],
    ["icp5", 32],
    ["icp6", 64],
    ["ic07", 128],
    ["ic08", 256],
    ["ic09", 512],
    ["ic10", 1024],
    ["ic11", 32],
    ["ic12", 64],
    ["ic13", 256],
    ["ic14", 512],
  ].map(([type, size]) => {
    const header = Buffer.alloc(8);
    header.write(type);
    header.writeUInt32BE(pngs.get(size).length + 8, 4);
    return Buffer.concat([header, pngs.get(size)]);
  });
  const header = Buffer.alloc(8);
  header.write("icns");
  header.writeUInt32BE(8 + chunks.reduce((sum, chunk) => sum + chunk.length, 0), 4);
  writeFileSync(new URL("icon.icns", icons), Buffer.concat([header, ...chunks]));
  const template = join(scratch, "template");
  run([fileURLToPath(new URL("source/template.svg", icons)), "--output", template, "--png", "32"]);
  copyFileSync(join(template, "32x32.png"), new URL("icon-template.png", icons));
  mkdirSync(new URL("public/", root), { recursive: true });
  copyFileSync(new URL("icon.ico", icons), new URL("public/favicon.ico", root));
} finally {
  rmSync(scratch, { recursive: true, force: true });
}
