import { type Dispatch, type SetStateAction } from "react";
import { GitBranch, X } from "lucide-react";
import { Field } from "@/components/forms/Field";
import { Button } from "@/components/ui/button";
import { DialogClose, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

export type RuleDraft = {
  name: string;
  targetType: string;
  target: string;
  port: string;
  action: "代理" | "直连";
};

export function RuleForm({
  draft,
  setDraft,
  isEditing,
  busy,
  onSave,
}: {
  draft: RuleDraft;
  setDraft: Dispatch<SetStateAction<RuleDraft>>;
  isEditing: boolean;
  busy: boolean;
  onSave: () => void;
}) {
  return (
    <form
      onSubmit={(event) => {
        event.preventDefault();
        onSave();
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
              onValueChange={(targetType) => setDraft((current) => ({ ...current, targetType }))}
            >
              <SelectTrigger id="rule-target-type" aria-label="匹配类型">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="domain">域名</SelectItem>
                <SelectItem value="domain_suffix">域名后缀</SelectItem>
                <SelectItem value="ip_cidr">CIDR</SelectItem>
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
              域名输入 example.com；域名后缀输入 example.com；CIDR 输入 192.168.0.0/16。
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
      </div>
      <DialogFooter className="border-t border-white/10 px-6 py-5 sm:px-7">
        <DialogClose asChild>
          <Button type="button" variant="secondary" className="min-w-28">
            取消
          </Button>
        </DialogClose>
        <Button type="submit" className="min-w-28" disabled={busy}>
          保存
        </Button>
      </DialogFooter>
    </form>
  );
}
