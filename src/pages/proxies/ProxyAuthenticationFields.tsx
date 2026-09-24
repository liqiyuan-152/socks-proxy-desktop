import { useState, type Dispatch, type SetStateAction } from "react";
import { Eye, EyeOff } from "lucide-react";
import { Field } from "@/components/forms/Field";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";

export type ProxyDraft = {
  name: string;
  protocol: string;
  server: string;
  port: string;
  authentication: boolean;
  username: string;
  password: string;
};

export function ProxyAuthenticationFields({
  draft,
  setDraft,
}: {
  draft: ProxyDraft;
  setDraft: Dispatch<SetStateAction<ProxyDraft>>;
}) {
  const [passwordVisible, setPasswordVisible] = useState(false);
  return (
    <div className="border-t border-white/10 pt-5">
      <div className="flex items-center gap-4">
        <Label htmlFor="proxy-authentication">启用认证</Label>
        <Switch
          id="proxy-authentication"
          checked={draft.authentication}
          onCheckedChange={(authentication) =>
            setDraft((current) => ({ ...current, authentication }))
          }
        />
      </div>
      {draft.authentication && (
        <div className="mt-5 grid gap-5 sm:grid-cols-2">
          <Field label="用户名" htmlFor="proxy-username">
            <Input
              id="proxy-username"
              placeholder="请输入用户名"
              value={draft.username}
              onChange={(event) =>
                setDraft((current) => ({ ...current, username: event.target.value }))
              }
            />
          </Field>
          <Field label="密码" htmlFor="proxy-password">
            <div className="relative">
              <Input
                id="proxy-password"
                type={passwordVisible ? "text" : "password"}
                className="pr-10"
                placeholder="请输入密码"
                value={draft.password}
                onChange={(event) =>
                  setDraft((current) => ({ ...current, password: event.target.value }))
                }
              />
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                className="absolute top-1/2 right-1 -translate-y-1/2 text-muted-foreground"
                aria-label={passwordVisible ? "隐藏密码" : "显示密码"}
                onClick={() => setPasswordVisible((visible) => !visible)}
              >
                {passwordVisible ? (
                  <EyeOff className="size-4" aria-hidden="true" />
                ) : (
                  <Eye className="size-4" aria-hidden="true" />
                )}
              </Button>
            </div>
          </Field>
        </div>
      )}
    </div>
  );
}
