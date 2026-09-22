import { useState } from "react";
import { ArchiveX, CheckCircle2, Download, Info, RefreshCw, RotateCcw, Upload } from "lucide-react";
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
import { Switch } from "@/components/ui/switch";
type SettingsAction = "clear" | "restore" | "export" | "import" | "update" | null;

export default function SettingsPage() {
  const [launchAtLogin, setLaunchAtLogin] = useState(true);
  const [logRetention, setLogRetention] = useState("30");
  const [pendingAction, setPendingAction] = useState<SettingsAction>(null);
  const [logCleared, setLogCleared] = useState(false);

  const actionDetails = {
    clear: {
      title: "清空选择日志",
      description: "确认后将清空当前会话中的日志展示数据。",
      confirmLabel: "确认清空",
      destructive: true,
    },
    restore: {
      title: "恢复全部重置",
      description: "确认后将开机启动和日志保留策略恢复为默认值。",
      confirmLabel: "恢复默认",
      destructive: true,
    },
    export: {
      title: "导出配置",
      description: "这是界面原型，当前不会创建或写入配置文件。",
      confirmLabel: "知道了",
      destructive: false,
    },
    import: {
      title: "导入配置",
      description: "这是界面原型，当前不会读取或修改本地配置文件。",
      confirmLabel: "知道了",
      destructive: false,
    },
    update: {
      title: "检查更新",
      description: "这是界面原型，当前不会发起网络请求检查新版本。",
      confirmLabel: "知道了",
      destructive: false,
    },
  } as const;

  const activeAction = pendingAction ? actionDetails[pendingAction] : null;

  function completeAction() {
    if (pendingAction === "clear") setLogCleared(true);
    if (pendingAction === "restore") {
      setLaunchAtLogin(true);
      setLogRetention("30");
      setLogCleared(false);
    }
    setPendingAction(null);
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
          <div className="grid gap-5 lg:grid-cols-2">
            <Card className="gap-0 border-white/10 bg-card py-0 shadow-none">
              <CardHeader className="py-5">
                <CardTitle role="heading" aria-level={2}>
                  启动
                </CardTitle>
              </CardHeader>
              <CardContent className="flex items-center justify-between gap-5 border-t border-white/10 py-5">
                <div>
                  <p className="font-medium">开机启动</p>
                  <p className="mt-1 text-sm text-muted-foreground">
                    登录系统后自动启动 Socks Proxy
                  </p>
                </div>
                <Switch
                  checked={launchAtLogin}
                  onCheckedChange={setLaunchAtLogin}
                  aria-label="开机启动"
                />
              </CardContent>
            </Card>

            <Card className="gap-0 border-white/10 bg-card py-0 shadow-none">
              <CardHeader className="py-5">
                <CardTitle role="heading" aria-level={2}>
                  配置备份
                </CardTitle>
              </CardHeader>
              <CardContent className="grid grid-cols-2 gap-3 border-t border-white/10 py-5">
                <Button variant="secondary" onClick={() => setPendingAction("export")}>
                  <Download className="size-4" aria-hidden="true" />
                  导出配置
                </Button>
                <Button variant="secondary" onClick={() => setPendingAction("import")}>
                  <Upload className="size-4" aria-hidden="true" />
                  导入配置
                </Button>
              </CardContent>
            </Card>

            <Card className="gap-0 border-white/10 bg-card py-0 shadow-none">
              <CardHeader className="py-5">
                <CardTitle role="heading" aria-level={2}>
                  日志
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4 border-t border-white/10 py-5">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <Button variant="secondary" onClick={() => setPendingAction("clear")}>
                    <ArchiveX className="size-4" aria-hidden="true" />
                    清空选择日志
                  </Button>
                  <span className="text-sm text-muted-foreground">
                    {logCleared ? "日志已清空" : "日志保留策略"}
                  </span>
                </div>
                <Select value={logRetention} onValueChange={setLogRetention}>
                  <SelectTrigger aria-label="日志保留策略">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="7">保留 7 天</SelectItem>
                    <SelectItem value="30">保留 30 天</SelectItem>
                    <SelectItem value="90">保留 90 天</SelectItem>
                    <SelectItem value="forever">永久保留</SelectItem>
                  </SelectContent>
                </Select>
              </CardContent>
            </Card>

            <Card className="gap-0 border-white/10 bg-card py-0 shadow-none">
              <CardHeader className="py-5">
                <CardTitle role="heading" aria-level={2}>
                  网络恢复
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4 border-t border-white/10 py-5">
                <Button variant="secondary" onClick={() => setPendingAction("restore")}>
                  <RotateCcw className="size-4" aria-hidden="true" />
                  恢复全部重置
                </Button>
                <p className="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-muted-foreground">
                  <span>上次恢复：2024-01-20 14:10:32</span>
                  <span className="flex items-center gap-1.5 text-emerald-400">
                    <CheckCircle2 className="size-4" aria-hidden="true" />
                    成功
                  </span>
                </p>
              </CardContent>
            </Card>
          </div>

          <Card className="gap-0 border-white/10 bg-card py-0 shadow-none">
            <CardHeader className="py-5">
              <CardTitle role="heading" aria-level={2}>
                关于
              </CardTitle>
            </CardHeader>
            <CardContent className="grid gap-5 border-t border-white/10 py-5 text-sm sm:grid-cols-[1fr_1fr_auto] sm:items-center">
              <dl className="grid grid-cols-[auto_1fr] gap-x-7 gap-y-3">
                <dt className="text-muted-foreground">版本</dt>
                <dd className="font-semibold">v1.2.0</dd>
                <dt className="text-muted-foreground">内核版本</dt>
                <dd className="font-semibold">1.83</dd>
              </dl>
              <dl className="grid grid-cols-[auto_1fr] gap-x-7 gap-y-3">
                <dt className="text-muted-foreground">许可证</dt>
                <dd className="font-semibold">MIT License</dd>
              </dl>
              <Button variant="secondary" onClick={() => setPendingAction("update")}>
                <RefreshCw className="size-4" aria-hidden="true" />
                检查更新
              </Button>
            </CardContent>
          </Card>
        </div>
      </div>

      <Dialog
        open={pendingAction !== null}
        onOpenChange={(open) => !open && setPendingAction(null)}
      >
        <DialogContent className="max-w-md p-0">
          {activeAction && (
            <>
              <DialogHeader className="border-b border-white/10 px-6 py-5">
                <DialogTitle>{activeAction.title}</DialogTitle>
              </DialogHeader>
              <div className="flex gap-3 px-6 py-5 text-sm text-muted-foreground">
                <Info className="mt-0.5 size-4 shrink-0 text-blue-400" aria-hidden="true" />
                <p>{activeAction.description}</p>
              </div>
              <DialogFooter className="border-t border-white/10 px-6 py-4">
                {activeAction.destructive && (
                  <Button variant="secondary" onClick={() => setPendingAction(null)}>
                    取消
                  </Button>
                )}
                <Button
                  variant={activeAction.destructive ? "destructive" : "default"}
                  onClick={completeAction}
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
