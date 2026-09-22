import { useState } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { AppLayout } from "@/components/AppLayout";
import { TooltipProvider } from "@/components/ui/tooltip";
import { proxyModes, type ProxyMode } from "@/lib/proxy-mode";
import ConnectionLogsPage from "@/pages/connection-history";
import ProxiesPage from "@/pages/proxies";
import RoutingRulesPage from "@/pages/rules";
import SettingsPage from "@/pages/settings";
import StatusPage from "@/pages/status";

export default function App() {
  const [mode, setMode] = useState<ProxyMode>("rules");
  const activeMode = proxyModes[mode];

  return (
    <TooltipProvider>
      <BrowserRouter>
        <Routes>
          <Route element={<AppLayout activeModeLabel={activeMode.label} />}>
            <Route
              index
              element={<StatusPage mode={mode} setMode={setMode} activeMode={activeMode} />}
            />
            <Route path="proxies" element={<ProxiesPage />} />
            <Route path="rules" element={<RoutingRulesPage />} />
            <Route path="logs" element={<ConnectionLogsPage />} />
            <Route path="settings" element={<SettingsPage />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </TooltipProvider>
  );
}
