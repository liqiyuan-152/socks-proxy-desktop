import { CheckCircle2, ChevronRight, CircleDot, XCircle } from "lucide-react";
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
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { proxyModes, type ProxyMode } from "@/lib/proxy-mode";
const recentConnections = [
  { time: "09:42:11", target: "example.com", port: "443", rule: "常用站点", status: "成功" },
  { time: "09:38:54", target: "git.intra.local", port: "22", rule: "公司内网", status: "成功" },
  { time: "09:35:02", target: "api.remote.net", port: "443", rule: "默认规则", status: "失败" },
];

function StatusDot({ tone = "success" }: { tone?: "success" | "danger" }) {
  const colors = { success: "bg-emerald-500", danger: "bg-rose-500" };

  return <span aria-hidden="true" className={`size-2.5 rounded-full ${colors[tone]}`} />;
}

export default function StatusDashboard({
  mode,
  setMode,
  activeMode,
}: {
  mode: ProxyMode;
  setMode: (mode: ProxyMode) => void;
  activeMode: (typeof proxyModes)[ProxyMode];
}) {
  return (
    <>
      <header className="border-b border-sidebar-border bg-sidebar px-5 py-4 text-sidebar-foreground sm:px-6">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">状态</h1>
          <p className="mt-1 text-sm text-muted-foreground">当前网络模式和实际运行状态</p>
        </div>
      </header>

      <div className="content-scroll min-h-0 flex-1 overflow-y-auto px-5 py-5 sm:px-6">
        <div className="w-full space-y-6">
          <Tabs value={mode} onValueChange={(value) => setMode(value as ProxyMode)}>
            <TabsList className="grid h-11 w-full grid-cols-3 overflow-hidden rounded-lg border border-white/15 bg-white/[0.035] p-0 sm:mx-auto sm:max-w-2xl">
              {(Object.keys(proxyModes) as ProxyMode[]).map((key) => (
                <TabsTrigger
                  key={key}
                  value={key}
                  className="h-full rounded-none border-0 border-r border-white/10 bg-transparent text-xs text-muted-foreground last:border-r-0 hover:bg-white/[0.045] hover:text-foreground data-[state=active]:!border-transparent data-[state=active]:!bg-blue-600 data-[state=active]:!text-white data-[state=active]:shadow-[inset_0_1px_0_rgb(255_255_255_/_0.18)] sm:text-sm"
                  onClick={() => setMode(key)}
                >
                  {proxyModes[key].label}
                </TabsTrigger>
              ))}
            </TabsList>
            {(Object.keys(proxyModes) as ProxyMode[]).map((key) => (
              <TabsContent key={key} value={key} className="sr-only">
                {proxyModes[key].description}
              </TabsContent>
            ))}
          </Tabs>

          <div className="grid gap-5 lg:grid-cols-2">
            <Card className="gap-0 border-white/10 bg-card py-0 shadow-none">
              <CardHeader className="border-b border-border py-5">
                <CardTitle role="heading" aria-level={2}>
                  当前代理信息
                </CardTitle>
              </CardHeader>
              <CardContent className="grid gap-y-4 py-5 text-sm sm:grid-cols-[132px_1fr]">
                <span className="text-muted-foreground">代理名称</span>
                <span className="font-medium">公司代理</span>
                <span className="text-muted-foreground">协议</span>
                <Badge
                  variant="outline"
                  className="w-fit border-blue-400/30 bg-blue-500/10 text-blue-300"
                >
                  SOCKS5
                </Badge>
                <span className="text-muted-foreground">服务器</span>
                <span className="font-medium">172.30.12.8</span>
                <span className="text-muted-foreground">端口</span>
                <span className="font-medium">1080</span>
                <span className="text-muted-foreground">认证状态</span>
                <span className="flex items-center gap-2 font-medium text-emerald-400">
                  <CheckCircle2 className="size-4" aria-hidden="true" />
                  已启用
                </span>
              </CardContent>
            </Card>

            <Card className="gap-0 border-white/10 bg-card py-0 shadow-none">
              <CardHeader className="border-b border-border py-5">
                <CardTitle role="heading" aria-level={2}>
                  内核状态
                </CardTitle>
              </CardHeader>
              <CardContent className="grid gap-y-4 py-5 text-sm sm:grid-cols-[132px_1fr]">
                <span className="text-muted-foreground">当前模式</span>
                <span data-testid="active-mode" className="font-medium text-blue-400">
                  {activeMode.label}
                </span>
                <span className="text-muted-foreground">运行时长</span>
                <span className="flex items-center gap-2 font-medium">
                  <CircleDot className="size-4 text-blue-500" aria-hidden="true" />
                  03:42:18
                </span>
                <span className="text-muted-foreground">TUN 状态</span>
                <span className="flex items-center gap-2 font-medium text-emerald-400">
                  <StatusDot />
                  已启用
                </span>
                <span className="text-muted-foreground">DNS 状态</span>
                <span className="flex items-center gap-2 font-medium text-emerald-400">
                  <StatusDot />
                  正常
                </span>
              </CardContent>
            </Card>
          </div>

          <section aria-labelledby="statistics-heading">
            <div className="mb-3 flex items-center justify-between">
              <div>
                <h2 id="statistics-heading" className="text-base font-semibold">
                  连接统计
                </h2>
                <p className="text-sm text-muted-foreground">今日流量概览</p>
              </div>
              <Badge variant="outline" className="hidden sm:inline-flex">
                今日
              </Badge>
            </div>
            <Card className="grid gap-0 overflow-hidden border-white/10 bg-card py-0 shadow-none sm:grid-cols-4">
              <Statistic label="今日连接数" value="148" />
              <Statistic label="成功数" value="139" tone="success" />
              <Statistic label="失败数" value="9" tone="danger" />
              <div className="p-5 sm:border-l sm:border-border">
                <p className="text-sm text-muted-foreground">最近命中规则</p>
                <p className="mt-2 text-base font-semibold text-blue-400">{activeMode.rule}</p>
              </div>
            </Card>
          </section>

          <section aria-labelledby="connections-heading">
            <div className="mb-3 flex items-center justify-between">
              <div>
                <h2 id="connections-heading" className="text-base font-semibold">
                  最近连接记录
                </h2>
                <p className="text-sm text-muted-foreground">最近的网络连接结果</p>
              </div>
              <Button variant="link" size="sm" className="text-blue-400">
                查看更多
                <ChevronRight className="size-4" aria-hidden="true" />
              </Button>
            </div>
            <Card className="gap-0 overflow-hidden border-white/10 bg-card py-0 shadow-none">
              <Table className="text-sm">
                <TableHeader className="border-white/10 bg-white/[0.04] [&_th]:h-11 [&_th]:px-4 [&_th]:text-xs [&_th]:font-medium [&_th]:text-muted-foreground">
                  <TableRow className="border-white/10 hover:bg-transparent">
                    <TableHead>时间</TableHead>
                    <TableHead>目标</TableHead>
                    <TableHead>端口</TableHead>
                    <TableHead>规则</TableHead>
                    <TableHead>结果</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {recentConnections.map((connection) => {
                    const successful = connection.status === "成功";
                    return (
                      <TableRow
                        key={`${connection.time}-${connection.target}`}
                        className="border-white/10 hover:bg-white/[0.035]"
                      >
                        <TableCell className="px-4 py-3 text-muted-foreground">
                          {connection.time}
                        </TableCell>
                        <TableCell className="px-4 py-3 font-medium">{connection.target}</TableCell>
                        <TableCell className="px-4 py-3 text-muted-foreground">
                          {connection.port}
                        </TableCell>
                        <TableCell className="px-4 py-3">{connection.rule}</TableCell>
                        <TableCell className="px-4 py-3">
                          <span
                            className={
                              successful
                                ? "flex items-center gap-2 text-emerald-400"
                                : "flex items-center gap-2 text-rose-400"
                            }
                          >
                            {successful ? (
                              <CheckCircle2 className="size-4" aria-hidden="true" />
                            ) : (
                              <XCircle className="size-4" aria-hidden="true" />
                            )}
                            {connection.status}
                          </span>
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </Card>
          </section>
        </div>
      </div>
    </>
  );
}

function Statistic({
  label,
  value,
  tone = "default",
}: {
  label: string;
  value: string;
  tone?: "default" | "success" | "danger";
}) {
  const valueColor = {
    default: "text-foreground",
    success: "text-emerald-400",
    danger: "text-rose-400",
  };

  return (
    <div className="p-5 sm:border-r sm:border-border last:border-r-0">
      <p className="text-sm text-muted-foreground">{label}</p>
      <p className={`mt-2 text-3xl font-semibold tracking-tight ${valueColor[tone]}`}>{value}</p>
    </div>
  );
}
