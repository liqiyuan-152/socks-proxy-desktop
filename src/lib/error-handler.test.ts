import { expect, it } from "vitest";
import { errorDetails, errorMessage, normalizeError, recoverySuggestion } from "./error-handler";

it("keeps backend fields and metadata while excluding unexpected private values", () => {
  const error = normalizeError({
    code: "proxy_in_use",
    message: "代理被引用",
    fields: [{ field: "rules[0]", message: "规则仍引用" }],
    password: "secret",
    context: {
      error_id: "error-1",
      timestamp_ms: 42,
      domain: "proxy",
      kind: "in_use",
      operation: "delete_profile",
      recovery_suggestion: "请修改引用规则",
      stack: ["private-path"],
    },
  });
  expect(error.context).toEqual({
    error_id: "error-1",
    timestamp_ms: 42,
    domain: "proxy",
    kind: "in_use",
    operation: "delete_profile",
    recovery_suggestion: "请修改引用规则",
  });
  expect(errorDetails(error)).toBe("rules[0]: 规则仍引用");
  expect(recoverySuggestion(error)).toBe("请修改引用规则");
  expect(JSON.stringify(error)).not.toMatch(/secret|private-path/);
});

it.each([null, undefined, "failure", 42, [], { message: 42, fields: "invalid" }])(
  "handles malformed errors: %j",
  (reason) => {
    expect(errorMessage(reason)).toBe("操作失败，请检查运行时状态。");
    expect(normalizeError(reason)).toEqual({
      code: "unknown_error",
      message: "操作失败，请检查运行时状态。",
      fields: [],
    });
  },
);

it("accepts ordinary JavaScript errors and legacy backend errors", () => {
  expect(errorMessage(new Error("请求失败"))).toBe("请求失败");
  const legacy = {
    code: "validation_error",
    message: "配置无效",
    fields: [null, {}, { field: "port", message: "无效端口" }],
  };
  expect(normalizeError(legacy).fields).toEqual([{ field: "port", message: "无效端口" }]);
  expect(errorDetails(legacy)).toBe("port: 无效端口");
  expect(recoverySuggestion(legacy)).toContain("字段");
  expect(errorDetails(new Error("请求失败"))).toBe("请求失败");
});

it.each([
  { domain: "unknown" },
  { timestamp_ms: NaN },
  { timestamp_ms: -1 },
  { timestamp_ms: Infinity },
  { kind: null },
  { error_id: 42 },
  { recovery_suggestion: null },
])("drops invalid contexts: %j", (invalid) => {
  const context = {
    error_id: "error-1",
    timestamp_ms: 42,
    domain: "proxy",
    kind: "not_found",
    recovery_suggestion: "刷新",
    ...invalid,
  };
  expect(
    normalizeError({ code: "proxy_not_found", message: "缺失", context }).context,
  ).toBeUndefined();
  expect(recoverySuggestion({ code: "proxy_not_found", context })).toContain("刷新");
});

it.each(["proxy_in_use", "credential_error", "configuration_recovery", "storage_error", "unknown"])(
  "provides legacy recovery suggestions: %s",
  (code) => {
    expect(recoverySuggestion({ code })).not.toBe("");
  },
);
