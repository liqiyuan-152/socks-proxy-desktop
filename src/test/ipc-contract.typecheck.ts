import { ipc } from "@/lib/ipc";
import type { RuntimeSnapshot } from "@/lib/generated/ipc";

// Compiled by pnpm typecheck, never executed. Removing command/argument/result
// protection makes the corresponding @ts-expect-error directive fail CI.
function assertContract() {
  const snapshot: Promise<RuntimeSnapshot> = ipc("get_runtime_snapshot");
  void snapshot;
  // @ts-expect-error Unknown command names are rejected.
  void ipc("unknown_command");
  // @ts-expect-error A route prediction requires target and port.
  void ipc("test_route", { target: "example.com" });
  // @ts-expect-error Port is a JSON number.
  void ipc("test_route", { target: "example.com", port: "443" });
  // @ts-expect-error Argument names follow Tauri camelCase.
  void ipc("get_proxy_latency_task", { subscription_id: "x" });
  // @ts-expect-error Command results cannot be chosen by the caller.
  const wrong: Promise<string> = ipc("get_runtime_snapshot");
  void wrong;
}
void assertContract;
