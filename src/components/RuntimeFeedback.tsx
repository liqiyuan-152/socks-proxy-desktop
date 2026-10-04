import { ipc } from "@/lib/ipc";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { errorMessage } from "@/lib/backend";
import { useBackend } from "@/lib/backend-state";
import { proxyModes } from "@/lib/proxy-mode";

export function RuntimeFeedback() {
  const { snapshot, error, capabilities, pending, switchMode, refresh } = useBackend();
  const [recovering, setRecovering] = useState(false);
  const [recoveryError, setRecoveryError] = useState<string | null>(null);
  const operationError = snapshot?.last_operation.error;
  const healthError = snapshot?.last_error;
  const errors = [...new Set([error, operationError, healthError, recoveryError].filter(Boolean))];
  const recoveryRequired = snapshot?.session_health === "recovery_required";
  if (!errors.length && !recoveryRequired) return null;

  async function recover() {
    setRecovering(true);
    setRecoveryError(null);
    try {
      await ipc("recover_network", { confirmed: true });
      await refresh();
    } catch (reason) {
      setRecoveryError(errorMessage(reason));
    } finally {
      setRecovering(false);
    }
  }

  return (
    <section aria-label="运行时反馈" className="space-y-2 rounded-lg border border-border p-4">
      <div role="alert" className="space-y-1 text-sm text-destructive">
        {errors.map((message) => (
          <p key={message}>{message}</p>
        ))}
        {recoveryRequired && <p>恢复未完成，请检查 Windows 系统代理设置；应用不会覆盖外部修改。</p>}
      </div>
      <p className="text-sm text-muted-foreground">
        当前仍生效的模式：
        {snapshot?.applied_mode ? proxyModes[snapshot.applied_mode].label : "未应用"}
        {snapshot?.session_health === "healthy" && "，原内核仍在运行。"}
      </p>
      <div className="flex gap-2">
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
