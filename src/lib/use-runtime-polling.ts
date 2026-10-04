import { useEffect } from "react";

export const RUNTIME_POLL_INTERVAL_MS = 1000;

export function useRuntimePolling(refresh: () => Promise<void>) {
  useEffect(() => {
    let disposed = false;
    let inFlight = false;
    let refreshRequested = false;
    let timer: number | undefined;

    function isVisible() {
      return document.visibilityState !== "hidden";
    }

    function schedule(delay = RUNTIME_POLL_INTERVAL_MS) {
      window.clearTimeout(timer);
      if (!disposed && isVisible()) {
        timer = window.setTimeout(() => void poll(), delay);
      }
    }

    async function poll() {
      if (disposed || !isVisible()) return;
      if (inFlight) {
        refreshRequested = true;
        return;
      }
      inFlight = true;
      try {
        await refresh();
      } catch {
        // The data source reports errors; a failed read must not stop polling.
      } finally {
        inFlight = false;
        schedule(refreshRequested ? 0 : RUNTIME_POLL_INTERVAL_MS);
        refreshRequested = false;
      }
    }

    function resume() {
      if (isVisible()) schedule(0);
      else window.clearTimeout(timer);
    }

    schedule();
    window.addEventListener("focus", resume);
    document.addEventListener("visibilitychange", resume);
    return () => {
      disposed = true;
      window.clearTimeout(timer);
      window.removeEventListener("focus", resume);
      document.removeEventListener("visibilitychange", resume);
    };
  }, [refresh]);
}
