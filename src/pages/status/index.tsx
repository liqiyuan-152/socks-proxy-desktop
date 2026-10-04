import { DailyConnectionTrend } from "./DailyConnectionTrend";
import { ConnectionStatistic } from "./ConnectionStatistic";
import { Skeleton } from "@/components/ui/skeleton";
import { PageHeader } from "@/components/PageHeader";
import { useEffect } from "react";
import { ChevronRight, CircleDot, Activity, CheckCircle2, Gauge, AlertCircle } from "lucide-react";
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
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { useBackendStore } from "@/store/backend-store";
import { useShallow } from "zustand/react/shallow";
import { RuntimeFeedback } from "@/components/RuntimeFeedback";
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
  const {
    capabilities,
    snapshot,
    profiles,
    connections,
    loading,
    pending,
    selectedMode,
    switchMode,
  } = useBackendStore(
    useShallow((state) => ({
      capabilities: state.capabilities,
      snapshot: state.snapshot,
      profiles: state.profiles,
      connections: state.connections,
      loading: state.loading,
      pending: state.pending,
      selectedMode: state.selectedMode,
      switchMode: state.switchMode,
    })),
  );
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
      <PageHeader title="状态" description="当前网络模式和实际运行状态" />
      <div className="content-scroll animate-fade-in min-h-0 flex-1 overflow-y-auto px-4 py-4 sm:px-6 sm:py-6">
        <div className="mx-auto w-full max-w-7xl space-y-6">
          {loading && (
            <div role="status" className="space-y-3">
              <p>正在加载运行时状态…</p>
              <Skeleton className="h-12 w-full" />
            </div>
          )}
          <RuntimeFeedback />
          <RadioGroup
            aria-label="代理模式"
            orientation="horizontal"
            className="grid h-14 w-full grid-cols-3 rounded-xl border-2 border-border bg-gradient-to-b from-muted/30 to-muted/60 p-1.5 sm:mx-auto sm:max-w-2xl"
            value={selectedMode ?? snapshot?.selected_mode ?? ""}
            onValueChange={(mode) => selectMode(mode as ProxyMode)}
          >
            {(Object.keys(proxyModes) as ProxyMode[]).map((mode) => (
              <RadioGroupItem
                key={mode}
                value={mode}
                className="h-full w-full aspect-auto rounded-lg border-0 bg-transparent text-xs font-semibold text-muted-foreground transition-[background-color,box-shadow,scale] duration-200 hover:bg-accent hover:text-foreground data-[state=checked]:!bg-[image:var(--gradient-primary)] data-[state=checked]:!text-primary-foreground data-[state=checked]:shadow-md data-[state=checked]:shadow-primary/25 motion-reduce:transition-none sm:text-sm"
                disabled={loading || !snapshot || !capabilities?.proxy_runtime}
                onClick={() => {
                  if (mode === selectedMode && !pending && snapshot?.applied_mode !== mode) {
                    selectMode(mode);
                  }
                }}
              >
                {proxyModes[mode].label}
              </RadioGroupItem>
            ))}
          </RadioGroup>
          <div className="stagger-children grid gap-5 lg:grid-cols-2">
            <Card variant="elevated" className="gap-0 py-0">
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
            <Card variant="elevated" className="gap-0 py-0">
              <CardHeader className="border-b border-border py-5">
                <CardTitle role="heading" aria-level={2}>
                  内核状态
                </CardTitle>
              </CardHeader>
              <CardContent className="grid gap-y-4 py-5 text-sm sm:grid-cols-[132px_1fr]">
                <span className="text-muted-foreground">运行阶段</span>
                <span>{snapshot ? phaseLabels[snapshot.phase] : "不可用"}</span>
                <span className="text-muted-foreground">会话健康</span>
                <span>
                  {snapshot
                    ? (
                        {
                          inactive: "未运行",
                          healthy: "健康",
                          exited: "已退出",
                          recovery_required: "恢复未完成",
                        } as const
                      )[snapshot.session_health]
                    : "不可用"}
                </span>
                <span className="text-muted-foreground">最近操作</span>
                <span>
                  {snapshot
                    ? (
                        {
                          idle: "无",
                          pending: "处理中",
                          succeeded: "成功",
                          failed: "失败",
                        } as const
                      )[snapshot.last_operation.outcome]
                    : "不可用"}
                </span>
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
            <div className="stagger-children grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
              <ConnectionStatistic
                icon={<Activity className="size-5" />}
                label="当前活跃连接"
                value={connections?.active_count?.toString() ?? "不可用"}
              >
                <DailyConnectionTrend
                  count={connections?.active_count ?? null}
                  trend={connections?.trend ?? null}
                />
              </ConnectionStatistic>
              <ConnectionStatistic
                icon={<CheckCircle2 className="size-5" />}
                label="已完成连接"
                value="不可用"
              />
              <ConnectionStatistic
                icon={<Gauge className="size-5" />}
                label="成功率"
                value="不可用"
              />
              <ConnectionStatistic
                icon={<AlertCircle className="size-5" />}
                label="失败详情"
                value="不可用"
              />
            </div>
            {connections?.status === "degraded" && (
              <p role="status" className="mt-2 text-sm text-muted-foreground">
                {connections.diagnostic ?? "活跃连接观测已降级。"}
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
