import { useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { normalizeError } from "@/lib/error-handler";
import type { AppError } from "@/lib/generated/ipc";
import { ErrorAlert } from "./ErrorAlert";

interface Props {
  name: string;
  resource: "代理" | "规则";
  error: AppError | string | null;
  onCancel: () => void;
  onConfirm: () => Promise<boolean>;
}

export function ConfirmDeletionDialog({ name, resource, error, onCancel, onConfirm }: Props) {
  const [pending, setPending] = useState(false);
  const [localError, setLocalError] = useState<AppError | null>(null);
  const submitting = useRef(false);
  const cancelButton = useRef<HTMLButtonElement>(null);

  async function confirm() {
    if (submitting.current) return;
    submitting.current = true;
    setPending(true);
    setLocalError(null);
    try {
      if (await onConfirm()) onCancel();
    } catch (reason) {
      setLocalError(normalizeError(reason));
    } finally {
      submitting.current = false;
      setPending(false);
    }
  }

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !submitting.current) onCancel();
      }}
    >
      <DialogContent
        role="alertdialog"
        className="gap-5 p-6 sm:max-w-md"
        onOpenAutoFocus={(event) => {
          event.preventDefault();
          cancelButton.current?.focus();
        }}
      >
        <DialogHeader>
          <DialogTitle>确认删除{resource}</DialogTitle>
          <DialogDescription>
            确认删除{resource}「{name}」？此操作无法撤销。
          </DialogDescription>
        </DialogHeader>
        {(error || localError) && (
          <ErrorAlert
            error={typeof error === "string" ? { message: error } : error || localError}
          />
        )}
        <DialogFooter>
          <Button ref={cancelButton} variant="outline" disabled={pending} onClick={onCancel}>
            取消
          </Button>
          <Button variant="destructive" disabled={pending} onClick={() => void confirm()}>
            {pending ? "正在删除…" : "确认删除"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
