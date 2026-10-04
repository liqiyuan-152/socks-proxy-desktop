# 贡献者迁移指南

新代码从 services::ApplicationService 引入应用入口，从 services::interfaces 引入领域接口。过渡期 ConfigurationService 别名和旧模块已移除；Rust 调用者应将旧模块导入替换为 services 下的同名 DTO，并将服务类型替换为 ApplicationService。既有 IPC 命令和配置 v2 保持兼容。

配置修改放到对应领域服务，通过共享 ConfigurationContext 的 mutation_lock 和 commit_durable 提交。不要直接调用 store.save 绕过凭据暂存、恢复意图或运行时确认。跨领域操作经 facade 协调，不能分别创建锁或存储实例来组合事务。

业务失败返回领域错误，转换到 AppError 时保留根因与已生成身份；应用边界补充操作名并记录脱敏诊断。前端使用 normalizeError 接收 unknown，保留完整 AppError 并交给 ErrorAlert。详见 [错误指南](error-handling.md)。

组件从 useBackendStore(selector) 读取共享状态，多个字段使用 useShallow。新增共享读取需使用请求版本以拒绝过期响应；新增写操作需遵循现有刷新与轮询协调。组件局部仍可使用 React state 保存输入与弹窗状态，凭据禁止进入 store 或 devtools。详见 [状态指南](frontend-state.md)。

新 IPC 同时修改 Rust 注册、契约定义和实际 MockRuntime 调用测试，再运行 pnpm ipc:generate；不要手改 src/lib/generated。旧命令签名保持不变，新诊断筛选字段和错误上下文字段均可选。

数据库迁移按版本逐步在事务中执行，保留配置 v2。旧二进制遇到高于支持版本的数据库必须拒绝打开；版本回退需要使用兼容的备份，不能删库或手改 user_version。导出配置不包含凭据，导入仍需要明确校验与事务应用。

追踪 span 使用 skip_all，仅显式记录固定操作、阶段、修订和随机身份；不记录用户输入、地址或错误消息。开发细节使用 debug，默认 info 保留生命周期和最终结果。异步队列可能丢弃日志，诊断快照提供计数；重要业务状态仍由 SQLite 持久化。

提交前按 [测试指南](testing.md) 执行聚焦回归和完整质量检查。新增或实质修改源文件应低于 400 行。该重构仍在验收中，发布前必须完成 Windows 系统代理与原生性能门禁；本文不表示已完成发布。
