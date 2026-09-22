import { Copy, Search, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
const connectionLogs = [
  {
    time: "14:32:18",
    target: "api.github.com",
    port: "443",
    exit: "代理",
    rule: "常用站点",
    status: "成功",
  },
  {
    time: "14:31:05",
    target: "192.168.1.10",
    port: "22",
    exit: "直连",
    rule: "局域网",
    status: "成功",
  },
  {
    time: "14:29:17",
    target: "example.com",
    port: "443",
    exit: "代理",
    rule: "常用站点",
    status: "成功",
  },
  {
    time: "14:28:03",
    target: "bad.example.com",
    port: "80",
    exit: "代理",
    rule: "-",
    status: "失败",
  },
  {
    time: "14:26:41",
    target: "10.0.0.5",
    port: "3389",
    exit: "直连",
    rule: "公司内网",
    status: "成功",
  },
];

function StatusDot({ tone = "success" }: { tone?: "success" | "danger" }) {
  const colors = { success: "bg-emerald-500", danger: "bg-rose-500" };

  return <span aria-hidden="true" className={`size-2.5 rounded-full ${colors[tone]}`} />;
}

export default function ConnectionLogs() {
  return (
    <>
      <header className="border-b border-white/5 px-5 py-4 sm:px-6">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">连接日志</h1>
          <p className="mt-1 text-sm text-muted-foreground">查看代理行为和故障定位信息。</p>
        </div>
      </header>

      <div className="content-scroll min-h-0 flex-1 overflow-y-auto px-5 py-5 sm:px-6">
        <div className="w-full space-y-5">
          <section
            className="flex flex-col gap-3 lg:flex-row lg:items-center"
            aria-label="日志筛选"
          >
            <div className="relative w-full lg:max-w-xl">
              <Search
                className="pointer-events-none absolute top-1/2 left-3 size-5 -translate-y-1/2 text-muted-foreground"
                aria-hidden="true"
              />
              <Input
                readOnly
                className="h-11 bg-card pl-10"
                placeholder="搜索目标地址、规则、错误信息..."
              />
            </div>
            <Select defaultValue="all">
              <SelectTrigger aria-label="结果筛选" className="h-11 lg:w-40">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">全部结果</SelectItem>
                <SelectItem value="success">成功</SelectItem>
                <SelectItem value="failure">失败</SelectItem>
              </SelectContent>
            </Select>
            <Select defaultValue="all">
              <SelectTrigger aria-label="出口筛选" className="h-11 lg:w-40">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">全部出站</SelectItem>
                <SelectItem value="proxy">代理</SelectItem>
                <SelectItem value="direct">直连</SelectItem>
              </SelectContent>
            </Select>
            <Button variant="secondary" className="h-11 lg:ml-auto" type="button">
              <Trash2 className="size-5" aria-hidden="true" />
              清空日志
            </Button>
          </section>

          <Card className="gap-0 overflow-hidden border-white/10 bg-card py-0 shadow-none">
            <Table className="min-w-[760px] text-sm">
              <TableHeader className="border-white/10 bg-white/[0.04] [&_th]:h-12 [&_th]:px-4 [&_th]:text-xs [&_th]:font-medium [&_th]:text-muted-foreground">
                <TableRow className="border-white/10 hover:bg-transparent">
                  <TableHead>时间</TableHead>
                  <TableHead>目标</TableHead>
                  <TableHead>端口</TableHead>
                  <TableHead>出站</TableHead>
                  <TableHead>规则</TableHead>
                  <TableHead>结果</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {connectionLogs.map((log) => {
                  const failed = log.status === "失败";

                  return (
                    <TableRow
                      key={`${log.time}-${log.target}`}
                      data-state={failed ? "selected" : undefined}
                      className={`border-white/10 hover:bg-white/[0.035] ${
                        failed
                          ? "data-[state=selected]:bg-blue-500/20 data-[state=selected]:hover:bg-blue-500/25"
                          : ""
                      }`}
                    >
                      <TableCell className="px-4 py-3.5 font-medium">
                        <span className="flex items-center gap-2">
                          <StatusDot tone={failed ? "danger" : "success"} />
                          {log.time}
                        </span>
                      </TableCell>
                      <TableCell className="px-4 py-3.5 font-medium">{log.target}</TableCell>
                      <TableCell className="px-4 py-3.5 text-muted-foreground">
                        {log.port}
                      </TableCell>
                      <TableCell className="px-4 py-3.5">{log.exit}</TableCell>
                      <TableCell className="px-4 py-3.5 text-muted-foreground">
                        {log.rule}
                      </TableCell>
                      <TableCell className="px-4 py-3.5">
                        <span
                          className={`flex items-center gap-2 font-medium ${
                            failed ? "text-rose-400" : "text-emerald-400"
                          }`}
                        >
                          <StatusDot tone={failed ? "danger" : "success"} />
                          {log.status}
                        </span>
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          </Card>

          <Card className="relative gap-0 border-white/10 bg-card py-0 shadow-none">
            <CardContent className="grid gap-6 py-5 pr-14 text-sm md:grid-cols-2">
              <dl className="grid grid-cols-[128px_1fr] gap-x-4 gap-y-3">
                <dt className="text-muted-foreground">时间</dt>
                <dd className="font-medium">2024-01-20 14:28:03</dd>
                <dt className="text-muted-foreground">目标地址</dt>
                <dd className="font-medium">bad.example.com</dd>
                <dt className="text-muted-foreground">端口</dt>
                <dd className="font-medium">80</dd>
                <dt className="text-muted-foreground">实际出口</dt>
                <dd className="font-medium">代理（公司代理）</dd>
              </dl>
              <dl className="grid grid-cols-[128px_1fr] gap-x-4 gap-y-3">
                <dt className="text-muted-foreground">命中规则</dt>
                <dd className="font-medium">-</dd>
                <dt className="text-muted-foreground">结果</dt>
                <dd className="font-semibold text-rose-400">失败</dd>
                <dt className="text-muted-foreground">错误原因</dt>
                <dd className="font-medium">连接超时（Connection timed out）</dd>
                <dt className="text-muted-foreground">建议</dt>
                <dd className="font-medium text-muted-foreground">
                  请检查目标地址是否可达，或更换代理。
                </dd>
              </dl>
            </CardContent>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="absolute top-3 right-3 text-muted-foreground"
              aria-label="复制日志详情"
            >
              <Copy className="size-5" aria-hidden="true" />
            </Button>
          </Card>
        </div>
      </div>
    </>
  );
}
