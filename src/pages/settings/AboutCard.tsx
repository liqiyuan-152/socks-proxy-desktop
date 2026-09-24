import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function AboutCard() {
  return (
    <Card className="gap-0 border-white/10 bg-card py-0 shadow-none">
      <CardHeader className="py-5">
        <CardTitle role="heading" aria-level={2}>
          关于
        </CardTitle>
      </CardHeader>
      <CardContent className="grid gap-5 border-t border-white/10 py-5 text-sm sm:grid-cols-[1fr_1fr_auto] sm:items-center">
        <dl className="grid grid-cols-[auto_1fr] gap-x-7 gap-y-3">
          <dt className="text-muted-foreground">版本</dt>
          <dd className="font-semibold">v0.1.0</dd>
          <dt className="text-muted-foreground">内核版本</dt>
          <dd className="font-semibold">目标 v1.14.1（未就绪）</dd>
        </dl>
        <dl className="grid grid-cols-[auto_1fr] gap-x-7 gap-y-3">
          <dt className="text-muted-foreground">许可证</dt>
          <dd className="font-semibold">应用未声明；内核 GPLv3</dd>
        </dl>
        <Button variant="secondary" disabled aria-describedby="update-unavailable">
          检查更新
        </Button>
        <p id="update-unavailable" className="text-xs text-muted-foreground sm:col-start-3">
          尚未配置发布渠道，无法检查更新。
        </p>
      </CardContent>
    </Card>
  );
}
