import { useState, type ReactNode } from "react";
import { ArrowDownRight, ArrowUpRight, Minus } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";

/** Compare real observations in this mounted view; never invent historical totals. */
export function ActiveConnectionTrend({ count }: { count: number | null }) {
  const [sample, setSample] = useState<{ count: number | null; previous: number | null }>({
    count,
    previous: null,
  });
  if (sample.count !== count) {
    setSample({ count, previous: count === null ? null : sample.count });
  }
  if (count === null || sample.previous === null) {
    return (
      <p className="mt-2 text-xs text-muted-foreground">
        {count === null ? "暂无可比较的采样" : "等待下一次连接数变化"}
      </p>
    );
  }
  const delta = count - sample.previous;
  const Icon = delta > 0 ? ArrowUpRight : delta < 0 ? ArrowDownRight : Minus;
  const change =
    sample.previous === 0
      ? `新增 ${delta} 个`
      : `${delta > 0 ? "+" : ""}${((delta / sample.previous) * 100).toFixed(1)}%`;
  return (
    <p className="mt-2 flex flex-wrap items-center gap-1 text-xs text-muted-foreground">
      <Icon className="size-3.5" aria-hidden="true" />
      较上次变化 <span className="font-medium text-foreground">{change}</span>
    </p>
  );
}

export function ConnectionStatistic({
  label,
  value,
  icon,
  children,
}: {
  label: string;
  value: string;
  icon: ReactNode;
  children?: ReactNode;
}) {
  return (
    <Card variant="elevated" className="gap-0 py-5">
      <CardContent>
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <p className="text-sm text-muted-foreground">{label}</p>
            <p className="mt-2 text-2xl font-bold tracking-tight tabular-nums">{value}</p>
          </div>
          <span
            className="shrink-0 rounded-lg bg-primary/10 p-2 text-primary ring-1 ring-primary/20"
            aria-hidden="true"
          >
            {icon}
          </span>
        </div>
        {children}
      </CardContent>
    </Card>
  );
}
