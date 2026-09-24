import { useCallback, useEffect, useMemo, useState } from "react";
import { Copy, Search, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
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
import { command, errorMessage, type ActiveConnection } from "@/lib/backend";
import { useBackend } from "@/lib/backend-context";

type Diagnostic = { id: string; created_at_ms: number; severity: string; summary: string };
type DiagnosticPage = { items: Diagnostic[]; total: number; next_offset: number | null };
type Filter = { from_ms: number | null; until_ms: number | null; severity: string | null };

export default function ConnectionLogs() {
  const { connections } = useBackend();
  const [query, setQuery] = useState("");
  const [severity, setSeverity] = useState("all");
  const [range, setRange] = useState("7");
  const [asOfMs] = useState(() => Date.now());
  const [page, setPage] = useState<DiagnosticPage | null>(null);
  const [selected, setSelected] = useState<ActiveConnection | null>(null);
  const [clearOpen, setClearOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const filter = useMemo<Filter>(
    () => ({
      from_ms: range === "all" ? null : asOfMs - Number(range) * 86_400_000,
      until_ms: null,
      severity: severity === "all" ? null : severity,
    }),
    [asOfMs, range, severity],
  );

  const refresh = useCallback(async () => {
    try {
      setPage(
        await command<DiagnosticPage>("get_runtime_diagnostics", { filter, offset: 0, limit: 100 }),
      );
      setError(null);
    } catch (reason) {
      setError(errorMessage(reason));
    }
  }, [filter]);
  useEffect(() => {
    const timer = window.setTimeout(() => void refresh(), 0);
    return () => window.clearTimeout(timer);
  }, [refresh]);

  async function copyDetail(id: string) {
    setBusy(true);
    setError(null);
    try {
      const text = await command<string>("copy_active_connection_detail", { id });
      await navigator.clipboard.writeText(text);
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  async function clearDiagnostics() {
    setBusy(true);
    setError(null);
    try {
      await command<number>("clear_runtime_diagnostics", { filter, confirmed: true });
      setClearOpen(false);
      await refresh();
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  const active =
    connections?.recent.filter((connection) =>
      `${connection.target_host} ${connection.matched_rule ?? ""}`
        .toLowerCase()
        .includes(query.toLowerCase()),
    ) ?? [];
  const diagnostics =
    page?.items.filter((item) => item.summary.toLowerCase().includes(query.toLowerCase())) ?? [];

  return (
    <>
      <header className="border-b border-sidebar-border bg-sidebar px-5 py-4 text-sidebar-foreground sm:px-6">
        <h1 className="text-2xl font-semibold tracking-tight">连接日志</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          活跃连接和运行时诊断。已完成连接历史、成功率与失败详情暂不可用。
        </p>
      </header>
      <div className="content-scroll min-h-0 flex-1 overflow-y-auto px-5 py-5 sm:px-6">
        <div className="w-full space-y-5">
          {error && (
            <p role="alert" className="text-destructive">
              {error}
            </p>
          )}
          <div className="flex flex-wrap gap-3" aria-label="日志筛选">
            <div className="relative flex-1">
              <Search
                className="pointer-events-none absolute top-1/2 left-3 size-5 -translate-y-1/2 text-muted-foreground"
                aria-hidden="true"
              />
              <Input
                className="h-11 bg-card pl-10"
                placeholder="搜索目标、规则或诊断..."
                value={query}
                onChange={(event) => setQuery(event.target.value)}
              />
            </div>
            <Select value={severity} onValueChange={setSeverity}>
              <SelectTrigger aria-label="诊断级别筛选" className="h-11 w-40">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">全部级别</SelectItem>
                <SelectItem value="info">信息</SelectItem>
                <SelectItem value="warning">警告</SelectItem>
                <SelectItem value="error">错误</SelectItem>
              </SelectContent>
            </Select>
            <Select value={range} onValueChange={setRange}>
              <SelectTrigger aria-label="诊断时间范围" className="h-11 w-40">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="7">最近 7 天</SelectItem>
                <SelectItem value="30">最近 30 天</SelectItem>
                <SelectItem value="90">最近 90 天</SelectItem>
                <SelectItem value="all">全部时间</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <section aria-labelledby="active-heading">
            <h2 id="active-heading" className="mb-3 font-semibold">
              当前活跃连接（{connections?.active_count ?? "不可用"}）
            </h2>
            {connections?.status === "degraded" && (
              <p role="status" className="mb-3 text-sm text-muted-foreground">
                活跃连接接口不可用，观测已降级。
              </p>
            )}
            <Card className="gap-0 overflow-hidden border-border bg-card py-0 shadow-none">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>开始时间</TableHead>
                    <TableHead>目标</TableHead>
                    <TableHead>命中规则</TableHead>
                    <TableHead>出口链</TableHead>
                    <TableHead>详情</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {active.map((item) => (
                    <TableRow key={item.id}>
                      <TableCell>{item.started_at}</TableCell>
                      <TableCell>
                        {item.target_host}:{item.target_port}
                      </TableCell>
                      <TableCell>{item.matched_rule ?? "未命中"}</TableCell>
                      <TableCell>{item.outbound_chain.join(" → ")}</TableCell>
                      <TableCell>
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          aria-label={`查看${item.target_host}详情`}
                          onClick={() => setSelected(item)}
                        >
                          <Copy className="size-4" />
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                  {active.length === 0 && (
                    <TableRow>
                      <TableCell colSpan={5} className="text-center text-muted-foreground">
                        暂无活跃连接
                      </TableCell>
                    </TableRow>
                  )}
                </TableBody>
              </Table>
            </Card>
          </section>

          <section aria-labelledby="diagnostics-heading">
            <div className="mb-3 flex items-center justify-between">
              <h2 id="diagnostics-heading" className="font-semibold">
                运行时诊断（{page?.total ?? "不可用"}）
              </h2>
              <Button
                variant="outline"
                disabled={busy || !page?.total}
                onClick={() => setClearOpen(true)}
              >
                <Trash2 className="size-4" />
                清理诊断
              </Button>
            </div>
            <Card className="gap-0 overflow-hidden border-border bg-card py-0 shadow-none">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>时间</TableHead>
                    <TableHead>级别</TableHead>
                    <TableHead>摘要</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {diagnostics.map((item) => (
                    <TableRow key={item.id}>
                      <TableCell>{new Date(item.created_at_ms).toLocaleString()}</TableCell>
                      <TableCell>{item.severity}</TableCell>
                      <TableCell>{item.summary}</TableCell>
                    </TableRow>
                  ))}
                  {diagnostics.length === 0 && (
                    <TableRow>
                      <TableCell colSpan={3} className="text-center text-muted-foreground">
                        暂无运行时诊断
                      </TableCell>
                    </TableRow>
                  )}
                </TableBody>
              </Table>
            </Card>
            {page?.next_offset != null && (
              <p className="mt-2 text-sm text-muted-foreground">
                仅显示前 100 条，可调整时间范围筛选。
              </p>
            )}
          </section>
        </div>
      </div>
      <Dialog open={selected !== null} onOpenChange={(open) => !open && setSelected(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>活跃连接详情</DialogTitle>
          </DialogHeader>
          {selected && (
            <div className="space-y-2 break-all text-sm">
              <p>
                目标：{selected.target_host}:{selected.target_port}
              </p>
              <p>开始时间：{selected.started_at}</p>
              <p>命中规则：{selected.matched_rule ?? "未命中"}</p>
              <p>出口链：{selected.outbound_chain.join(" → ")}</p>
              <p className="text-muted-foreground">该连接仍在进行中，最终结果不可用。</p>
              <Button disabled={busy} onClick={() => void copyDetail(selected.id)}>
                <Copy className="size-4" />
                复制脱敏详情
              </Button>
            </div>
          )}
        </DialogContent>
      </Dialog>
      <Dialog open={clearOpen} onOpenChange={setClearOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>确认清理诊断</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">
            仅删除当前时间范围和级别匹配的运行时诊断；活跃连接不受影响。
          </p>
          <DialogFooter>
            <Button variant="secondary" onClick={() => setClearOpen(false)}>
              取消
            </Button>
            <Button variant="destructive" disabled={busy} onClick={() => void clearDiagnostics()}>
              确认清理
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
