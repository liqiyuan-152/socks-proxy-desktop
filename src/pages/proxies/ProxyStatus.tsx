type Props = { active: boolean; enabled: boolean };

export function ProxyStatus({ active, enabled }: Props) {
  const label = active ? "默认代理" : enabled ? "已启用" : "已停用";
  const color = active
    ? "text-emerald-700 dark:text-emerald-400"
    : enabled
      ? "text-blue-700 dark:text-blue-400"
      : "text-destructive";

  return (
    <span className={`inline-flex items-center gap-1.5 text-xs font-medium ${color}`}>
      <span className="size-1.5 shrink-0 rounded-full bg-current" aria-hidden="true" />
      {label}
    </span>
  );
}
