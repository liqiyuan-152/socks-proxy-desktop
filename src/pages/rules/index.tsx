import { useState } from "react";
import { Edit3, GitBranch, Plus, Search, Trash2, X } from "lucide-react";
import { Field } from "@/components/forms/Field";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
const routingRules = [
  { name: "公司内网", targetType: "CIDR", target: "173.30.0.0/16", port: "任意", action: "代理" },
  { name: "常用站点", targetType: "域名", target: "example.com", port: "443", action: "代理" },
  { name: "局域网", targetType: "CIDR", target: "192.168.0.0/16", port: "任意", action: "直连" },
  { name: "国内直连", targetType: "域名", target: "cn", port: "任意", action: "直连" },
];

type RoutingRule = (typeof routingRules)[number];

type RuleDraft = {
  name: string;
  targetType: string;
  target: string;
  port: string;
  action: "代理" | "直连";
  remark: string;
};

function createRuleDraft(rule?: RoutingRule): RuleDraft {
  const action = rule?.action === "直连" ? "直连" : "代理";

  return {
    name: rule?.name ?? "常用站点",
    targetType: rule?.targetType ?? "域名",
    target: rule?.target ?? "example.com",
    port: rule?.port === "任意" || !rule ? "443" : rule.port,
    action,
    remark: rule ? `${rule.name}${action === "代理" ? "走代理" : "直连"}` : "常用站点走代理",
  };
}

export default function RoutingRuleList() {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<RoutingRule | null>(null);
  const [draft, setDraft] = useState<RuleDraft>(() => createRuleDraft());
  const isEditing = editingRule !== null;

  function openDialog(rule?: RoutingRule) {
    setEditingRule(rule ?? null);
    setDraft(createRuleDraft(rule));
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
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
            <div className="relative w-full sm:max-w-md">
              <Search
                className="pointer-events-none absolute top-1/2 left-3 size-5 -translate-y-1/2 text-muted-foreground"
                aria-hidden="true"
              />
              <Input readOnly className="h-11 bg-card pl-10" placeholder="搜索规则名称或目标..." />
            </div>
            <span className="shrink-0 text-sm font-medium text-muted-foreground">共 4 条规则</span>
            <Button className="h-11 sm:ml-auto sm:min-w-36" onClick={() => openDialog()}>
              <Plus className="size-5" aria-hidden="true" />
              添加规则
            </Button>
          </div>

          <Card className="gap-0 overflow-hidden border-white/10 bg-card py-0 shadow-none">
            <Table className="min-w-[760px] text-sm">
              <TableHeader className="border-white/10 bg-white/[0.04] [&_th]:h-12 [&_th]:px-4 [&_th]:text-xs [&_th]:font-medium [&_th]:text-muted-foreground">
                <TableRow className="border-white/10 hover:bg-transparent">
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
                {routingRules.map((rule) => (
                  <TableRow key={rule.name} className="border-white/10 hover:bg-white/[0.035]">
                    <TableCell className="px-4 py-4 font-medium">{rule.name}</TableCell>
                    <TableCell className="px-4 py-4 text-muted-foreground">
                      {rule.targetType}
                    </TableCell>
                    <TableCell className="px-4 py-4 font-medium">{rule.target}</TableCell>
                    <TableCell className="px-4 py-4 text-muted-foreground">{rule.port}</TableCell>
                    <TableCell className="px-4 py-4">
                      <span
                        className={rule.action === "代理" ? "text-blue-400" : "text-foreground"}
                      >
                        {rule.action}
                      </span>
                    </TableCell>
                    <TableCell className="px-4 py-4">
                      <Switch
                        checked
                        onCheckedChange={() => undefined}
                        aria-label={`${rule.name}已启用`}
                      />
                    </TableCell>
                    <TableCell className="px-4 py-4">
                      <div className="flex justify-end gap-1">
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <Button
                              variant="ghost"
                              size="icon-sm"
                              aria-label={`编辑${rule.name}`}
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
                              className="text-rose-400 hover:bg-rose-500/10 hover:text-rose-300"
                              aria-label={`删除${rule.name}`}
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
          <form
            onSubmit={(event) => {
              event.preventDefault();
              closeDialog();
            }}
          >
            <DialogHeader className="relative border-b border-white/10 px-6 py-5 sm:px-7">
              <DialogTitle>{isEditing ? "编辑规则" : "添加规则"}</DialogTitle>
              <DialogClose asChild>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  className="absolute top-3 right-4 text-muted-foreground hover:text-foreground"
                  aria-label="关闭规则表单"
                >
                  <X className="size-5" aria-hidden="true" />
                </Button>
              </DialogClose>
            </DialogHeader>

            <div className="space-y-5 px-6 py-6 sm:px-7">
              <div className="grid gap-5 sm:grid-cols-[1.1fr_0.9fr]">
                <Field label="规则名称" required htmlFor="rule-name">
                  <Input
                    id="rule-name"
                    aria-label="规则名称"
                    placeholder="请输入规则名称"
                    value={draft.name}
                    onChange={(event) =>
                      setDraft((current) => ({ ...current, name: event.target.value }))
                    }
                  />
                </Field>
                <Field label="匹配类型" required htmlFor="rule-target-type">
                  <Select
                    value={draft.targetType}
                    onValueChange={(targetType) =>
                      setDraft((current) => ({ ...current, targetType }))
                    }
                  >
                    <SelectTrigger id="rule-target-type" aria-label="匹配类型">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="域名">域名</SelectItem>
                      <SelectItem value="CIDR">CIDR</SelectItem>
                      <SelectItem value="IP">IP</SelectItem>
                    </SelectContent>
                  </Select>
                </Field>
              </div>

              <div className="grid gap-5 sm:grid-cols-[1.7fr_0.8fr]">
                <Field label="目标值" required htmlFor="rule-target">
                  <Input
                    id="rule-target"
                    aria-label="目标值"
                    placeholder="请输入目标值，如 example.com"
                    value={draft.target}
                    onChange={(event) =>
                      setDraft((current) => ({ ...current, target: event.target.value }))
                    }
                  />
                  <p className="text-xs text-muted-foreground">
                    支持单个域名或通配符，如 *.example.com
                  </p>
                </Field>
                <Field label="端口" htmlFor="rule-port">
                  <Input
                    id="rule-port"
                    aria-label="端口"
                    inputMode="numeric"
                    placeholder="请输入端口，如 443"
                    value={draft.port}
                    onChange={(event) =>
                      setDraft((current) => ({ ...current, port: event.target.value }))
                    }
                  />
                  <p className="text-xs text-muted-foreground">留空表示任意端口</p>
                </Field>
              </div>

              <div className="grid gap-2">
                <Label>
                  动作 <span className="text-rose-400">*</span>
                </Label>
                <div className="grid h-12 max-w-sm grid-cols-2 overflow-hidden rounded-md border border-input">
                  <Button
                    type="button"
                    variant="ghost"
                    className={`h-full rounded-none ${draft.action === "代理" ? "bg-blue-600 text-white hover:bg-blue-600 hover:text-white dark:hover:!bg-blue-600 dark:hover:!text-white" : "text-muted-foreground"}`}
                    onClick={() => setDraft((current) => ({ ...current, action: "代理" }))}
                    aria-pressed={draft.action === "代理"}
                  >
                    <GitBranch className="size-4" aria-hidden="true" />
                    代理
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    className={`h-full rounded-none border-l border-input ${draft.action === "直连" ? "bg-blue-600 text-white hover:bg-blue-600 hover:text-white dark:hover:!bg-blue-600 dark:hover:!text-white" : "text-muted-foreground"}`}
                    onClick={() => setDraft((current) => ({ ...current, action: "直连" }))}
                    aria-pressed={draft.action === "直连"}
                  >
                    直连
                  </Button>
                </div>
              </div>

              <Field label="备注" htmlFor="rule-remark">
                <Textarea
                  id="rule-remark"
                  aria-label="备注"
                  rows={3}
                  placeholder="请输入备注（可选）"
                  className="min-h-24 resize-y"
                  value={draft.remark}
                  onChange={(event) =>
                    setDraft((current) => ({ ...current, remark: event.target.value }))
                  }
                />
              </Field>
            </div>

            <DialogFooter className="border-t border-white/10 px-6 py-5 sm:px-7">
              <DialogClose asChild>
                <Button type="button" variant="secondary" className="min-w-28">
                  取消
                </Button>
              </DialogClose>
              <Button type="submit" className="min-w-28">
                保存
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  );
}
