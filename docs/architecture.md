# 服务架构

Tauri IPC 接收命令并从应用状态获取 ApplicationService。它作为统一 facade 委托代理、路由和运行时领域服务。IPC 契约统一维护并生成前端类型；模式参数及状态中的 RuntimeMode 使用相同结构。服务接口见 src-tauri/src/services/interfaces.rs，依赖适配器见 services/adapters.rs；业务服务不依赖 Tauri 窗口。

| 边界                 | 负责内容                                       | 错误类型     |
| -------------------- | ---------------------------------------------- | ------------ |
| ProxyProfileService  | 档案校验、认证凭据暂存、删除引用保护           | ProxyError   |
| RoutingService       | 规则完整性、顺序、路由预测、国内直连           | RoutingError |
| RuntimeService       | 模式、停止、网络恢复、运行时快照               | RuntimeError |
| ApplicationService   | IPC 委托、配置传输、健康检查、统一错误与诊断   | AppError     |
| ConfigurationContext | 共享串行锁、恢复门禁、持久化事务与适配器所有权 | AppError     |

各领域服务共享同一 ConfigurationContext。配置修改先获得串行锁并检查待恢复事务，再校验候选配置；恢复意图必须先于凭据、启动项或运行时的外部修改写入。SQLite 原子提交标记决定重启后的方向：未提交恢复旧配置，已提交完成新配置的清理。失败不能误删仍被健康会话使用的凭据。

ManagedRuntime 执行后端 I/O，再把结果送到纯 RuntimeStateNode 状态机。候选会话和已应用会话分离；配置事务持有完整旧节点，失败恢复旧节点，无法恢复则撤回健康声明。状态节点生成快照，最近 128 次脱敏转换用于排障。详见 [状态机指南](runtime-state-machine.md)。

前端 BackendProvider 为每个生命周期创建独立 Zustand store，selector 控制订阅范围；请求版本、串行模式切换和窗口可见性管理位于 store。表单草稿和凭据留在局部状态。详见 [前端状态指南](frontend-state.md)。

RuntimeMode 包含 `Rules { use_china_direct, default_action }`、`Global` 和 `Direct`。Rules 按用户规则、可选国内直连、默认动作的顺序确定出口；国内直连关闭时跳过预设，默认动作允许 Proxy 或 Direct。前端新建 Rules 默认使用 false 和 Proxy。参数是配置的一部分，模式修改通过 ConfigurationContext 的串行锁、恢复门禁和 `commit_durable` 提交；不能直接写 store 或只更新运行时状态。失败恢复旧配置和已应用会话，恢复失败则撤回健康声明。

配置文档为 v3，SQLite schema 为 v7。数据库 v6→v7 的事务迁移同时转换配置文档和恢复意图中的 v2 配置：保留旧 Rules 的国内直连开关，默认动作设为 Proxy，Global／Direct 保持原模式，移除全局 `china_direct_enabled`。旧版关闭国内直连时的未匹配流量原本直连，迁移后默认代理；用户可以在状态页改回 Direct。selected_mode 的 rules 参数仅在 Rules 模式有值，Global／Direct 必须为空。历史导入会规范化为 v3；未来版本数据库和配置必须拒绝，不能通过删库或改版本号绕过。

真实代理内核和系统代理接管仍仅支持 Windows。macOS／Linux 的不可用运行时适配器允许配置事务保存模式参数，并保持已应用模式为空、内核未运行；用户规则和关闭国内直连时的默认动作可预测，国内规则集匹配不可用时返回明确错误。配置保存和预测不代表真实代理联网成功。

诊断保留 schema 5 引入的可选错误类型和操作，错误上下文含可选追踪身份；旧纯文本诊断仍可查看。错误身份和恢复建议见 [错误指南](error-handling.md)，日志、指标和只读工具见 [观测指南](observability.md)。

测试通过可注入适配器构建独立应用，每个测试拥有自己的临时 SQLite、内存凭据和 MockBackend。真实 Windows 内核、系统代理和原生窗口需独立验收；mock 不能代替这些证据。详见 [测试指南](testing.md)。
