import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { AppLayout } from "@/components/AppLayout";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { BackendProvider } from "@/lib/backend-context";
import ConnectionLogsPage from "@/pages/connection-history";
import ProxiesPage from "@/pages/proxies";
import RoutingRulesPage from "@/pages/rules";
import SettingsPage from "@/pages/settings";
import StatusPage from "@/pages/status";

export default function App() {
  return (
    <TooltipProvider>
      <BackendProvider>
        <BrowserRouter>
          <Toaster />
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
        </BrowserRouter>
      </BackendProvider>
    </TooltipProvider>
  );
}
