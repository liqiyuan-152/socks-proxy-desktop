# 服务架构

Tauri IPC 接收命令并从应用状态获取 ApplicationService。它作为统一 facade 委托代理、路由和运行时领域服务，保持原有命令参数和返回结构。服务接口见 src-tauri/src/services/interfaces.rs，依赖适配器见 services/adapters.rs；业务服务不依赖 Tauri 窗口。

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

配置文档仍为 v2，诊断 schema 5 增加可选错误类型和操作，错误上下文新增可选追踪身份；旧纯文本诊断仍可查看。错误身份和恢复建议见 [错误指南](error-handling.md)，日志、指标和只读工具见 [观测指南](observability.md)。

测试通过可注入适配器构建独立应用，每个测试拥有自己的临时 SQLite、内存凭据和 MockBackend。真实 Windows 内核、系统代理和原生窗口需独立验收；mock 不能代替这些证据。详见 [测试指南](testing.md)。
