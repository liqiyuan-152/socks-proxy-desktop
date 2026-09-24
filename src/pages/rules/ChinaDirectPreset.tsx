import { useEffect, useState } from "react";
import { Switch } from "@/components/ui/switch";
import { command, errorMessage } from "@/lib/backend";
import { useBackend } from "@/lib/backend-context";

type ChinaDirectStatus = {
  enabled: boolean;
  available: boolean;
  data_date: string | null;
};

export function ChinaDirectPreset() {
  const { snapshot, refresh } = useBackend();
  const [status, setStatus] = useState<ChinaDirectStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const defaultAvailable = Boolean(snapshot?.active_profile_id);

  useEffect(() => {
    let active = true;
    void command<ChinaDirectStatus>("get_china_direct_status")
      .then((value) => {
        if (active) setStatus(value);
      })
      .catch((reason: unknown) => {
        if (active) setError(errorMessage(reason));
      });
    return () => {
      active = false;
    };
  }, []);

  async function change(enabled: boolean) {
    setBusy(true);
    setError(null);
    try {
      setStatus(await command<ChinaDirectStatus>("set_china_direct_enabled", { enabled }));
      await refresh();
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="border-b border-border pb-5" aria-labelledby="china-direct-title">
      <div className="flex items-center justify-between gap-4">
        <div>
          <h2 id="china-direct-title" className="text-base font-semibold">
            国内直连
          </h2>
          <p id="china-direct-description" className="mt-1 text-sm text-muted-foreground">
            用户规则优先；命中中国域名集的域名、国内或私有的字面 IP 直连，其余走默认代理。
          </p>
        </div>
        <Switch
          aria-label="国内直连"
          aria-describedby="china-direct-description"
          checked={status?.enabled ?? false}
          disabled={
            !status || busy || (!status.enabled && (!status.available || !defaultAvailable))
          }
          onCheckedChange={(enabled) => void change(enabled)}
        />
      </div>
      <p className="mt-2 text-xs text-muted-foreground">
        域名集外的域名不按解析 IP 分流；仅覆盖进入本地代理的流量，IP 归属不等于 GFW 可达性。
        {status?.data_date ? ` 数据版本：${status.data_date}。` : ""}
        {!defaultAvailable ? " 请先设置默认代理。" : ""}
        {status && !status.available ? " 本地规则集不可用。" : ""}
      </p>
      {error && (
        <p role="alert" className="mt-2 text-sm text-destructive">
          {error}
        </p>
      )}
    </section>
  );
}
