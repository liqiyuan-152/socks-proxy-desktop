import { useRef, useState } from "react";
import { AlertCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { normalizeError, recoverySuggestion } from "@/lib/error-handler";

interface Props {
  error: unknown;
  title?: string;
  onRetry?: () => void | Promise<void>;
  onOpenDiagnostics?: () => void;
}

/** Display typed failures and legacy errors with explicit, user-initiated actions. */
export function ErrorAlert({ error, title = "操作未完成", onRetry, onOpenDiagnostics }: Props) {
  const [pending, setPending] = useState(false);
  const [retryError, setRetryError] = useState<{ original: unknown; reason: unknown } | null>(null);
  const submitting = useRef(false);
  const normalized = normalizeError(
    retryError && retryError.original === error ? retryError.reason : error,
  );

  const fields = [
    ...new Map(
      normalized.fields.map((item) => [JSON.stringify([item.field, item.message]), item]),
    ).values(),
  ];

  async function retry() {
    if (!onRetry || submitting.current) return;
    submitting.current = true;
    setPending(true);
    setRetryError(null);
    try {
      await onRetry();
    } catch (reason) {
      setRetryError({ original: error, reason });
    } finally {
      submitting.current = false;
      setPending(false);
    }
  }

  return (
    <Card role="alert" className="gap-3 border-destructive/40 p-4">
      <div className="flex items-center gap-2 font-medium text-destructive">
        <AlertCircle className="size-4 shrink-0" aria-hidden="true" />
        <span>{title}</span>
      </div>
      <p className="text-sm text-foreground">{normalized.message}</p>
      {fields.length > 0 && (
        <ul className="list-inside list-disc text-sm text-muted-foreground">
          {fields.map(({ field, message }) => (
            <li key={JSON.stringify([field, message])}>
              {field}: {message}
            </li>
          ))}
        </ul>
      )}
      <p className="text-sm text-muted-foreground">{recoverySuggestion(normalized)}</p>
      {normalized.context && (
        <p className="break-all text-xs text-muted-foreground">
          错误编号：{normalized.context.error_id}
        </p>
      )}
      {(onRetry || onOpenDiagnostics) && (
        <div className="flex flex-wrap gap-2">
          {onRetry && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={pending}
              onClick={() => void retry()}
            >
              {pending ? "正在重试…" : "重试"}
            </Button>
          )}
          {onOpenDiagnostics && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={pending}
              onClick={onOpenDiagnostics}
            >
              查看诊断
            </Button>
          )}
        </div>
      )}
    </Card>
  );
}
