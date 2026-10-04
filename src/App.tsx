import { lazy, Suspense } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { AppLayout } from "@/components/AppLayout";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { BackendProvider } from "@/lib/backend-context";
const ConnectionLogsPage = lazy(() => import("@/pages/connection-history"));
const ProxiesPage = lazy(() => import("@/pages/proxies"));
const RoutingRulesPage = lazy(() => import("@/pages/rules"));
const SettingsPage = lazy(() => import("@/pages/settings"));
const StatusPage = lazy(() => import("@/pages/status"));

export default function App() {
  return (
    <TooltipProvider>
      <BackendProvider>
        <BrowserRouter>
          <Toaster />
          <Suspense fallback={<p role="status">正在加载页面…</p>}>
            <Routes>
              <Route element={<AppLayout />}>
                <Route index element={<StatusPage />} />
                <Route path="proxies" element={<ProxiesPage />} />
                <Route path="rules" element={<RoutingRulesPage />} />
                <Route path="logs" element={<ConnectionLogsPage />} />
                <Route path="settings" element={<SettingsPage />} />
                <Route path="*" element={<Navigate to="/" replace />} />
              </Route>
            </Routes>
          </Suspense>
        </BrowserRouter>
      </BackendProvider>
    </TooltipProvider>
  );
}
