import { useEffect } from "react";
import { ChevronRight, CircleDot } from "lucide-react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useBackend } from "@/lib/backend-context";
import { proxyModes, type ProxyMode } from "@/lib/proxy-mode";

const phaseLabels = {
  stopped: "未运行",
  starting: "启动中",
  running: "运行中",
  switching: "切换中",
  recovering: "恢复中",
  failed: "异常",
} as const;
const modeSwitchToastId = "proxy-mode-switch";

export default function StatusDashboard() {
  const { snapshot, profiles, connections, loading, pending, selectedMode, error, switchMode } =
    useBackend();
  const navigate = useNavigate();
  const profile = profiles.find((item) => item.id === snapshot?.active_profile_id);

  useEffect(() => {
    if (!pending) return;
    toast.loading("正在切换代理模式…", { id: modeSwitchToastId });
    return () => {
      toast.dismiss(modeSwitchToastId);
    };
  }, [pending]);

  function selectMode(mode: ProxyMode) {
    if (mode === "global" && !profile) {
      const noProfiles = profiles.length === 0;
      toast.error(
        noProfiles
          ? "尚未添加代理，请先添加代理后再切换代理模式。"
          : "尚未设置默认代理，请先在代理列表中设置。",
        {
          id: "missing-active-proxy",
          action: {
            label: noProfiles ? "去添加" : "去选择",
            onClick: () => navigate("/proxies"),
          },
        },
      );
      return;
    }
    toast.dismiss("missing-active-proxy");
    void switchMode(mode);
  }

  return (
    <>
      <header className="border-b border-sidebar-border bg-sidebar px-5 py-4 text-sidebar-foreground sm:px-6">
        <h1 className="text-2xl font-semibold tracking-tight">状态</h1>
        <p className="mt-1 text-sm text-muted-foreground">当前网络模式和实际运行状态</p>
      </header>
      <div className="content-scroll min-h-0 flex-1 overflow-y-auto px-5 py-5 sm:px-6">
        <div className="w-full space-y-6">
          {loading && <p role="status">正在加载运行时状态…</p>}
          {error && (
            <p role="alert" className="text-destructive">
              {error}
            </p>
          )}
          <Tabs
            value={selectedMode ?? snapshot?.selected_mode ?? ""}
            onValueChange={(mode) => selectMode(mode as ProxyMode)}
          >
            <TabsList className="grid h-11 w-full grid-cols-3 overflow-hidden rounded-lg border border-border bg-muted/50 p-0 sm:mx-auto sm:max-w-2xl">
              {(Object.keys(proxyModes) as ProxyMode[]).map((mode) => (
                <TabsTrigger
                  key={mode}
                  value={mode}
                  className="h-full rounded-none border-0 border-r border-border bg-transparent text-xs text-muted-foreground last:border-r-0 hover:bg-muted hover:text-foreground data-[state=active]:!border-transparent data-[state=active]:!bg-blue-600 data-[state=active]:!text-white data-[state=active]:shadow-[inset_0_1px_0_rgb(255_255_255_/_0.18)] sm:text-sm"
                  disabled={loading || !snapshot}
                  onClick={() => {
                    if (mode === selectedMode && !pending && snapshot?.applied_mode !== mode) {
                      selectMode(mode);
                    }
                  }}
                >
                  {proxyModes[mode].label}
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>
          <div className="grid gap-5 lg:grid-cols-2">
            <Card className="gap-0 border-border bg-card py-0 shadow-none">
              <CardHeader className="border-b border-border py-5">
                <CardTitle role="heading" aria-level={2}>
                  默认代理
                </CardTitle>
              </CardHeader>
              <CardContent className="grid gap-y-4 py-5 text-sm sm:grid-cols-[132px_1fr]">
                <span className="text-muted-foreground">代理名称</span>
                <span>{profile?.name ?? "未选择"}</span>
                <span className="text-muted-foreground">协议</span>
                <span>{profile?.protocol.toUpperCase() ?? "不可用"}</span>
                <span className="text-muted-foreground">服务器</span>
                <span>{profile?.host ?? "不可用"}</span>
                <span className="text-muted-foreground">端口</span>
                <span>{profile?.port ?? "不可用"}</span>
                <span className="text-muted-foreground">认证状态</span>
                <span>
                  {profile ? (profile.authentication_enabled ? "已配置" : "未启用") : "不可用"}
                </span>
              </CardContent>
            </Card>
            <Card className="gap-0 border-border bg-card py-0 shadow-none">
              <CardHeader className="border-b border-border py-5">
                <CardTitle role="heading" aria-level={2}>
                  内核状态
                </CardTitle>
              </CardHeader>
              <CardContent className="grid gap-y-4 py-5 text-sm sm:grid-cols-[132px_1fr]">
                <span className="text-muted-foreground">运行阶段</span>
                <span>{snapshot ? phaseLabels[snapshot.phase] : "不可用"}</span>
                <span className="text-muted-foreground">已应用模式</span>
                <span>
                  {snapshot?.applied_mode ? proxyModes[snapshot.applied_mode].label : "未应用"}
                </span>
                <span className="text-muted-foreground">运行时长</span>
                <span>
                  {snapshot?.runtime_uptime_ms == null
                    ? "不可用"
                    : `${Math.floor(snapshot.runtime_uptime_ms / 1000)} 秒`}
                </span>
                <span className="text-muted-foreground">TUN 状态</span>
                <span>{snapshot?.tun_enabled ? "已启用" : "未启用"}</span>
                <span className="text-muted-foreground">覆盖范围</span>
                <span>
                  {snapshot?.coverage === "system_proxy_apps"
                    ? "仅遵循 Windows 系统代理设置的应用流量"
                    : "未接管系统代理"}
                </span>
              </CardContent>
            </Card>
          </div>

          <section aria-labelledby="statistics-heading">
            <h2 id="statistics-heading" className="mb-3 text-base font-semibold">
              连接统计
            </h2>
            <Card className="grid gap-0 border-border bg-card py-0 shadow-none sm:grid-cols-4">
              <Statistic
                label="当前活跃连接"
                value={connections?.active_count?.toString() ?? "不可用"}
              />
              <Statistic label="已完成连接" value="不可用" />
              <Statistic label="成功率" value="不可用" />
              <Statistic label="失败详情" value="不可用" />
            </Card>
            {connections?.status === "degraded" && (
              <p role="status" className="mt-2 text-sm text-muted-foreground">
                活跃连接观测已降级。
              </p>
            )}
          </section>

          <section aria-labelledby="connections-heading">
            <div className="mb-3 flex items-center justify-between">
              <div>
                <h2 id="connections-heading" className="font-semibold">
                  当前活跃连接
                </h2>
                <p className="text-sm text-muted-foreground">连接结果与历史记录暂不可用</p>
              </div>
              <Button variant="link" size="sm" onClick={() => navigate("/logs")}>
                查看更多
                <ChevronRight className="size-4" aria-hidden="true" />
              </Button>
            </div>
            <Card className="gap-0 overflow-hidden border-border bg-card py-0 shadow-none">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>开始时间</TableHead>
                    <TableHead>目标</TableHead>
                    <TableHead>端口</TableHead>
                    <TableHead>规则</TableHead>
                    <TableHead>出口链</TableHead>
                    <TableHead>状态</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {connections?.recent.map((item) => (
                    <TableRow key={item.id}>
                      <TableCell>{item.started_at}</TableCell>
                      <TableCell>{item.target_host}</TableCell>
                      <TableCell>{item.target_port}</TableCell>
                      <TableCell>{item.matched_rule ?? "未命中"}</TableCell>
                      <TableCell>{item.outbound_chain.join(" → ")}</TableCell>
                      <TableCell>
                        <Badge variant="outline">
                          <CircleDot className="size-3" />
                          进行中
                        </Badge>
                      </TableCell>
                    </TableRow>
                  ))}
                  {!connections?.recent.length && (
                    <TableRow>
                      <TableCell colSpan={6} className="text-center text-muted-foreground">
                        {connections?.status === "degraded" ? "连接观测不可用" : "暂无活跃连接"}
                      </TableCell>
                    </TableRow>
                  )}
                </TableBody>
              </Table>
            </Card>
          </section>
        </div>
      </div>
    </>
  );
}

function Statistic({ label, value }: { label: string; value: string }) {
  return (
    <div className="p-5 sm:border-r sm:border-border last:border-r-0">
      <p className="text-sm text-muted-foreground">{label}</p>
      <p className="mt-2 text-2xl font-semibold">{value}</p>
    </div>
  );
}
