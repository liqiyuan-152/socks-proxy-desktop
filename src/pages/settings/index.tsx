import { DiagnosticsCard } from "./DiagnosticsCard";
import { ErrorAlert } from "@/components/ErrorAlert";
import { normalizeError } from "@/lib/error-handler";
import type { AppError } from "@/lib/generated/ipc";
import { ipc } from "@/lib/ipc";
import { useCallback, useEffect, useRef, useState } from "react";
import { ArchiveX, Download, Info, Upload } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useBackendStore } from "@/store/backend-store";
import { useShallow } from "zustand/react/shallow";
import { AboutCard } from "./AboutCard";
import { LatencyTestCard } from "./LatencyTestCard";
import { useConfigurationImport } from "./useConfigurationImport";
import { ConfigurationImportDialog } from "./ConfigurationImportDialog";
import { NetworkRecoveryCard } from "./NetworkRecoveryCard";
import { StartupCard } from "./StartupCard";
import type { Retention, Settings } from "./settingsTypes";

type SettingsAction = "clear" | "restore" | null;

export default function SettingsPage() {
  const { refresh, capabilities, persistSettings, importConfiguration, recoverNetwork } =
    useBackendStore(
      useShallow((state) => ({
        refresh: state.refresh,
        capabilities: state.capabilities,
        persistSettings: state.updateSettings,
        importConfiguration: state.importConfiguration,
        recoverNetwork: state.recoverNetwork,
      })),
    );
  const active = useRef(false);
  const exportFlight = useRef(false);
  const settingsRequest = useRef(0);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [pendingAction, setPendingAction] = useState<SettingsAction>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const load = useCallback(async () => {
    const version = ++settingsRequest.current;
    try {
      const next = await ipc("get_settings");
      if (!active.current || version !== settingsRequest.current) return;
      setSettings(next);
      setError(null);
    } catch (reason) {
      if (active.current && version === settingsRequest.current) setError(normalizeError(reason));
    }
  }, []);
  useEffect(() => {
    active.current = true;
    const timer = window.setTimeout(() => void load(), 0);
    return () => {
      active.current = false;
      settingsRequest.current += 1;
      window.clearTimeout(timer);
    };
  }, [load]);

  async function updateSettings(next: Settings) {
    setBusy(true);
    setError(null);
    setMessage(null);
    const version = ++settingsRequest.current;
    try {
      const updated = await persistSettings(next);
      if (active.current && version === settingsRequest.current) setSettings(updated);
    } catch (reason) {
      if (!active.current || version !== settingsRequest.current) return;
      setError(normalizeError(reason));
    } finally {
      if (active.current) setBusy(false);
    }
  }

  async function exportConfig() {
    if (exportFlight.current) return;
    exportFlight.current = true;
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const saved = await ipc("save_configuration");
      if (active.current && saved) setMessage("已保存不含密码的配置；请妥善保管文件。");
    } catch (reason) {
      if (active.current) setError(normalizeError(reason));
    } finally {
      exportFlight.current = false;
      if (active.current) setBusy(false);
    }
  }

  const importFlow = useConfigurationImport(async () => {
    setMessage("配置已导入；密码没有从导出文件恢复。");
    await load();
  }, importConfiguration);
  const actionBusy = busy || importFlow.busy;

  const actionDetails = {
    clear: {
      title: "清理运行时诊断",
      description: "确认后将删除保存的运行时诊断，不会清理或伪造活跃连接。",
      confirmLabel: "确认清空",
      destructive: true,
    },
    restore: {
      title: "确认恢复网络设置",
      description:
        "将停止本应用的代理内核，仅恢复仍可确认由本应用管理的 Windows 系统代理设置；外部修改不会被覆盖。",
      confirmLabel: "确认恢复",
      destructive: true,
    },
  } as const;

  const activeAction = pendingAction ? actionDetails[pendingAction] : null;

  async function completeAction() {
    if (pendingAction === "clear") {
      setBusy(true);
      setError(null);
      try {
        const count = await ipc("clear_runtime_diagnostics", {
          filter: { from_ms: null, until_ms: null, severity: null },
          confirmed: true,
        });
        setMessage(`已清理 ${count} 条运行时诊断。`);
        setPendingAction(null);
      } catch (reason) {
        setError(normalizeError(reason));
      } finally {
        if (active.current) setBusy(false);
      }
    }
    if (pendingAction === "restore") {
      setBusy(true);
      setError(null);
      try {
        const result = await recoverNetwork();
        setMessage(
          `网络恢复检查完成：${new Date(result.completed_at_ms).toLocaleString()}。仅处理本应用可确认拥有的设置。`,
        );
        setPendingAction(null);
        await refresh();
      } catch (reason) {
        setError(normalizeError(reason));
      } finally {
        if (active.current) setBusy(false);
      }
    }
  }

  return (
    <>
      <header className="border-b border-sidebar-border bg-sidebar px-5 py-4 text-sidebar-foreground sm:px-6">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">设置</h1>
          <p className="mt-1 text-sm text-muted-foreground">管理应用启动、日志和配置选项。</p>
        </div>
      </header>

      <div className="content-scroll min-h-0 flex-1 overflow-y-auto px-5 py-5 sm:px-6">
        <div className="w-full space-y-5">
          {error && !pendingAction && <ErrorAlert error={error} />}
          {message && (
            <p role="status" className="text-muted-foreground">
              {message}
            </p>
          )}
          {!settings && !error && <p role="status">正在加载设置…</p>}
          <div className="grid gap-5 lg:grid-cols-2">
            {settings && (
              <LatencyTestCard
                key={settings.latency_test_url}
                url={settings.latency_test_url}
                busy={actionBusy}
                onSave={(url) => void updateSettings({ ...settings, latency_test_url: url })}
              />
            )}
            <StartupCard
              enabled={settings?.launch_at_login ?? false}
              loaded={!!settings}
              available={!!capabilities?.startup}
              busy={actionBusy}
              onChange={(enabled) =>
                settings && void updateSettings({ ...settings, launch_at_login: enabled })
              }
            />

            <Card className="gap-0 border-border bg-card py-0 shadow-none">
              <CardHeader className="py-5">
                <CardTitle role="heading" aria-level={2}>
                  配置备份
                </CardTitle>
              </CardHeader>
              <CardContent className="grid grid-cols-2 gap-3 border-t border-border py-5">
                <Button
                  variant="secondary"
                  onClick={() => void exportConfig()}
                  disabled={actionBusy || !settings}
                >
                  <Download className="size-4" aria-hidden="true" />
                  导出配置
                </Button>
                <label className="inline-flex cursor-pointer items-center justify-center gap-2 rounded-md border border-input bg-secondary px-3 py-2 text-sm font-medium text-secondary-foreground">
                  <Upload className="size-4" aria-hidden="true" />
                  导入配置
                  <input
                    className="sr-only"
                    type="file"
                    accept="application/json,.json"
                    aria-label="选择配置文件"
                    disabled={actionBusy}
                    onChange={(event) => {
                      const file = event.target.files?.[0];
                      event.target.value = "";
                      if (file) void importFlow.read(file);
                    }}
                  />
                </label>
              </CardContent>
            </Card>

            <Card className="gap-0 border-border bg-card py-0 shadow-none">
              <CardHeader className="py-5">
                <CardTitle role="heading" aria-level={2}>
                  日志
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4 border-t border-border py-5">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <Button
                    variant="secondary"
                    onClick={() => setPendingAction("clear")}
                    disabled={actionBusy}
                  >
                    <ArchiveX className="size-4" aria-hidden="true" />
                    清理运行时诊断
                  </Button>
                  <span className="text-sm text-muted-foreground">诊断保留策略</span>
                </div>
                <Select
                  value={settings?.diagnostic_retention ?? "days30"}
                  onValueChange={(value) =>
                    settings &&
                    void updateSettings({ ...settings, diagnostic_retention: value as Retention })
                  }
                  disabled={!settings || actionBusy}
                >
                  <SelectTrigger aria-label="日志保留策略">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="days7">保留 7 天</SelectItem>
                    <SelectItem value="days30">保留 30 天</SelectItem>
                    <SelectItem value="days90">保留 90 天</SelectItem>
                    <SelectItem value="permanent">永久保留（最多 100,000 条）</SelectItem>
                  </SelectContent>
                </Select>
              </CardContent>
            </Card>

            <NetworkRecoveryCard
              available={!!capabilities?.network_recovery}
              busy={actionBusy}
              onRestore={() => {
                setError(null);
                setPendingAction("restore");
              }}
            />
          </div>

          <DiagnosticsCard />
          <AboutCard />
        </div>
      </div>

      <ConfigurationImportDialog flow={importFlow} />
      {importFlow.error && !importFlow.open && <ErrorAlert error={importFlow.error} />}
      <Dialog
        open={pendingAction !== null}
        onOpenChange={(open) => !open && setPendingAction(null)}
      >
        <DialogContent className="max-w-md p-0">
          {activeAction && (
            <>
              <DialogHeader className="border-b border-border px-6 py-5">
                <DialogTitle>{activeAction.title}</DialogTitle>
              </DialogHeader>
              <div className="flex gap-3 px-6 py-5 text-sm text-muted-foreground">
                <Info className="mt-0.5 size-4 shrink-0 text-primary" aria-hidden="true" />
                <p>{activeAction.description}</p>
              </div>
              {error && <ErrorAlert error={error} />}
              <DialogFooter className="border-t border-border px-6 py-4">
                {activeAction.destructive && (
                  <Button variant="secondary" onClick={() => setPendingAction(null)}>
                    取消
                  </Button>
                )}
                <Button
                  variant={activeAction.destructive ? "destructive" : "default"}
                  disabled={actionBusy}
                  onClick={() => void completeAction()}
                >
                  {activeAction.confirmLabel}
                </Button>
              </DialogFooter>
            </>
          )}
        </DialogContent>
      </Dialog>
    </>
  );
}
