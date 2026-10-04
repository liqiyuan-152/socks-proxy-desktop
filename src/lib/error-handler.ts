import type { AppError, ErrorContext, ErrorDomain, FieldError } from "./generated/ipc";

const fallbackMessage = "操作失败，请检查运行时状态。";
const domains: ReadonlySet<ErrorDomain> = new Set([
  "proxy",
  "routing",
  "runtime",
  "storage",
  "credential",
  "validation",
  "application",
]);

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isFieldError(value: unknown): value is FieldError {
  return record(value) && typeof value.field === "string" && typeof value.message === "string";
}

function context(value: unknown): ErrorContext | undefined {
  if (
    !record(value) ||
    typeof value.error_id !== "string" ||
    typeof value.timestamp_ms !== "number" ||
    !Number.isFinite(value.timestamp_ms) ||
    value.timestamp_ms < 0 ||
    typeof value.domain !== "string" ||
    !domains.has(value.domain as ErrorDomain) ||
    typeof value.kind !== "string" ||
    typeof value.recovery_suggestion !== "string"
  )
    return undefined;
  return {
    error_id: value.error_id,
    timestamp_ms: value.timestamp_ms,
    domain: value.domain as ErrorDomain,
    kind: value.kind,
    recovery_suggestion: value.recovery_suggestion,
    ...(typeof value.operation === "string" ? { operation: value.operation } : {}),
  };
}

/** Normalize rejected IPC values and JavaScript errors without trusting their shape. */
export function normalizeError(reason: unknown): AppError {
  if (!record(reason)) {
    return { code: "unknown_error", message: fallbackMessage, fields: [] };
  }
  const details = context(reason.context);
  return {
    code: typeof reason.code === "string" ? reason.code : "unknown_error",
    message: typeof reason.message === "string" ? reason.message : fallbackMessage,
    fields: Array.isArray(reason.fields)
      ? reason.fields.filter(isFieldError).map(({ field, message }) => ({ field, message }))
      : [],
    ...(details ? { context: details } : {}),
  };
}

/** Preserve the existing concise message helper while all callers migrate. */
export function errorMessage(reason: unknown): string {
  return normalizeError(reason).message;
}

/** Field-level validation failures can be displayed without unsafe casts. */
export function errorDetails(reason: unknown): string {
  const error = normalizeError(reason);
  return (
    error.fields.map(({ field, message }) => `${field}: ${message}`).join("；") || error.message
  );
}

/** Older backends still receive useful suggestions while new contexts are authoritative. */
export function recoverySuggestion(reason: unknown): string {
  const error = normalizeError(reason);
  if (error.context?.recovery_suggestion) return error.context.recovery_suggestion;
  switch (error.code) {
    case "proxy_not_found":
    case "not_found":
      return "请刷新代理列表并重新选择代理。";
    case "proxy_in_use":
      return "请先取消默认代理选择，并修改引用此代理的规则。";
    case "credential_error":
      return "请检查代理认证设置并重新填写凭据。";
    case "validation_error":
      return "请检查标记的字段，修正后重试。";
    case "configuration_recovery":
    case "rollback_failed":
    case "system_proxy_failed":
    case "runtime_invariant_violated":
      return "请停止代理，检查运行时状态，并执行网络恢复；保留原有配置。";
    case "storage_error":
      return "请确认本地存储可写并有足够空间；保留数据并查看诊断信息。";
    default:
      return "请刷新状态后重试；若仍失败，请查看诊断信息。";
  }
}
