import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function AboutCard() {
  return (
    <Card className="gap-0 border-border bg-card py-0 shadow-none">
      <CardHeader className="py-5">
        <CardTitle role="heading" aria-level={2}>
          关于
        </CardTitle>
      </CardHeader>
      <CardContent className="grid gap-5 border-t border-border py-5 text-sm sm:grid-cols-2">
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
      </CardContent>
    </Card>
  );
}
