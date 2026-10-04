import { invoke, isTauri } from "@tauri-apps/api/core";
import type { AppError, CommandMap } from "./generated/ipc";

type Arguments<K extends keyof CommandMap> = CommandMap[K][0] extends null
  ? []
  : [arguments_: CommandMap[K][0]];

/** Parameter/result types come from Rust DTOs. Secret commands remain explicit
 * in generated/credentials.ts; never log command arguments or results here. */
export async function ipc<K extends keyof CommandMap>(
  name: K,
  ...arguments_: Arguments<K>
): Promise<CommandMap[K][1]> {
  if (!isTauri()) {
    throw {
      code: "unavailable",
      message: "仅在桌面应用中可使用代理后端。",
      fields: [],
    } satisfies AppError;
  }
  return invoke<CommandMap[K][1]>(name, (arguments_[0] ?? {}) as Record<string, unknown>);
}
