import { spawnSync } from "node:child_process";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";

const require = createRequire(import.meta.url);
function run(binary, args, env = process.env) {
  const result = spawnSync(binary, args, { stdio: "inherit", env });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${binary} exited with ${result.status}`);
}

const directory = await mkdtemp(join(tmpdir(), "socks-ipc-"));
try {
  run(
    "cargo",
    ["test", "--manifest-path", "src-tauri/Cargo.toml", "--locked", "generate_ipc_contract"],
    { ...process.env, SOCKS_PROXY_CONTRACT_OUTPUT: directory },
  );
  const files = ["ipc.ts", "credentials.ts"];
  const formatter = join(dirname(require.resolve("oxfmt/package.json")), "bin/oxfmt");
  run(process.execPath, [formatter, ...files.map((file) => join(directory, file))]);
  const destination = resolve("src/lib/generated");
  const check = process.argv.includes("--check");
  if (!check) await mkdir(destination, { recursive: true });
  await Promise.all(
    files.map(async (file) => {
      const generated = await readFile(join(directory, file), "utf8");
      if (check) {
        const existing = await readFile(join(destination, file), "utf8");
        if (existing !== generated)
          throw new Error(`IPC contract drift: ${file}. Run pnpm ipc:generate.`);
      } else {
        await writeFile(join(destination, file), generated);
      }
    }),
  );
  console.log(check ? "IPC 契约未漂移" : "IPC 契约已生成");
} finally {
  await rm(directory, { recursive: true, force: true });
}
