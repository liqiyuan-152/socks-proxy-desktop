import type { Dispatch, SetStateAction } from "react";
import { X } from "lucide-react";
import { Field } from "@/components/forms/Field";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
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
import { ProxyAuthenticationFields, type ProxyDraft } from "./ProxyAuthenticationFields";
import { ProxyLinkInput } from "./ProxyLinkInput";

type ProxyFormDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  isEditing: boolean;
  busy: boolean;
  credentialLoading: boolean;
  credentialError: string | null;
  onRetryCredential: () => void;
  draft: ProxyDraft;
  setDraft: Dispatch<SetStateAction<ProxyDraft>>;
  proxyLink: string;
  linkError: string | null;
  onLinkChange: (value: string) => void;
  onParse: () => void;
  onSave: () => void;
};

export function ProxyFormDialog({
  open,
  onOpenChange,
  isEditing,
  busy,
  credentialLoading,
  credentialError,
  onRetryCredential,
  draft,
  setDraft,
  proxyLink,
  linkError,
  onLinkChange,
  onParse,
  onSave,
}: ProxyFormDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[calc(100vh-2rem)] p-0 sm:max-h-[680px]">
        <form
          onSubmit={(event) => {
            event.preventDefault();
            onSave();
          }}
        >
          <DialogHeader className="relative border-b border-border px-6 py-5 sm:px-7">
            <DialogTitle>{isEditing ? "编辑代理" : "添加代理"}</DialogTitle>
            <DialogClose asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                className="absolute top-3 right-4 text-muted-foreground hover:text-foreground"
                aria-label="关闭代理表单"
              >
                <X className="size-5" aria-hidden="true" />
              </Button>
            </DialogClose>
          </DialogHeader>

          <div className="space-y-5 px-6 py-6 sm:px-7">
            {!isEditing && (
              <ProxyLinkInput
                value={proxyLink}
                error={linkError}
                onChange={onLinkChange}
                onParse={onParse}
              />
            )}
            <div className="grid gap-5 sm:grid-cols-[1.1fr_0.9fr]">
              <Field label="名称" required htmlFor="proxy-name">
                <Input
                  id="proxy-name"
                  placeholder="请输入代理名称"
                  value={draft.name}
                  onChange={(event) =>
                    setDraft((current) => ({ ...current, name: event.target.value }))
                  }
                />
              </Field>
              <Field label="协议" required htmlFor="proxy-protocol">
                <Select
                  value={draft.protocol}
                  onValueChange={(protocol) => setDraft((current) => ({ ...current, protocol }))}
                >
                  <SelectTrigger id="proxy-protocol">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="socks5">SOCKS5</SelectItem>
                    <SelectItem value="http">HTTP</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
            </div>

            <div className="grid gap-5 sm:grid-cols-[1.7fr_0.8fr]">
              <Field label="服务器" required htmlFor="proxy-server">
                <Input
                  id="proxy-server"
                  placeholder="例如：proxy.example.com"
                  value={draft.server}
                  onChange={(event) =>
                    setDraft((current) => ({ ...current, server: event.target.value }))
                  }
                />
              </Field>
              <Field label="端口" required htmlFor="proxy-port">
                <Input
                  id="proxy-port"
                  inputMode="numeric"
                  placeholder="例如：1080"
                  value={draft.port}
                  onChange={(event) =>
                    setDraft((current) => ({ ...current, port: event.target.value }))
                  }
                />
              </Field>
            </div>

            <ProxyAuthenticationFields
              draft={draft}
              setDraft={setDraft}
              disabled={credentialLoading || !!credentialError}
            />
            {credentialLoading && (
              <p role="status" className="text-sm text-muted-foreground">
                正在读取认证凭据…
              </p>
            )}
            {credentialError && (
              <div role="alert" className="flex items-center gap-3 text-sm text-destructive">
                <span>读取认证凭据失败：{credentialError}</span>
                <Button type="button" variant="outline" size="sm" onClick={onRetryCredential}>
                  重试
                </Button>
              </div>
            )}
          </div>

          <DialogFooter className="border-t border-border px-6 py-5 sm:px-7">
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
      </DialogContent>
    </Dialog>
  );
}
