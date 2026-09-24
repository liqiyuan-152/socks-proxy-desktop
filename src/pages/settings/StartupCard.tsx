import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";

export function StartupCard({
  enabled,
  loaded,
  busy,
  onChange,
}: {
  enabled: boolean;
  loaded: boolean;
  busy: boolean;
  onChange: (enabled: boolean) => void;
}) {
  return (
    <Card className="gap-0 border-border bg-card py-0 shadow-none">
      <CardHeader className="py-5">
        <CardTitle role="heading" aria-level={2}>
          启动
        </CardTitle>
      </CardHeader>
      <CardContent className="flex items-center justify-between gap-5 border-t border-border py-5">
        <div>
          <p className="font-medium">开机启动</p>
          <p className="mt-1 text-sm text-muted-foreground">登录系统后自动启动 Socks Proxy</p>
        </div>
        <Switch
          checked={enabled}
          disabled={!loaded || busy}
          onCheckedChange={onChange}
          aria-label="开机启动"
        />
      </CardContent>
    </Card>
  );
}
