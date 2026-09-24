import { useCallback, useEffect, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
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
import { Input } from "@/components/ui/input";
import { command, errorMessage, type BackendError } from "@/lib/backend";
import { useBackend } from "@/lib/backend-context";
import { AboutCard } from "./AboutCard";
import { LatencyTestCard } from "./LatencyTestCard";
import { parseImportProfiles, type ImportProfile } from "./parseImportProfiles";
import { NetworkRecoveryCard } from "./NetworkRecoveryCard";
import { StartupCard } from "./StartupCard";
import type { Retention, Settings } from "./settingsTypes";

type SettingsAction = "clear" | "import" | "restore" | null;
type NetworkRecoveryResult = { completed_at_ms: number };

export default function SettingsPage() {
  const { refresh } = useBackend();
  const [settings, setSettings] = useState<Settings | null>(null);
  const [pendingAction, setPendingAction] = useState<SettingsAction>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [importJson, setImportJson] = useState("");
  const [importProfiles, setImportProfiles] = useState<ImportProfile[]>([]);
  const [newCredentials, setNewCredentials] = useState<
    Record<string, { username: string; password: string }>
  >({});
  const networkRecoveryAvailable = isTauri() && navigator.userAgent.includes("Windows");

  const load = useCallback(async () => {
    try {
      setSettings(await command<Settings>("get_settings"));
      setError(null);
    } catch (reason) {
      setError(errorMessage(reason));
    }
  }, []);
  useEffect(() => {
    const timer = window.setTimeout(() => void load(), 0);
    return () => window.clearTimeout(timer);
  }, [load]);

  async function updateSettings(next: Settings) {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      setSettings(await command<Settings>("update_settings", { settings: next }));
    } catch (reason) {
      const typed = reason as Partial<BackendError>;
      setError(typed.fields?.map((field) => field.message).join("；") || errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  async function exportConfig() {
    setBusy(true);
    setError(null);
    try {
      const json = await command<string>("export_configuration");
      const url = URL.createObjectURL(new Blob([json], { type: "application/json" }));
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = "socks-proxy-config.json";
      anchor.click();
      window.setTimeout(() => URL.revokeObjectURL(url), 0);
      setMessage("已导出不含密码的配置；请妥善保管文件。");
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  async function readImportFile(file: File) {
    try {
      const json = await file.text();
      const data: unknown = JSON.parse(json);
      const profiles = parseImportProfiles(data);
      setImportJson(json);
      setImportProfiles(profiles);
      setNewCredentials({});
      setError(null);
      setPendingAction("import");
    } catch {
      setError("配置文件不是有效的 JSON。");
    }
  }

  async function importConfig() {
    setBusy(true);
    setError(null);
    try {
      const updates: Record<string, { action: "replace"; username: string; password: string }> = {};
      for (const profile of importProfiles.filter((item) => item.authentication_enabled)) {
        const credential = newCredentials[profile.id];
        if (!credential?.password) {
          setError(`请为「${profile.name}」重新输入认证密码。`);
          return;
        }
        updates[profile.id] = { action: "replace", ...credential };
      }
      await command("import_configuration", { json: importJson, updates });
      setPendingAction(null);
      setMessage("配置已导入；密码没有从导出文件恢复。");
      await Promise.all([load(), refresh()]);
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  const actionDetails = {
    clear: {
      title: "清理运行时诊断",
      description: "确认后将删除保存的运行时诊断，不会清理或伪造活跃连接。",
      confirmLabel: "确认清空",
      destructive: true,
    },
    import: {
      title: "导入配置",
      description: "导入会校验并替换配置；已认证档案必须重新提供密码。",
      confirmLabel: "确认导入",
      destructive: false,
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
    if (pendingAction === "import") {
      await importConfig();
      return;
    }
    if (pendingAction === "clear") {
      setBusy(true);
      setError(null);
      try {
        const count = await command<number>("clear_runtime_diagnostics", {
          filter: { from_ms: null, until_ms: null, severity: null },
          confirmed: true,
        });
        setMessage(`已清理 ${count} 条运行时诊断。`);
        setPendingAction(null);
      } catch (reason) {
        setError(errorMessage(reason));
      } finally {
        setBusy(false);
      }
    }
    if (pendingAction === "restore") {
      setBusy(true);
      setError(null);
      try {
        const result = await command<NetworkRecoveryResult>("recover_network", { confirmed: true });
        setMessage(
          `网络恢复检查完成：${new Date(result.completed_at_ms).toLocaleString()}。仅处理本应用可确认拥有的设置。`,
        );
        setPendingAction(null);
        await refresh();
      } catch (reason) {
        setError(errorMessage(reason));
      } finally {
        setBusy(false);
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
          {error && !pendingAction && (
            <p role="alert" className="text-destructive">
              {error}
            </p>
          )}
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
                busy={busy}
                onSave={(url) => void updateSettings({ ...settings, latency_test_url: url })}
              />
            )}
            <StartupCard
              enabled={settings?.launch_at_login ?? false}
              loaded={!!settings}
              busy={busy}
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
                  disabled={busy || !settings}
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
                    disabled={busy}
                    onChange={(event) => {
                      const file = event.target.files?.[0];
                      if (file) void readImportFile(file);
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
                    disabled={busy}
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
                  disabled={!settings || busy}
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
              available={networkRecoveryAvailable}
              busy={busy}
              onRestore={() => {
                setError(null);
                setPendingAction("restore");
              }}
            />
          </div>

          <AboutCard />
        </div>
      </div>

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
              {error && (
                <p role="alert" className="px-6 pb-4 text-sm text-destructive">
                  {error}
                </p>
              )}
              {pendingAction === "import" &&
                importProfiles
                  .filter((profile) => profile.authentication_enabled)
                  .map((profile) => (
                    <div key={profile.id} className="space-y-2 px-6 pb-4">
                      <p className="font-medium">{profile.name} 的认证凭据</p>
                      <Input
                        aria-label={`${profile.name}用户名`}
                        placeholder="用户名"
                        autoComplete="off"
                        onChange={(event) =>
                          setNewCredentials((current) => ({
                            ...current,
                            [profile.id]: {
                              username: event.target.value,
                              password: current[profile.id]?.password ?? "",
                            },
                          }))
                        }
                      />
                      <Input
                        aria-label={`${profile.name}密码`}
                        placeholder="重新输入密码"
                        type="password"
                        autoComplete="new-password"
                        onChange={(event) =>
                          setNewCredentials((current) => ({
                            ...current,
                            [profile.id]: {
                              username: current[profile.id]?.username ?? "",
                              password: event.target.value,
                            },
                          }))
                        }
                      />
                    </div>
                  ))}
              <DialogFooter className="border-t border-border px-6 py-4">
                {activeAction.destructive && (
                  <Button variant="secondary" onClick={() => setPendingAction(null)}>
                    取消
                  </Button>
                )}
                <Button
                  variant={activeAction.destructive ? "destructive" : "default"}
                  disabled={busy}
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
