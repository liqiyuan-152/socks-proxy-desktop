# 错误处理与诊断开发指南

## 服务边界

代理、路由和运行时服务分别返回 `ProxyError`、`RoutingError`、`RuntimeError`。ApplicationService 将其转换为 AppError，并补充错误身份、操作和恢复建议。跨层转换保留已有身份；以稳定错误码分类旧适配器，不能通过中文消息猜测错误类型。

AppError 保留 `code`、`message`、`fields`，可选 `context` 提供 `error_id`、`timestamp_ms`、`domain`、`kind`、`operation`、`recovery_suggestion`、可选 `trace_id`/`span_id`。旧错误没有 context 时保留原有 JSON 形状；现有 IPC 命令参数和配置 v2 文档格式保持兼容。

前端从 `src/lib/generated/ipc.ts` 使用生成类型，运行 `pnpm ipc:generate` 更新；不能手工编辑生成文件。新增命令必须同时更新 Tauri 注册和 IPC 契约，并通过实际 Tauri IPC 测试。

## 界面

捕获值的类型是 unknown。调用 `normalizeError(reason)` 验证其形状后，保存完整 AppError，不要压成字符串或使用 `Partial<BackendError>` 强制转型。

```tsx
const [error, setError] = useState<AppError | null>(null);
try {
  await ipc("delete_profile", { id });
} catch (reason) {
  setError(normalizeError(reason));
}
return error && <ErrorAlert error={error} />;
```

ErrorAlert 展示消息、字段、恢复建议和错误编号。只有明确允许的操作才提供 onRetry 或 onOpenDiagnostics。重试由用户点击发起，并防止并发点击；表单内的错误操作按钮使用 type=button，避免误提交。删除和保存失败应在当前弹窗内可见，并保留输入，不自动重试修改配置或恢复网络。

运行时快照仍使用旧的文本错误字段。同一失败同时来自 IPC 和快照时，界面优先保留完整上下文；显示应用模式和会话健康，避免把目标模式误当成正在生效的模式。React 错误边界只重建界面，不停止原生代理或刷新 WebView。

## 诊断保存、聚合与导出

应用边界记录固定摘要、错误身份、领域变体、操作、恢复建议和脱敏 Rust 符号堆栈。禁止记录用户消息、字段值、输入值、密码和认证信息。堆栈过滤本机路径、地址、超长符号，最多 16 帧；它不进入 AppError 的 IPC 响应，只保存在本地诊断中。诊断写入失败不得覆盖原业务失败。

运行时诊断的 severity 使用 `info`、`warning`、`error`。无效写入或筛选被拒绝。`error_type` 是独立的可选领域变体，如 `proxy.not_found`；`operation` 可选。普通运行事件与旧文本记录允许不具有错误类型。

同一 error_id 重复写入不会新增或覆盖记录。不同错误编号保留独立记录；聚合按 error_type、operation、severity 分组，报告次数、最早与最近时间，不按用户消息聚合，也不删除原始身份。只有带错误类型的记录进入同类错误统计。

- `get_runtime_diagnostics` 保持分页和时间/级别筛选契约。
- `get_diagnostic_groups` 使用相同筛选并返回完整错误分组。
- `export_runtime_diagnostics` 返回当前筛选的完整 JSON Lines 快照，每行一个诊断对象，保留独立编号；空结果为空字符串，不受界面的 100 条分页上限约束。
- `save_runtime_diagnostics` 在原生保存对话框中选择文件位置，Rust 原子写入完整快照；取消时返回 false，不写文件。IPC 不接受任意保存路径，也不向前端开放通用文件系统权限。写入失败保留原文件并返回带身份的存储错误。
- 设置页提供查看、刷新、级别筛选和显式导出，使用系统保存对话框。JSON Lines 文件仍是本地诊断资料，由用户决定是否分享。

导出在同一个 SQLite 读取语句下完成，避免分页之间并发写入造成重复或遗漏。保留策略和清理确认要求继续适用；导出不清理原始记录。

## 数据库迁移与回退

数据库 schema 5 为诊断表新增 error_type 和 operation；旧结构化摘要提取相应元数据，旧文本摘要保持原样。DDL 与数据更新在同一个事务中执行；迁移中断会回滚。配置文档仍为 v2，代理、规则与凭据引用不因此变化。

旧二进制拒绝高于自身支持版本的数据库，这是已有版本保护。回退前应保存对应数据库及受保护凭据备份；仅替换二进制或导出不含密码的配置不等于可安全回退。不能绕过数据库版本保护。

## 验证

运行 `pnpm check`、`pnpm build` 和完整 Rust 测试。Windows 真实内核测试须校验固定 sing-box 版本与 SHA-256；测试通过数量本身不证明真实系统代理接管。迁移、去重、不同操作隔离、筛选、超过 100 条的 JSON Lines 导出、错误身份与隐私、实际 IPC、界面导出和重复点击均需覆盖。原生界面验收和启动/IPC 性能门禁需单独收集证据。
