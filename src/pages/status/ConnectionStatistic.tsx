import { type ReactNode } from "react";

import { Card, CardContent } from "@/components/ui/card";

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
            className="shrink-0 rounded-lg bg-primary/10 p-2 text-primary-text ring-1 ring-primary/20"
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
