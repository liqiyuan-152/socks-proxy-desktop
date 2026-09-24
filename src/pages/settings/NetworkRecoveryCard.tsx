import { RotateCcw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function NetworkRecoveryCard({
  available,
  busy,
  onRestore,
}: {
  available: boolean;
  busy: boolean;
  onRestore: () => void;
}) {
  return (
    <Card className="gap-0 border-white/10 bg-card py-0 shadow-none">
      <CardHeader className="py-5">
        <CardTitle role="heading" aria-level={2}>
          网络恢复
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4 border-t border-white/10 py-5">
        <Button
          variant="secondary"
          disabled={!available || busy}
          onClick={onRestore}
          aria-describedby={available ? undefined : "restore-unavailable"}
        >
          <RotateCcw className="size-4" aria-hidden="true" />
          恢复网络设置
        </Button>
        {!available && (
          <p id="restore-unavailable" className="text-sm text-muted-foreground">
            仅 Windows 桌面应用支持管理和恢复用户级系统代理。
          </p>
        )}
      </CardContent>
    </Card>
  );
}
