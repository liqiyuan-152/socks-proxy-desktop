import { useEffect, useMemo, type ReactNode } from "react";
import { onRuntimeSnapshot } from "@/lib/backend";
import { BackendContext } from "@/lib/backend-state";
import { useBackendObservation } from "@/lib/use-backend-observation";
import { useModeSwitching } from "@/lib/use-mode-switching";

export function BackendProvider({ children }: { children: ReactNode }) {
  const observation = useBackendObservation();
  const { applySnapshot, refreshConnections, reportError } = observation;
  const { pending, selectedMode, operationError, switchMode, observeSnapshot } =
    useModeSwitching(observation);

  useEffect(() => {
    if (observation.snapshot) observeSnapshot(observation.snapshot);
  }, [observation.snapshot, observeSnapshot]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void onRuntimeSnapshot((next) => {
      if (disposed) return;
      applySnapshot(next);
      observeSnapshot(next);
      void refreshConnections();
    })
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch((reason: unknown) => {
        if (!disposed) reportError(reason);
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [applySnapshot, observeSnapshot, refreshConnections, reportError]);

  const { capabilities, snapshot, profiles, connections, loading, refresh } = observation;
  const error = operationError ?? observation.error;
  const value = useMemo(
    () => ({
      capabilities,
      snapshot,
      profiles,
      connections,
      loading,
      error,
      refresh,
      pending,
      selectedMode,
      switchMode,
    }),
    [
      capabilities,
      snapshot,
      profiles,
      connections,
      loading,
      error,
      refresh,
      pending,
      selectedMode,
      switchMode,
    ],
  );
  return <BackendContext.Provider value={value}>{children}</BackendContext.Provider>;
}
