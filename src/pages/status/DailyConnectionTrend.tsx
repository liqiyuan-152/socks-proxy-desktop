import { ArrowDownRight, ArrowUpRight, Minus } from "lucide-react";
import type { ConnectionTrend } from "@/lib/generated/ipc";

export function DailyConnectionTrend({
  count,
  trend,
}: {
  count: number | null;
  trend: ConnectionTrend | null;
}) {
  if (count === null || !trend || trend.yesterday_samples === 0) {
    return (
      <p className="mt-2 text-xs text-muted-foreground">
        {count === null ? "当前连接观测不可用" : "暂无昨日采样"}
      </p>
    );
  }
  const average = trend.yesterday_count_sum / trend.yesterday_samples;
  const delta = count - average;
  const Icon = delta > 0 ? ArrowUpRight : delta < 0 ? ArrowDownRight : Minus;
  const change =
    average === 0
      ? count === 0
        ? "持平"
        : `增加 ${count} 个（昨日为零）`
      : `${delta > 0 ? "+" : ""}${((delta / average) * 100).toFixed(1)}%`;
  return (
    <div className="mt-2 space-y-1 text-xs text-muted-foreground">
      <p className="flex flex-wrap items-center gap-1">
        <Icon className="size-3.5" aria-hidden="true" />
        比昨日均值 <span className="font-medium text-foreground">{change}</span>
      </p>
      <p>
        {trend.yesterday_date} · 均值 {average.toFixed(1)} · {trend.yesterday_samples} 次采样
      </p>
      <p>仅统计应用运行期间的有效观测</p>
    </div>
  );
}
