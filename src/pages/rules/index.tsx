import { Badge } from "@/components/ui/badge";
import { PageHeader } from "@/components/PageHeader";
import { normalizeError } from "@/lib/error-handler";
import { ErrorAlert } from "@/components/ErrorAlert";
import { useState } from "react";
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
import { useRoutingRules, type RoutingRule } from "./useRoutingRules";
import { useBackendStore } from "@/store/backend-store";
import { useShallow } from "zustand/react/shallow";
import { RuleForm, type RuleDraft } from "./RuleForm";
import { ChinaDirectPreset } from "./ChinaDirectPreset";
import { RouteTest } from "./RouteTest";
import { ConfirmDeletionDialog } from "@/components/ConfirmDeletionDialog";

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
  const { profiles, snapshot } = useBackendStore(
    useShallow((state) => ({ profiles: state.profiles, snapshot: state.snapshot })),
  );
  const { routingRules, loading, busy, error, setError, replace, moveRule } = useRoutingRules();
  const [search, setSearch] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<RoutingRule | null>(null);
  const [deletingRule, setDeletingRule] = useState<RoutingRule | null>(null);
  const [draft, setDraft] = useState<RuleDraft>(() => createRuleDraft());
  const isEditing = editingRule !== null;

  async function saveRule() {
    const start = draft.port ? Number(draft.port) : null;
    const end = draft.portEnd ? Number(draft.portEnd) : null;
    if (
      (start !== null && (!Number.isInteger(start) || start < 1 || start > 65535)) ||
      (end !== null && (start === null || !Number.isInteger(end) || end < start || end > 65535))
    ) {
      setError(
        normalizeError({
          code: "validation_error",
          message: "端口范围必须在 1 到 65535 之间，且结束端口不小于起始端口。",
        }),
      );
      return;
    }
    const rule: RoutingRule = {
      id: editingRule?.id ?? crypto.randomUUID(),
      name: draft.name,
      matcher: draft.targetType,
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
      <PageHeader title="分流规则" description="定义哪些流量走代理，哪些流量直连。" />

      <div className="content-scroll animate-fade-in min-h-0 flex-1 overflow-y-auto px-4 py-4 sm:px-6 sm:py-6">
        <div className="mx-auto w-full max-w-7xl space-y-5">
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
          {error && !dialogOpen && !deletingRule && <ErrorAlert error={error} />}
          {loading && <p role="status">正在加载分流规则…</p>}

          <Card variant="elevated" className="gap-0 overflow-hidden py-0 hover:border-primary/30">
            <Table
              className="table-modern min-w-[760px] text-sm"
              containerClassName="max-h-[60vh] overflow-auto"
            >
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
                        <Badge
                          variant={
                            rule.matcher === "ip_cidr"
                              ? "warning"
                              : rule.matcher === "domain_suffix"
                                ? "success"
                                : "info"
                          }
                        >
                          {rule.matcher === "ip_cidr"
                            ? "CIDR"
                            : rule.matcher === "domain_suffix"
                              ? "域名后缀"
                              : "域名"}
                        </Badge>
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
                          className={
                            rule.action === "proxy" ? "text-primary-text" : "text-foreground"
                          }
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
                                  setError(null);
                                  setDeletingRule(rule);
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

      {deletingRule && (
        <ConfirmDeletionDialog
          name={deletingRule.name}
          resource="规则"
          error={error}
          onCancel={() => setDeletingRule(null)}
          onConfirm={() => replace(routingRules.filter((item) => item.id !== deletingRule.id))}
        />
      )}
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
            error={error}
            profiles={profiles}
            onSave={() => void saveRule()}
          />
        </DialogContent>
      </Dialog>
    </>
  );
}
