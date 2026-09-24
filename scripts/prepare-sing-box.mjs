import { createHash } from "node:crypto";
import { createWriteStream, existsSync } from "node:fs";
import { copyFile, mkdir, mkdtemp, readFile, rename, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";

// This is a Windows-only, pinned GPLv3 dependency. Never ship an unchecked
// download or confuse the release ZIP digest with the executable digest.
const version = "1.14.1";
const archiveSha256 = "5197f16d492d93202dc623622149a6ed040f8eca263128f91d603f2b901baa89";
const executableSha256 = "b838de45bd0b2e6ddbed1977e4745622f7dffab3b293807ff4c6b1b640fed909";
const librarySha256 = "3217c6260fbca5f16072e0b79735742f40109a63bb0ff88fd6b96dd6b54a2928";
const licenseSha256 = "bb3805862b583aee73ad6f7805ec634747a37257a637a3069857843f05ea589c";
const archiveName = `sing-box-${version}-windows-amd64.zip`;
const outputDir = resolve("src-tauri/resources/sing-box/windows-amd64");

async function digest(path) {
  return createHash("sha256")
    .update(await readFile(path))
    .digest("hex");
}

if (process.platform !== "win32" && !process.argv.includes("--target-windows")) {
  console.log("sing-box Windows resource: skipped on this platform");
} else {
  const executable = join(outputDir, "sing-box.exe");
  const library = join(outputDir, "libcronet.dll");
  const license = join(outputDir, "LICENSE");
  if (
    existsSync(executable) &&
    existsSync(library) &&
    existsSync(license) &&
    (await digest(executable)) === executableSha256 &&
    (await digest(library)) === librarySha256 &&
    (await digest(license)) === licenseSha256
  ) {
    console.log(`sing-box ${version}: verified existing Windows resource`);
  } else {
    const temporary = await mkdtemp(join(tmpdir(), "socks-proxy-sing-box-"));
    try {
      const archive = join(temporary, archiveName);
      const source = `https://github.com/SagerNet/sing-box/releases/download/v${version}/${archiveName}`;
      const response = await fetch(source);
      if (!response.ok || !response.body)
        throw new Error(`sing-box download failed: HTTP ${response.status}`);
      await pipeline(Readable.fromWeb(response.body), createWriteStream(archive));
      if ((await digest(archive)) !== archiveSha256)
        throw new Error("sing-box release ZIP SHA-256 mismatch");
      const extraction = spawnSync("tar", ["-xf", archive, "-C", temporary], { stdio: "pipe" });
      if (extraction.status !== 0) throw new Error("cannot extract verified sing-box release ZIP");
      const extracted = join(temporary, `sing-box-${version}-windows-amd64`);
      if ((await digest(join(extracted, "sing-box.exe"))) !== executableSha256) {
        throw new Error("sing-box.exe SHA-256 mismatch");
      }
      if ((await digest(join(extracted, "libcronet.dll"))) !== librarySha256) {
        throw new Error("libcronet.dll SHA-256 mismatch");
      }
      if ((await digest(join(extracted, "LICENSE"))) !== licenseSha256) {
        throw new Error("sing-box LICENSE SHA-256 mismatch");
      }
      // Stage all three verified files in
      // the destination filesystem, then keep the old directory recoverable
      // until the replacement is complete.
      await mkdir(dirname(outputDir), { recursive: true });
      const staged = `${outputDir}.pending`;
      const backup = `${outputDir}.previous`;
      if (existsSync(staged) || existsSync(backup)) {
        throw new Error("previous sing-box resource staging is unresolved");
      }
      await mkdir(staged);
      try {
        await Promise.all(
          ["sing-box.exe", "libcronet.dll", "LICENSE"].map((name) =>
            copyFile(join(extracted, name), join(staged, name)),
          ),
        );
        if (existsSync(outputDir)) await rename(outputDir, backup);
        try {
          await rename(staged, outputDir);
        } catch (error) {
          if (existsSync(backup)) await rename(backup, outputDir);
          throw error;
        }
        if (existsSync(backup)) await rm(backup, { recursive: true });
      } finally {
        if (existsSync(staged)) await rm(staged, { recursive: true });
      }
      console.log(`sing-box ${version}: verified and staged for Windows bundling`);
    } finally {
      await rm(temporary, { recursive: true, force: true });
    }
  }
}
