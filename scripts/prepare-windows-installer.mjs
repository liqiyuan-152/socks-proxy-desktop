import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { pathToFileURL } from "node:url";

// Keep the official template, with three bounded changes for data retention.
// Generated NSIS source stays in target/; an upstream change fails closed.
const revision = "9452ddee5ebefd9b678a94ff003521379df6c9ae";
const sha256 = "20f4ecc730defb71f1342eaeaec4021df13be3d843abba0effe88ea5835fa079";
const sourceUrl = `https://raw.githubusercontent.com/tauri-apps/tauri/${revision}/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi`;
const output = resolve("src-tauri/target/branding-installer.nsi");
const cached = resolve("src-tauri/target/branding-installer-upstream.nsi");

function replaceOnce(source, pattern, replacement) {
  const matches = [...source.matchAll(new RegExp(pattern.source, "g"))];
  if (matches.length !== 1) throw new Error("Unexpected official NSIS template structure");
  return source.replace(pattern, replacement);
}

export function retainInstallerData(source) {
  let result = replaceOnce(
    source,
    /      \$\{IfThen\} \$UpdateMode = 1 \$\{\|\} StrCpy \$R1 "\$R1 \/UPDATE" \$\{\|\} ; append \/UPDATE/,
    '      StrCpy $R1 "$R1 /UPDATE" ; Replacement always preserves existing app data',
  );
  result = replaceOnce(
    result,
    /Var DeleteAppDataCheckbox\n[\s\S]*?(?=!define MUI_PAGE_CUSTOMFUNCTION_PRE un\.SkipIfPassive)/,
    '!define MUI_UNCONFIRMPAGE_TEXT_TOP "卸载仅移除应用程序和快捷方式，代理配置和应用数据会保留。"\n',
  );
  result = replaceOnce(
    result,
    /  ; Delete app data if the checkbox is selected\n[\s\S]*?(?=  !ifmacrodef NSIS_HOOK_POSTUNINSTALL)/,
    "  ; Application data is retained on every uninstall.\n\n",
  );
  if (/DeleteAppDataCheckbox|RmDir\s+\/r\s+"\$(?:LOCAL)?APPDATA/i.test(result)) {
    throw new Error("Generated installer can still delete application data");
  }
  return result;
}

export async function prepareWindowsInstaller() {
  const cli = JSON.parse(await readFile("node_modules/@tauri-apps/cli/package.json", "utf8"));
  if (cli.version !== "2.11.5") throw new Error("Review NSIS template before upgrading Tauri CLI");
  let bytes;
  try {
    bytes = await readFile(cached);
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
    const response = await fetch(sourceUrl);
    if (!response.ok)
      throw new Error(`NSIS template download failed: HTTP ${response.status}`, { cause: error });
    bytes = Buffer.from(await response.arrayBuffer());
  }
  if (createHash("sha256").update(bytes).digest("hex") !== sha256) {
    throw new Error("Official NSIS template SHA-256 mismatch");
  }
  const template = retainInstallerData(bytes.toString("utf8"));
  await mkdir(dirname(output), { recursive: true });
  await writeFile(cached, bytes);
  await writeFile(output, template);
  console.log("Windows installer: verified official template with enforced data retention");
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  if (process.platform === "win32" || process.argv.includes("--target-windows")) {
    await prepareWindowsInstaller();
  } else {
    console.log("Windows installer template: skipped on this platform");
  }
}
