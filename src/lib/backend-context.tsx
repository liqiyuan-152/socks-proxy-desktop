import { useEffect, useState, type ReactNode } from "react";
import { BackendStoreContext, createBackendStore } from "@/store/backend-store";

export function BackendProvider({ children }: { children: ReactNode }) {
  const [store] = useState(createBackendStore);
  useEffect(() => {
    void store.getState().initialize();
    return () => store.getState().dispose();
  }, [store]);
  return <BackendStoreContext.Provider value={store}>{children}</BackendStoreContext.Provider>;
}
