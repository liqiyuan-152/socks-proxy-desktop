import { isTauri } from "@tauri-apps/api/core";
import { ipc } from "@/lib/ipc";

/** 页面已挂载且初始数据可用后，跨两个可见帧确认就绪；不是像素首绘测量。 */
export function scheduleStartupReady(): () => void {
  if (!isTauri()) return () => {};
  let frame: number | undefined;
  let canceled = false;
  let scheduled = false;
  const schedule = () => {
    if (canceled || scheduled || document.visibilityState !== "visible") return;
    scheduled = true;
    frame = requestAnimationFrame(() => {
      frame = requestAnimationFrame(() => {
        if (!canceled && document.visibilityState === "visible") {
          // 指标上报失败不改变用户操作或触发自动业务重试。
          void ipc("acknowledge_frontend_ready").catch(() => {});
        } else {
          scheduled = false;
        }
      });
    });
  };
  document.addEventListener("visibilitychange", schedule);
  schedule();
  return () => {
    canceled = true;
    document.removeEventListener("visibilitychange", schedule);
    if (frame !== undefined) cancelAnimationFrame(frame);
  };
}
