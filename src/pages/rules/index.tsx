import { useCallback, useEffect, useState } from "react";
import { Edit3, Plus, Search, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { command, errorMessage, type BackendError } from "@/lib/backend";
import { useBackend } from "@/lib/backend-context";
import { RuleForm, type RuleDraft } from "./RuleForm";
import { ChinaDirectPreset } from "./ChinaDirectPreset";
import { RouteTest } from "./RouteTest";

type RoutingRule = {
  id: string;
  name: string;
  matcher: "domain" | "domain_suffix" | "ip_cidr";
  target: string;
  port_start: number | null;
  port_end: number | null;
  action: "proxy" | "direct";
  proxy_profile_id: string | null;
  enabled: boolean;
};

function createRuleDraft(rule?: RoutingRule, defaultProfileId = ""): RuleDraft {
  const action = rule?.action === "direct" ? "直连" : "代理";

  return {
    name: rule?.name ?? "",
    targetType: rule?.matcher ?? "domain",
    target: rule?.target ?? "",
    port: rule?.port_start?.toString() ?? "",
    portEnd: rule?.port_end && rule.port_end !== rule.port_start ? String(rule.port_end) : "",
    action,
    proxyProfileId: rule?.proxy_profile_id ?? defaultProfileId,
  };
}

export default function RoutingRuleList() {
  const { profiles, snapshot } = useBackend();
  const [routingRules, setRoutingRules] = useState<RoutingRule[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<RoutingRule | null>(null);
  const [draft, setDraft] = useState<RuleDraft>(() => createRuleDraft());
  const isEditing = editingRule !== null;

  const refresh = useCallback(async () => {
    try {
      setRoutingRules(await command<RoutingRule[]>("list_rules"));
      setError(null);
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setLoading(false);
    }
  }, []);
  useEffect(() => {
    const timer = window.setTimeout(() => void refresh(), 0);
    return () => window.clearTimeout(timer);
  }, [refresh]);

  async function replace(next: RoutingRule[]): Promise<boolean> {
    setBusy(true);
    setError(null);
    try {
      await command("replace_rules", { rules: next });
      await refresh();
      return true;
    } catch (reason) {
      const typed = reason as Partial<BackendError>;
      setError(
        typed.fields?.map((field) => `${field.field}: ${field.message}`).join("；") ||
          errorMessage(reason),
      );
      return false;
    } finally {
      setBusy(false);
    }
  }

  async function saveRule() {
    const start = draft.port ? Number(draft.port) : null;
    const end = draft.portEnd ? Number(draft.portEnd) : null;
    if (
      (start !== null && (!Number.isInteger(start) || start < 1 || start > 65535)) ||
      (end !== null && (start === null || !Number.isInteger(end) || end < start || end > 65535))
    ) {
      setError("端口范围必须在 1 到 65535 之间，且结束端口不小于起始端口。");
      return;
    }
    const rule: RoutingRule = {
      id: editingRule?.id ?? crypto.randomUUID(),
      name: draft.name,
      matcher: draft.targetType as RoutingRule["matcher"],
      target: draft.target,
      port_start: start,
      port_end: end,
      action: draft.action === "代理" ? "proxy" : "direct",
      proxy_profile_id: draft.action === "代理" ? draft.proxyProfileId : null,
      enabled: editingRule?.enabled ?? true,
    };
    const saved = await replace(
      editingRule
        ? routingRules.map((item) => (item.id === rule.id ? rule : item))
        : [...routingRules, rule],
    );
    if (saved) closeDialog();
  }

  async function moveRule(index: number, delta: number) {
    const next = [...routingRules];
    const target = index + delta;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    setBusy(true);
    setError(null);
    try {
      await command("reorder_rules", { ids: next.map((rule) => rule.id) });
      await refresh();
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  function openDialog(rule?: RoutingRule) {
    setEditingRule(rule ?? null);
    setDraft(
      createRuleDraft(
        rule,
        snapshot?.active_profile_id ?? profiles.find((profile) => profile.enabled)?.id,
      ),
    );
    setDialogOpen(true);
  }

  function closeDialog() {
    setDialogOpen(false);
    setEditingRule(null);
  }

  return (
    <>
      <header className="border-b border-sidebar-border bg-sidebar px-5 py-4 text-sidebar-foreground sm:px-6">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">分流规则</h1>
          <p className="mt-1 text-sm text-muted-foreground">定义哪些流量走代理，哪些流量直连。</p>
        </div>
      </header>

      <div className="content-scroll min-h-0 flex-1 overflow-y-auto px-5 py-5 sm:px-6">
        <div className="w-full space-y-5">
          <ChinaDirectPreset />
          <RouteTest />
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
            <div className="relative w-full sm:max-w-md">
              <Search
                className="pointer-events-none absolute top-1/2 left-3 size-5 -translate-y-1/2 text-muted-foreground"
                aria-hidden="true"
              />
              <Input
                className="h-11 bg-card pl-10"
                placeholder="搜索规则名称或目标..."
                value={search}
                onChange={(event) => setSearch(event.target.value)}
              />
            </div>
            <span className="shrink-0 text-sm font-medium text-muted-foreground">
              共 {routingRules.length} 条规则
            </span>
            <Button
              className="h-11 sm:ml-auto sm:min-w-36"
              onClick={() => openDialog()}
              disabled={busy || loading}
            >
              <Plus className="size-5" aria-hidden="true" />
              添加规则
            </Button>
          </div>
          {error && (
            <p role="alert" className="text-destructive">
              {error}
            </p>
          )}
          {loading && <p role="status">正在加载分流规则…</p>}

          <Card className="gap-0 overflow-hidden border-border bg-card py-0 shadow-none">
            <Table className="min-w-[760px] text-sm">
              <TableHeader className="bg-muted/50 [&_th]:h-12 [&_th]:px-4 [&_th]:text-xs [&_th]:font-medium [&_th]:text-muted-foreground">
                <TableRow className="hover:bg-transparent">
                  <TableHead>规则名称</TableHead>
                  <TableHead>匹配目标</TableHead>
                  <TableHead>目标值</TableHead>
                  <TableHead>端口</TableHead>
                  <TableHead>动作</TableHead>
                  <TableHead>启用</TableHead>
                  <TableHead className="text-right">操作</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {routingRules
                  .filter((rule) =>
                    `${rule.name} ${rule.target}`.toLowerCase().includes(search.toLowerCase()),
                  )
                  .map((rule) => (
                    <TableRow key={rule.id}>
                      <TableCell className="px-4 py-4 font-medium">{rule.name}</TableCell>
                      <TableCell className="px-4 py-4 text-muted-foreground">
                        {rule.matcher === "ip_cidr"
                          ? "CIDR"
                          : rule.matcher === "domain_suffix"
                            ? "域名后缀"
                            : "域名"}
                      </TableCell>
                      <TableCell className="px-4 py-4 font-medium">{rule.target}</TableCell>
                      <TableCell className="px-4 py-4 text-muted-foreground">
                        {rule.port_start == null
                          ? "任意"
                          : rule.port_end && rule.port_end !== rule.port_start
                            ? `${rule.port_start}–${rule.port_end}`
                            : rule.port_start}
                      </TableCell>
                      <TableCell className="px-4 py-4">
                        <span
                          className={rule.action === "proxy" ? "text-primary" : "text-foreground"}
                        >
                          {rule.action === "proxy"
                            ? (profiles.find((profile) => profile.id === rule.proxy_profile_id)
                                ?.name ?? "待修复出口")
                            : "直连"}
                        </span>
                      </TableCell>
                      <TableCell className="px-4 py-4">
                        <Switch
                          checked={rule.enabled}
                          disabled={busy}
                          onCheckedChange={(enabled) =>
                            void replace(
                              routingRules.map((item) =>
                                item.id === rule.id ? { ...item, enabled } : item,
                              ),
                            )
                          }
                          aria-label={`${rule.name}${rule.enabled ? "已启用" : "已停用"}`}
                        />
                      </TableCell>
                      <TableCell className="px-4 py-4">
                        <div className="flex justify-end gap-1">
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            disabled={busy || routingRules.indexOf(rule) === 0}
                            aria-label={`上移${rule.name}`}
                            onClick={() => void moveRule(routingRules.indexOf(rule), -1)}
                          >
                            ↑
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            disabled={
                              busy || routingRules.indexOf(rule) === routingRules.length - 1
                            }
                            aria-label={`下移${rule.name}`}
                            onClick={() => void moveRule(routingRules.indexOf(rule), 1)}
                          >
                            ↓
                          </Button>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <Button
                                variant="ghost"
                                size="icon-sm"
                                aria-label={`编辑${rule.name}`}
                                disabled={busy}
                                onClick={() => openDialog(rule)}
                              >
                                <Edit3 className="size-4" aria-hidden="true" />
                              </Button>
                            </TooltipTrigger>
                            <TooltipContent>编辑规则</TooltipContent>
                          </Tooltip>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <Button
                                variant="ghost"
                                size="icon-sm"
                                className="text-destructive hover:bg-destructive/10 hover:text-destructive"
                                aria-label={`删除${rule.name}`}
                                disabled={busy}
                                onClick={() => {
                                  if (window.confirm(`确认删除规则「${rule.name}」？`)) {
                                    void replace(
                                      routingRules.filter((item) => item.id !== rule.id),
                                    );
                                  }
                                }}
                              >
                                <Trash2 className="size-4" aria-hidden="true" />
                              </Button>
                            </TooltipTrigger>
                            <TooltipContent>删除规则</TooltipContent>
                          </Tooltip>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                {!loading && routingRules.length === 0 && (
                  <TableRow>
                    <TableCell colSpan={7} className="text-center text-muted-foreground">
                      暂无分流规则
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </Card>
        </div>
      </div>

      <Dialog
        open={dialogOpen}
        onOpenChange={(open) => {
          setDialogOpen(open);
          if (!open) setEditingRule(null);
        }}
      >
        <DialogContent className="max-h-[calc(100vh-2rem)] p-0 sm:max-h-[680px]">
          <RuleForm
            draft={draft}
            setDraft={setDraft}
            isEditing={isEditing}
            busy={busy}
            profiles={profiles}
            onSave={() => void saveRule()}
          />
        </DialogContent>
      </Dialog>
    </>
  );
}
