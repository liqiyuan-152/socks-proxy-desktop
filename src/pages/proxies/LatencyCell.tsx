type Props = {
  result?: { latency?: number; error?: string; at?: number; pending?: boolean };
};

export function LatencyCell({ result }: Props) {
  const label = result?.pending
    ? "测试中…"
    : result?.error
      ? "失败"
      : result?.latency !== undefined
        ? `${result.latency} ms`
        : "—";
  const detail =
    result?.at !== undefined
      ? `${result.error ?? "测试完成"} · ${new Date(result.at).toLocaleString()}`
      : undefined;
  const color = result?.error
    ? "text-destructive"
    : result?.latency === undefined
      ? "text-muted-foreground"
      : result.latency <= 200
        ? "text-emerald-700 dark:text-emerald-400"
        : result.latency <= 500
          ? "text-amber-700 dark:text-amber-400"
          : "text-red-700 dark:text-red-400";

  return (
    <span
      className={`block truncate text-xs sm:text-sm ${color}`}
      title={detail}
      aria-live="polite"
    >
      {label}
    </span>
  );
}
