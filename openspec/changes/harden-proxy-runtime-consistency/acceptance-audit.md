## Scope and Evidence Boundary

核对基线：`1a073c309eb6a9e77cf255deb2dd3393c2048ae3` 加当前未提交工作区。以下文件均以仓库根目录为相对路径；最新命令与平台证据见 `verification.md` 和 [windows-execution.md](windows-execution.md)。下表保留首次本地规格核对的实现入口及当时的证据边界；其中 Windows 待验收项现已完成干净检出、六点崩溃/Job 清理、固定真实内核及实际系统凭据/启动项恢复。安装包桌面验收已完成，Actions URL 仍待补齐，不能将历史表格中的待执行状态当作当前进度。

## Historical Requirement Audit

### runtime-consistency

| Requirement / Scenario                                                     | 实现与验证依据                                                                                                                                                                                                                            | 结论与剩余证据                                                 |
| -------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| 凭据更新驱动实际运行会话：仅修改密码、相同配置重新导入、新凭据候选提交失败 | `configuration_real_core_tests.rs` 实际内核认证头核对（macOS 额外证据）；`configuration_runtime_tests.rs` 实际服务与协调器组合验证；`configuration_staging_tests.rs` 暂存、运行时、存储失败；`runtime_plan_tests.rs` 随机版本影响有效计划 | 本地组合测试通过；任务 8.2 的 Windows 固定内核凭据替换仍待执行 |
| 操作结果独立于会话健康：切换失败后旧会话继续运行、旧内核随后退出           | `runtime_health.rs` 持续检查活动会话；`runtime_recovery_tests.rs` 与 `configuration_plan_tests.rs` 连续故障恢复；`App.runtime-feedback.test.tsx` 错误与仍生效模式并存                                                                     | 本地通过；Windows 所有权恢复与托盘退出待 8.2 / 8.3             |
| 按实际运行计划决定内核切换：改名不重启、全局无关出口凭据缺失               | `runtime_plan.rs`、`configuration_plan_tests.rs` 进程身份与修订；`sing_box_observation_tests.rs` 受控实际内核的全局/规则凭据边界                                                                                                          | 本地通过；规则模式仍加载所有启用出口，不增加故障直连           |

### core-control-observation

| Requirement / Scenario                                     | 实现与验证依据                                                                                                       | 结论与剩余证据                                             |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| 有界且受保护的控制请求：分块响应、持续慢速、超限/未授权    | `core_control.rs` 回环、no_proxy、无重定向、Tokio 2 秒整流时限、1 MiB 上限；`core_control_tests.rs` 协议与错误夹具   | 本地 HTTP 夹具通过；真实内核响应和 Windows 环境待 8.2      |
| 观测不阻塞会话管理：读取期间切换内核                       | `sing_box_backend.rs` 锁外读取、run_id 前后核对；`sing_box_observation_tests.rs` 慢响应期间切换/停止，迟到成功被丢弃 | 本地受控内核通过；Windows 固定内核待 8.2                   |
| 可解释的错误与恢复反馈：切换失败但旧模式有效、恢复归属不明 | `RuntimeFeedback.tsx`、`use-mode-switching.ts`、`tray.rs`；前端及托盘单测；恢复命令现显式传入 `confirmed: true`      | 本地通过；外部系统代理改变、真实托盘和 WebView2 呈现待 8.3 |

### configuration-recovery

| Requirement / Scenario                                                     | 实现与验证依据                                                                                                                                                                  | 结论与剩余证据                                                                                                                       |
| -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| 数据库迁移原子性：建表后版本更新前中断                                     | `store_migrations.rs` 每步 SQLite 事务；子进程中断/重开测试；未来版本拒绝且数据库不改写                                                                                         | 本地子进程测试通过                                                                                                                   |
| 跨资源提交可恢复：凭据暂存后崩溃、提交后清理前崩溃、启动项恢复遇到外部修改 | `configuration_durable_commit.rs`、`configuration_startup_recovery.rs`；`configuration_crash_tests.rs` 六个中断点；启动项类型/字节比较测试                                      | 合成持久化资源与本地实际 SQLite 通过；Windows Job 进程与实际外部资源尚未验证，5.5 保持未完成，schema v4 尚不可作为验收完成的升级交付 |
| 恢复期间凭据与格式兼容：旧版本配置首次升级                                 | `configuration_staging_tests.rs` 旧 ID 引用迁移重试；`configuration_upgrade_tests.rs` v3/v4 与匹配合成凭据回退；`transfer.rs` v1/v2 无秘密导出；`ipc_contract.rs` 公共 DTO 边界 | 本地通过；Windows 系统凭据升级/回退操作仍需目标环境演练                                                                              |

### diagnostic-task-lifecycle

| Requirement / Scenario                                         | 实现与验证依据                                                                                                                                                                    | 结论与剩余证据                                                                               |
| -------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| 后端测速资源限制与去重：单测/批测交错、同代理重复请求          | `latency_tasks.rs` 最多 3 执行/32 等待；`latency_task_tests.rs` 边界与复用；`ipc_latency_tests.rs` 实际 Tauri mock webview                                                        | 本地通过                                                                                     |
| 测速取消与修订隔离：退出/重入、凭据更新期间取消                | `latency_task_lifecycle.rs` 最后订阅者释放与失效；`latency_tests.rs` 受控实际测试内核退出及目录回收、主内核不受影响；前端迟到订阅释放和页面重入测试                               | 本地通过；Windows 固定内核进程回收待 8.2                                                     |
| 路由预测绑定输入和已保存配置：等待期间编辑目标、修改已保存规则 | `configuration_china.rs` 在串行锁中读取配置与持久化修订；`RouteTest.tsx` 输入世代及配置事件失效；`RouteTest.test.tsx` 目标/端口/旧修订/卸载；`runtime_events.rs` 修订变化触发事件 | 本地通过；结果仅预测规则模式，不声称建立连接；默认出口、规则与预设均通过相同成功提交修订失效 |

### desktop-quality-gates

| Requirement / Scenario                          | 实现与验证依据                                                                                 | 结论与剩余证据                                                                     |
| ----------------------------------------------- | ---------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| 跨平台干净检出可验证：Windows 默认 Git 环境检出 | `.gitattributes`、Quality/Package 分阶段工作流；本地完整检查                                   | 未充分验收：1.1 / 1.3 仍缺 Windows 干净检出、二进制对照和当前源码对应 Actions 链接 |
| 真实内核测试显式执行：内核变量缺失              | `test-real-core.mjs` 固定路径/版本/哈希；`test_core.rs` 必需模式禁止提前返回成功；门禁回归测试 | 负向门禁已验证；当前环境缺固定 Windows 内核，不计作正向真实内核通过                |
| 跨层与桌面验收证据：本地测试通过但桌面未验收    | 本文、本地命令记录、未勾选的 5.5 / 8.2 / 8.3                                                   | 待完成：当前 Windows/NSIS/WebView2 桌面证据缺失，本地构建不能替代                  |

## Historical Specification Synchronization Order

此处仅记录同步方案，不执行同步或归档，不改动历史变更任务完成记录。

1. 先同步 `implement-proxy-runtime-backend`：建立 `proxy-configuration`、`proxy-runtime`、`proxy-observability` 主规格基线。
2. 再同步 `multi-proxy-china-direct-routing`：建立 `multi-proxy-routing`、`china-direct-routing`，同时修订较早 `proxy-configuration` 的活动档案删除场景，明确默认出口/启用规则引用必须拒绝删除，不能自动清空引用或静默切换直连。独立 capability 新增不会自动解决旧 capability 内的冲突，归档前需显式编辑并审查这一衔接。
3. 最后同步本变更的五个独立加固 capability；在主规格中明确按模式生成运行计划、分离会话健康与操作结果、提交中断恢复的新增保证。规则模式所有启用出口语义保留，全局模式仅默认出口；旧会话保留时长与已应用模式，未成功建立的新会话无运行时长。
4. 每一步完成后严格校验所有主规格与待归档变更；目标平台验收完成前不归档本变更。

## Regression Invariants

- 引用保护：`configuration_transaction_tests.rs::referenced_rule_blocks_disable_and_delete_without_changing_configuration` 保持通过；默认引用校验仍在配置模型及服务内。
- 禁止静默降级：缺失凭据/无效出口导致运行计划或候选配置拒绝，保留旧会话；未命中规则的既定直连 fallback 不属于故障降级。`sing_box_config_tests.rs`、运行时回滚组合测试保持通过。
- 秘密隔离：普通档案、恢复日志、导出、事件和生成普通 DTO 不包含密码、用户名值或内部凭据引用；显式凭据读取/替换的 DTO 位于 `generated/credentials.ts`。生成器只纳入公开白名单类型，不纳入 `ProxyCredential`、`StoredCredential`、`LatencyInput` 或持久化内部模型。
- 类型契约：所有应用命令统一经过 `src/lib/ipc.ts`；生成漂移检查已列入 `pnpm check`，负向编译用例保证错误命令名、错误参数和自选返回类型不可通过。后端导入仍进行运行时验证，TS 类型不替代外部 JSON 校验。

## Outstanding Acceptance

目前 29/30 项完成；仍需当前修订对应的 GitHub Actions URL。NSIS 安装启动、托盘退出和异常退出恢复的桌面证据已完成。Windows 干净检出、六点中断/Job 清理、固定真实内核和实际凭据库/启动项恢复已通过，完整记录见 `windows-execution.md`。整体变更保持实施中，不把规格核对本身当成产品验收通过。
