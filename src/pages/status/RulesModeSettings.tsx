import { useEffect, useRef, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ErrorAlert } from "@/components/ErrorAlert";
import { normalizeError } from "@/lib/error-handler";
import type { AppError, RuntimeMode } from "@/lib/generated/ipc";
import { useBackendStore } from "@/store/backend-store";

export type RulesMode = Extract<RuntimeMode, { rules: unknown }>;
type Parameters = RulesMode["rules"];

export function RulesModeSettings({ mode }: { mode: RulesMode }) {
  const switchMode = useBackendStore((state) => state.switchMode);
  const [draft, setDraft] = useState<Parameters | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const lifecycle = useRef<{
    timer: ReturnType<typeof setTimeout> | null;
    version: number;
    active: boolean;
  }>({ timer: null, version: 0, active: false });
  const parameters = draft ?? mode.rules;

  useEffect(() => {
    const current = lifecycle.current;
    current.active = true;
    return () => {
      current.active = false;
      current.version++;
      if (current.timer !== null) clearTimeout(current.timer);
    };
  }, []);

  function change(next: Parameters) {
    const current = lifecycle.current;
    setDraft(next);
    setError(null);
    const request = ++current.version;
    if (current.timer !== null) clearTimeout(current.timer);
    current.timer = setTimeout(() => {
      current.timer = null;
      void (async () => {
        try {
          await switchMode({ rules: next });
        } catch (reason) {
          if (current.active && request === current.version) setError(normalizeError(reason));
        } finally {
          if (current.active && request === current.version) setDraft(null);
        }
      })();
    }, 350);
  }

  return (
    <Card className="sm:mx-auto sm:max-w-2xl">
      <CardHeader>
        <CardTitle>规则代理配置</CardTitle>
      </CardHeader>
      <CardContent className="space-y-5">
        <div className="flex items-start justify-between gap-4">
          <div className="space-y-1">
            <Label htmlFor="rules-china-direct">国内直连</Label>
            <p id="rules-china-help" className="text-sm text-muted-foreground">
              优先应用用户规则，再让中国域名和中国或私有字面 IP 直连。默认关闭。
            </p>
          </div>
          <Switch
            id="rules-china-direct"
            aria-describedby="rules-china-help"
            checked={parameters.use_china_direct}
            onCheckedChange={(use_china_direct) => change({ ...parameters, use_china_direct })}
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="rules-default-action">未匹配流量</Label>
          <Select
            value={parameters.default_action}
            onValueChange={(value) => {
              if (value === "proxy" || value === "direct")
                change({ ...parameters, default_action: value });
            }}
          >
            <SelectTrigger id="rules-default-action" aria-describedby="rules-default-help">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="proxy">走代理</SelectItem>
              <SelectItem value="direct">直连</SelectItem>
            </SelectContent>
          </Select>
          <p id="rules-default-help" className="text-sm text-muted-foreground">
            未命中用户规则或国内直连时应用此动作。默认走代理，启动时需要默认代理档案。
          </p>
        </div>
        <p className="text-xs text-muted-foreground" role="status">
          {draft ? "正在保存配置…" : "配置已同步"}
        </p>
        {error && <ErrorAlert error={error} />}
      </CardContent>
    </Card>
  );
}
