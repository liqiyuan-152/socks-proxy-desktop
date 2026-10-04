import { ipc } from "@/lib/ipc";

export type { LatencyTaskSnapshot as LatencyTask } from "@/lib/generated/ipc";
import type { LatencyTaskSnapshot as LatencyTask } from "@/lib/generated/ipc";

export function createLatencyRequest(id: string, revision: number) {
  let cancelled = false;
  let subscription: string | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let wake: (() => void) | undefined;
  let released = false;
  async function release() {
    if (!subscription || released) return;
    released = true;
    await ipc("release_proxy_latency_task", { subscriptionId: subscription });
  }
  const cancel = () => {
    cancelled = true;
    if (timer) clearTimeout(timer);
    wake?.();
    void release().catch(() => {});
  };
  const result = (async () => {
    try {
      const created = await ipc("start_proxy_latency_task", { id });
      subscription = created.subscription_id;
      async function poll(task: LatencyTask): Promise<{ latency_ms: number } | null> {
        if (cancelled) return null;
        if (task.configuration_revision !== revision || task.profile_id !== id) return null;
        if (task.state === "succeeded") return task.result;
        if (task.state === "failed") throw task.error;
        if (task.state === "cancelled" || task.state === "cancelling") return null;
        await new Promise<void>((resolve) => {
          wake = resolve;
          timer = setTimeout(resolve, 100);
        });
        wake = undefined;
        timer = undefined;
        if (cancelled) return null;
        const next = await ipc("get_proxy_latency_task", {
          subscriptionId: created.subscription_id,
        });
        return poll(next);
      }
      return await poll(created.task);
    } finally {
      await release();
    }
  })();
  return { result, cancel };
}
