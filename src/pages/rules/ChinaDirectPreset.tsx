import { ErrorAlert } from "@/components/ErrorAlert";
import { normalizeError } from "@/lib/error-handler";
import type { AppError, ChinaDirectStatus } from "@/lib/generated/ipc";
import { ipc } from "@/lib/ipc";
import { useEffect, useState } from "react";
import { Link } from "react-router-dom";

export function ChinaDirectPreset() {
  const [status, setStatus] = useState<ChinaDirectStatus | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  useEffect(() => {
    let active = true;
    void ipc("get_china_direct_status")
      .then((value) => {
        if (active) setStatus(value);
      })
      .catch((reason: unknown) => {
        if (active) setError(normalizeError(reason));
      });
    return () => {
      active = false;
    };
  }, []);
  return (
    <section className="space-y-2 border-b border-border pb-5" aria-labelledby="china-direct-title">
      <h2 id="china-direct-title" className="text-base font-semibold">
        国内直连
      </h2>
      <p className="text-sm text-muted-foreground">
        国内直连属于规则代理参数；用户规则优先，其余流量按配置的默认动作处理。
      </p>
      <p className="text-xs text-muted-foreground">
        域名集外的域名不按解析 IP 分流；路由预测不代表实际联网结果。
        {status?.data_date ? ` 数据版本：${status.data_date}。` : ""}
        {status && !status.available ? " 本地规则集不可用。" : ""}
      </p>
      <Link to="/" className="text-sm text-primary underline underline-offset-4">
        在状态页配置规则代理
      </Link>
      {error && <ErrorAlert error={error} />}
    </section>
  );
}
