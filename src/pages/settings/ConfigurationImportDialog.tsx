import { ErrorAlert } from "@/components/ErrorAlert";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import type { useConfigurationImport } from "./useConfigurationImport";

export function ConfigurationImportDialog({
  flow,
}: {
  flow: ReturnType<typeof useConfigurationImport>;
}) {
  return (
    <Dialog open={flow.open} onOpenChange={(open) => !open && flow.close()}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>导入配置</DialogTitle>
        </DialogHeader>
        <p className="text-sm text-muted-foreground">
          导入会校验并替换配置；已认证档案必须重新提供密码。
        </p>
        {flow.error && <ErrorAlert error={flow.error} />}
        <div className="max-h-[50vh] space-y-4 overflow-y-auto">
          {flow.profiles
            .filter((profile) => profile.authentication_enabled)
            .map((profile) => (
              <div key={profile.id} className="space-y-2">
                <p className="font-medium">{profile.name} 的认证凭据</p>
                <Input
                  aria-label={`${profile.name}用户名`}
                  placeholder="用户名"
                  autoComplete="off"
                  disabled={flow.busy}
                  value={flow.credentials[profile.id]?.username ?? ""}
                  onChange={(event) =>
                    flow.updateCredential(profile.id, "username", event.target.value)
                  }
                />
                <Input
                  aria-label={`${profile.name}密码`}
                  placeholder="重新输入密码"
                  type="password"
                  autoComplete="new-password"
                  disabled={flow.busy}
                  value={flow.credentials[profile.id]?.password ?? ""}
                  onChange={(event) =>
                    flow.updateCredential(profile.id, "password", event.target.value)
                  }
                />
              </div>
            ))}
        </div>
        <DialogFooter>
          <Button variant="secondary" disabled={flow.busy} onClick={flow.close}>
            取消
          </Button>
          <Button disabled={flow.busy} onClick={() => void flow.submit()}>
            确认导入
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
