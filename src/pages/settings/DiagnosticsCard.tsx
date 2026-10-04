import { useCallback, useEffect, useRef, useState } from "react";
import { Download, RefreshCw } from "lucide-react";
import { ErrorAlert } from "@/components/ErrorAlert";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ipc } from "@/lib/ipc";
import { normalizeError } from "@/lib/error-handler";
import type {
  AppError,
  DiagnosticFilter,
  DiagnosticGroup,
  DiagnosticPage,
} from "@/lib/generated/ipc";

const levels: Record<string, string> = { info: "信息", warning: "警告", error: "错误" };

/** Old plain summaries remain readable alongside structured error diagnostics. */
function summary(text: string): string {
  try {
    const value: unknown = JSON.parse(text);
    if (
      typeof value === "object" &&
      value !== null &&
      "summary" in value &&
      typeof value.summary === "string"
    )
      return value.summary;
  } catch {
    // Earlier releases saved plain text summaries.
  }
  return text;
}

export function DiagnosticsCard() {
  const [search, setSearch] = useState("");
  const [severity, setSeverity] = useState("all");
  const [page, setPage] = useState<DiagnosticPage | null>(null);
  const [groups, setGroups] = useState<DiagnosticGroup[]>([]);
  const [error, setError] = useState<AppError | null>(null);
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const active = useRef(false);
  const generation = useRef(0);
  const exportFlight = useRef(false);

  const filter: DiagnosticFilter = {
    from_ms: null,
    until_ms: null,
    severity: severity === "all" ? null : severity,
    ...(search ? { search } : {}),
  };
  const load = useCallback(async () => {
    const version = ++generation.current;
    const query: DiagnosticFilter = {
      from_ms: null,
      until_ms: null,
      severity: severity === "all" ? null : severity,
      ...(search ? { search } : {}),
    };
    setLoading(true);
    setPage(null);
    setGroups([]);
    setError(null);
    try {
      const [next, aggregated] = await Promise.all([
        ipc("get_runtime_diagnostics", { filter: query, offset: 0, limit: 100 }),
        ipc("get_diagnostic_groups", { filter: query }),
      ]);
      if (!active.current || version !== generation.current) return;
      setPage(next);
      setGroups(aggregated);
    } catch (reason) {
      if (active.current && version === generation.current) setError(normalizeError(reason));
    } finally {
      if (active.current && version === generation.current) setLoading(false);
    }
  }, [severity, search]);

  useEffect(() => {
    const requests = generation;
    active.current = true;
    const timer = window.setTimeout(() => void load(), 250);
    return () => {
      active.current = false;
      requests.current++;
      window.clearTimeout(timer);
    };
  }, [load]);

  async function exportDiagnostics() {
    if (exportFlight.current) return;
    exportFlight.current = true;
    setExporting(true);
    setError(null);
    setMessage(null);
    try {
      const saved = await ipc("save_runtime_diagnostics", { filter });
      if (active.current && saved) setMessage("已保存当前筛选的全部诊断记录。");
    } catch (reason) {
      if (active.current) setError(normalizeError(reason));
    } finally {
      exportFlight.current = false;
      if (active.current) setExporting(false);
    }
  }

  return (
    <Card
      role="region"
      aria-label="诊断信息"
      className="gap-0 border-border bg-card py-0 shadow-none"
    >
      <CardHeader className="py-5">
        <CardTitle role="heading" aria-level={2}>
          诊断信息
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4 border-t border-border py-5">
        <div className="flex flex-wrap items-center gap-3">
          <Select value={severity} onValueChange={setSeverity} disabled={exporting}>
            <SelectTrigger aria-label="设置诊断级别" className="w-36">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部级别</SelectItem>
              {Object.entries(levels).map(([level, label]) => (
                <SelectItem key={level} value={level}>
                  {label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Input
            aria-label="搜索诊断"
            placeholder="搜索编号、摘要、类型或操作"
            value={search}
            maxLength={128}
            disabled={exporting}
            onChange={(event) => setSearch(event.target.value)}
            className="w-72"
          />
          <Button variant="outline" disabled={loading} onClick={() => void load()}>
            <RefreshCw className="size-4" />
            刷新诊断
          </Button>
          <Button variant="outline" disabled={exporting} onClick={() => void exportDiagnostics()}>
            <Download className="size-4" />
            {exporting ? "正在导出…" : "导出诊断 JSON Lines"}
          </Button>
        </div>
        {error && <ErrorAlert error={error} />}
        {loading && <p role="status">正在加载诊断…</p>}
        {message && <p role="status">{message}</p>}
        <p className="text-sm text-muted-foreground">
          共 {page?.total ?? "—"} 条记录；显示最近 100 条，导出包含当前筛选的全部记录。
        </p>
        {groups.length > 0 && (
          <section aria-label="同类错误聚合" className="space-y-2">
            <h3 className="text-sm font-medium">同类错误</h3>
            {groups.map((group) => (
              <p
                className="text-sm"
                key={JSON.stringify([group.error_type, group.operation, group.severity])}
              >
                {group.error_type} · {group.operation ?? "未指定操作"} ·{" "}
                {levels[group.severity] ?? group.severity} · {group.occurrences} 次
                <span className="ml-2 text-muted-foreground">
                  最近：{new Date(group.last_seen_ms).toLocaleString()}
                </span>
              </p>
            ))}
          </section>
        )}
        <div role="group" className="max-h-80 space-y-2 overflow-y-auto" aria-label="诊断记录">
          {page?.items.map((item) => (
            <details key={item.id} className="rounded-md border border-border p-3 text-sm">
              <summary className="cursor-pointer">
                {levels[item.severity] ?? item.severity} · {summary(item.summary)} ·{" "}
                {new Date(item.created_at_ms).toLocaleString()}
              </summary>
              <p className="mt-2 break-all text-muted-foreground">
                编号：{item.id}
                {item.error_type ? ` · 类型：${item.error_type}` : ""}
                {item.operation ? ` · 操作：${item.operation}` : ""}
              </p>
              <pre className="mt-2 whitespace-pre-wrap break-all text-xs">{item.summary}</pre>
            </details>
          ))}
          {page?.total === 0 && <p className="text-sm text-muted-foreground">暂无诊断记录</p>}
        </div>
      </CardContent>
    </Card>
  );
}
