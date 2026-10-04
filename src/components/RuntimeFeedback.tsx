import { ipc } from "@/lib/ipc";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { normalizeError } from "@/lib/error-handler";
import type { AppError } from "@/lib/generated/ipc";
import { ErrorAlert } from "./ErrorAlert";
import { useNavigate } from "react-router-dom";
import { useBackendStore } from "@/store/backend-store";
import { useShallow } from "zustand/react/shallow";
import { proxyModes } from "@/lib/proxy-mode";

export function RuntimeFeedback() {
  const { snapshot, error, capabilities, pending, switchMode, refresh } = useBackendStore(
    useShallow((state) => ({
      snapshot: state.snapshot,
      error: state.error,
      capabilities: state.capabilities,
      pending: state.pending,
      switchMode: state.switchMode,
      refresh: state.refresh,
    })),
  );
  const [recovering, setRecovering] = useState(false);
  const [recoveryError, setRecoveryError] = useState<AppError | null>(null);
  const operationError = snapshot?.last_operation.error;
  const healthError = snapshot?.last_error;
  const navigate = useNavigate();
  const candidates = [error, recoveryError, operationError, healthError]
    .filter((reason) => reason != null)
    .map((reason) => normalizeError(typeof reason === "string" ? { message: reason } : reason));
  const errors = [
    ...new Map([...candidates].reverse().map((reason) => [reason.message, reason])).values(),
  ];
  const recoveryRequired = snapshot?.session_health === "recovery_required";
  if (!errors.length && !recoveryRequired) return null;

  async function recover() {
    setRecovering(true);
    setRecoveryError(null);
    try {
      await ipc("recover_network", { confirmed: true });
      await refresh();
    } catch (reason) {
      setRecoveryError(normalizeError(reason));
    } finally {
      setRecovering(false);
    }
  }

  return (
    <section
      aria-label="运行时反馈"
      className="animate-fade-in space-y-3 rounded-xl border-2 border-destructive/40 bg-gradient-to-br from-destructive/5 to-destructive/10 p-5 shadow-sm shadow-destructive/10"
    >
      <div className="space-y-1 text-sm text-destructive">
        {errors.map((reason) => (
          <ErrorAlert
            key={reason.context?.error_id ?? reason.message}
            error={reason}
            onOpenDiagnostics={() => navigate("/logs")}
          />
        ))}
        {recoveryRequired && (
          <p role="alert">恢复未完成，请检查 Windows 系统代理设置；应用不会覆盖外部修改。</p>
        )}
      </div>
      <p className="text-sm text-muted-foreground">
        当前仍生效的模式：
        {snapshot?.applied_mode ? proxyModes[snapshot.applied_mode].label : "未应用"}
        {snapshot?.session_health === "healthy" && "，原内核仍在运行。"}
      </p>
      <div className="flex flex-wrap gap-2 pt-1">
        {capabilities?.proxy_runtime && snapshot && !recoveryRequired && (
          <Button
            variant="outline"
            size="sm"
            disabled={pending || recovering}
            onClick={() => void switchMode(snapshot.desired_mode)}
          >
            重试模式切换
          </Button>
        )}
        {capabilities?.network_recovery && (
          <Button
            variant="outline"
            size="sm"
            disabled={pending || recovering}
            onClick={() => void recover()}
          >
            {recovering ? "正在恢复…" : "恢复系统代理"}
          </Button>
        )}
      </div>
    </section>
  );
}
