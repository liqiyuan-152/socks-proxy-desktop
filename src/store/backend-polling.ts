export const RUNTIME_POLL_INTERVAL_MS = 1000;

/** 可见窗口使用无重叠轮询；隐藏时暂停，重新可见时立即刷新。 */
export function startRuntimePolling(refresh: () => Promise<void>): () => void {
  let disposed = false;
  let inFlight = false;
  let requested = false;
  let timer: number | undefined;
  const visible = () => document.visibilityState !== "hidden";
  const schedule = (delay = RUNTIME_POLL_INTERVAL_MS) => {
    window.clearTimeout(timer);
    if (!disposed && visible()) timer = window.setTimeout(() => void poll(), delay);
  };
  const poll = async () => {
    if (disposed || !visible()) return;
    if (inFlight) {
      requested = true;
      return;
    }
    inFlight = true;
    try {
      await refresh();
    } catch {
      /* 下一轮继续读取，错误由数据源处理。 */
    } finally {
      inFlight = false;
      schedule(requested ? 0 : RUNTIME_POLL_INTERVAL_MS);
      requested = false;
    }
  };
  const resume = () => {
    if (visible()) schedule(0);
    else window.clearTimeout(timer);
  };
  schedule();
  window.addEventListener("focus", resume);
  document.addEventListener("visibilitychange", resume);
  return () => {
    disposed = true;
    window.clearTimeout(timer);
    window.removeEventListener("focus", resume);
    document.removeEventListener("visibilitychange", resume);
  };
}
