import { Badge } from "@/components/ui/badge";

type Props = { active: boolean; enabled: boolean };

export function ProxyStatus({ active, enabled }: Props) {
  const label = active ? "默认代理" : enabled ? "已启用" : "已停用";
  return (
    <Badge variant={enabled ? "success" : "secondary"} className="gap-1.5">
      <span className="size-1.5 shrink-0 rounded-full bg-current" aria-hidden="true" />
      {label}
    </Badge>
  );
}
