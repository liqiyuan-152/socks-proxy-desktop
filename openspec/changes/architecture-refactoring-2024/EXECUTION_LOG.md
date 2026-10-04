# 架构重构执行日志

## 2026-10-04：任务 1.1 创建服务模块结构

- 阅读执行指南、proposal、design 和 tasks；OpenSpec apply 状态为 ready。
- 新增 `services/` 模块，在 `lib.rs` 声明；建立代理档案、路由、运行时和应用 facade 的实现模块。
- 在 `services/interfaces.rs` 定义 `ProfileService`、`RoutingService`、`RuntimeService`。三个接口均支持 `Send + Sync` 和 `Arc<dyn Trait>`，接口与实现命名分离，避免后续具体服务同名冲突。
- 复用现有 DTO、配置版本号和 `AppError`；领域错误在阶段二引入。
- 为接口方法添加中文文档，明确引用校验、凭据隔离、事务回滚、模式持久化和恢复阻塞语义。模块文档包含可编译的三个接口使用示例。
- 现有业务实现和 IPC 路径保持不变，任务 1.2–1.6 尚未完成。

验证结果：

- `pnpm rust:fmt`：通过。
- `pnpm rust:clippy`：通过，包含所有目标，警告视为错误。
- `cargo test --manifest-path src-tauri/Cargo.toml`：155 项通过，3 项忽略（由隔离进程测试调用的现有子测试），无失败。
- 补全接口示例后运行 `cargo test --manifest-path src-tauri/Cargo.toml --doc`：2 项文档测试通过。

后续从任务 1.2 开始提取代理档案逻辑，需要保持现有持久化事务与共享写入锁。

## 2026-10-04：任务 1.2 实现 ProxyProfileService

- 提取档案列表、保存、删除、默认代理选择与凭据读取到独立服务，实现 `ProfileService`；旧入口通过 `Arc<dyn ProfileService>` 委托。
- 将共享资源与持久化事务边界提取为 `ConfigurationContext`，构造函数注入 `Arc<dyn ConfigurationStore>`、`Arc<dyn CredentialStore>`、启动与运行时适配器。所有服务复用同一串行锁和恢复阻塞标记。
- 现有事务日志、凭据暂存、运行时确认、故障回滚与测速失效逻辑保持完整；不以直接数据库保存替代跨资源事务。
- 档案 DTO 移至服务层并保留旧路径重导出，IPC 格式不变。
- 将三个档案单元测试迁移为直接调用 `Arc<dyn ProfileService>`；补充双档案凭据隔离与数据库提交失败回滚测试，并将删除测试改为检查真实持久化凭据引用。
- Rust 回归测试：156 项通过，3 项原有隔离进程子测试忽略，2 项文档测试通过。Rust 格式与 Clippy 检查通过。

## 2026-10-04：任务 1.3–1.6 与阶段一自动化验证

- `RoutingService` 实现规则列表、替换、重排、路由预测和国内直连操作；将现有规则集校验测试迁至服务模块，并直接通过 trait 验证引用完整性、重复排序拒绝、配置版本及运行时失败回滚。
- `RuntimeService` 委托现有 `RuntimeCoordinator`，统一模式保存、停止和网络恢复。使用真实 `ManagedRuntime` 搭配测试 backend 验证停止前保存失败、切换失败保留旧模式、恢复失败阻塞写入及恢复成功解除阻塞。
- 创建 `ApplicationService` facade，以 `Arc<dyn Trait>` 组合档案、路由、运行时服务，另设设置服务。应用持有共享事务协调对象，跨资源写入仍由原有恢复日志协议完成。
- 配置导入导出移至应用层；增加健康检查、诊断查询与清理、用户确认后的网络恢复响应及历史能力接口。
- 诊断读写通过配置存储接口委托，SQLite 适配器继续维护原有分页、过滤、确认和保留策略。模式失败记录及网络恢复记录从 IPC 迁至应用层，保留原有触发路径与响应行为。
- IPC、测速 IPC、托盘、运行时事件与 `lib.rs` 改用 `ApplicationService`。旧 `ConfigurationService` 保留为弃用别名，并通过健康检查测试验证别名兼容性。
- 所有本次新增或实质修改源文件少于 400 行。原规划文档仅追加必要实现说明并应用格式规范；不修改既有工作区 `.cargo/` 配置。

自动化验证：

- `pnpm check`：通过，包含格式、前端 lint、类型检查、99 项前端测试、Rust 格式、全部目标 Clippy 及 IPC 契约检查。
- `cargo test --manifest-path src-tauri/Cargo.toml`：159 项通过、3 项原有隔离进程子测试忽略；2 项文档测试通过。
- `cargo build --manifest-path src-tauri/Cargo.toml`：通过。
- `pnpm build`：通过。
- `git diff --check`：通过。

剩余阶段一验收：

- 1.7.3 手工验收尚未执行；当前宿主为 macOS，Windows 系统代理接管必须在目标平台验证，不能由测试 backend 或自动返回的真实内核测试替代。
- 1.7.4 启动、配置加载与 IPC 性能基准尚未完成，不据此声明阶段一完整验收。
- 新代码覆盖率尚未量化，后续需生成报告并确认达到执行指南的 80% 要求。

## 2026-10-04：补充覆盖率、性能采样与原生 UI 证据

- 安装 `cargo-llvm-cov` 与 LLVM 工具，收集当前平台覆盖率；服务层源文件按规范化路径汇总，并排除测试夹具，详细结果保存在 `reports/phase1-coverage-summary.json`。
- 增加显式忽略的性能采样测试，常规测试不施加时间阈值。分别构建 Git HEAD 基线与重构工作区的二进制，在无编译负载的情况下交替采样三轮；配置加载中位数 +1.59%，测试宿主初始化 -3.65%，Tauri 命令分发 -1.92%。完整原生启动和 JavaScript IPC 往返尚未测量，1.7.4 保持未完成。
- 补充旧同步测速入口测试，验证其使用共享注册表且不改变默认代理与运行模式。
- 构建独立标识的 macOS `.app` 并通过原生 UI 验证代理新增、编辑、默认选择、规则保存与预测。原有代理删除按钮的 `window.confirm` 未显示确认框；模式切换在当前平台按能力禁用。1.7.3 保持未完成，详细证据与限制见 `reports/phase1-verification.md`。
- 最新 `pnpm check` 通过，IPC 契约未漂移，99 项前端测试通过。覆盖率运行的 Rust 测试为 160 项通过、4 项忽略（3 项现有隔离进程子测试和 1 项显式性能采样）。
- 已询问可用 Windows 验收环境；当前目标继续保持进行中，不以测试 backend 或 macOS 验证替代目标平台验收。

## 2026-10-04：修复原生删除确认，并实施任务 2.1 与 2.2.1

- 代理与规则删除改用复用 shadcn Dialog/Button 的应用内确认组件，初始聚焦取消。提交期间禁止重复请求和关闭；失败保留确认框与错误，成功关闭。
- 新增 4 项组件测试和 5 项真实页面/IPC 适配测试，分别验证取消无写入、确认的命令与参数、规则列表更新、代理刷新、引用拒绝及重试。
- 隔离标识的 `.app` 重新构建成功；原生 UI 验证代理/规则确认框、初始焦点、引用拒绝后的错误展示和取消后数据保留。没有点击原生成功删除的最终按钮，测试数据保留；Windows 模式切换仍未验证，因此 1.7.3 保持未完成。
- 创建公开 `domain_errors` 模块，使用 thiserror 定义 ProxyError、RuntimeError、RoutingError、StorageError，以及验证、凭据和引用位置类型。存储失败细分读取、写入、事务、损坏数据、版本和锁失败。
- 所有领域错误可转换为 AppError；字段及引用完整保留，未迁移适配器的原始 code/message/fields 无损传递。系统代理与状态不变量错误的对外消息隐藏内部细节。此步骤未更新服务返回类型或 IPC 序列化结构，后续按 2.2 与 2.3 继续。
- 新增 12 项 Rust 测试覆盖嵌套来源、所有转换分支、字段保留、原始序列化契约和内部信息隐藏。Rust 回归 172 项通过、4 项忽略，2 项文档测试通过；Rust 格式和全部目标 Clippy 通过。
- 最终 `pnpm check` 全部通过：108 项前端测试通过，Rust 格式与全部目标 Clippy 通过，IPC 契约未漂移。`git diff --check` 通过。
- 使用 cargo-llvm-cov 仅运行 12 项领域错误测试，转换模块 68/68 可执行行覆盖（100%），明细见 `reports/phase2-domain-errors-coverage.json`；类型声明自身无 LLVM 可执行行。
- 当前 37/183 项完成。下一项为 2.2.2 结构化错误上下文；阶段一 1.7.3 和 1.7.4 仍须补充原生成功删除、Windows 模式切换与完整原生启动/JavaScript IPC 性能证据。

## 2026-10-04：任务 2.2、2.3 与前端错误处理基础设施

- 完成 2.2.2–2.2.5：AppError 新增可选 context，包含 UUID、UTC 毫秒、根因领域与变体、操作名称和恢复建议。上下文使用 Box 保持错误值大小；旧 code/message/fields 保留，旧错误无上下文时序列化形状不变。
- 堆栈只收集脱敏符号，过滤文件路径、地址和超长符号，最多 16 帧。堆栈只写入本地诊断，不进入 IPC。日志与诊断不记录用户错误消息、字段内容、输入值和凭据。
- 应用边界统一补充和持久化失败上下文，诊断记录与响应共享错误 ID；诊断失败不覆盖原业务失败。模式切换与恢复失败避免重复生成身份。实际 Tauri 命令测试验证身份、时间、类型、恢复建议和诊断关联。
- 完成 2.3.1–2.3.5：三个领域接口及实现返回 ProxyError、RoutingError、RuntimeError，ApplicationService 统一转换。不存在、被引用、凭据状态、规则目标、缺失资产、恢复阻塞及存储失败均有直接变体测试；未迁移适配器按稳定错误码映射，未知原因保留原始错误。
- IPC 命令签名和配置格式不变；代理不存在、被引用分别使用设计中的 proxy_not_found/proxy_in_use，字段详情仍完整。凭据未启用/缺失使用 credential_error。错误上下文生成新的可选 TypeScript 类型，前端旧错误兼容仍保留。
- 完成 2.4.1–2.4.3 与 2.4.5：统一错误处理器验证未知拒绝值并过滤非契约字段，原 errorMessage 调用链委托该处理器。新增 ErrorAlert，展示字段、恢复建议及身份，支持明确点击的重试与诊断按钮；防止重复重试，失败后可重试，新错误覆盖旧重试失败。AppErrorBoundary 保护整个 React 界面，允许重新创建界面并保留原生运行时，隐藏技术异常文本。
- 前端新增 20 项错误处理器测试、6 项错误展示测试和 2 项错误边界测试。各页面富错误信息与操作按钮迁移尚未完成，2.4.4 保持未勾选。
- Rust 全套验证 184 项通过、4 项忽略，2 项文档测试通过；领域服务错误分支、来源保留、身份传递、恢复阻塞及原事务回滚测试全部绿色。
- 覆盖率明细见 reports/phase2-services-coverage.json：领域适配器/转换/上下文 100%，应用错误记录 95.3%，代理服务 93.0%，路由服务 88.9%，运行时服务 97.7%；当前目标总计 95.1%。不据此证明 Windows 条件分支。
- 曾在覆盖率编译同时运行 Rust 回归时出现已有规则测试进程的 2 秒超时；编译结束后完整回归重新通过，没有放宽测试或生产超时。
- 当前 50/183 项完成；下一项为 2.4.4 页面错误展示迁移。阶段一目标平台手工验收与完整启动/IPC 性能门禁仍未完成。

- 本轮最终 `pnpm check` 全部通过：136 项前端测试、Rust 格式与全部目标 Clippy、类型检查与 IPC 契约检查通过；`pnpm build` 成功，`git diff --check` 通过。

## 2026-10-04：Windows 隔离验证与任务 2.4.4

- 使用用户授权的 OpenSSH Windows 11 x64 环境，在独立目录解压当前代码快照；不修改常规应用数据，不保存凭据。进程使用已安装的 stable Rust 1.99、Node 22.22.2、pnpm 10.33.0。
- 上传快照通过 Windows pnpm check（136 项前端测试）、固定 sing-box 1.14.1 的校验与真实内核回归（194 项 Rust 测试、6 项忽略、2 项文档测试），前端构建和独立应用标识的 debug 桌面构建成功。环境与快照摘要见 reports/windows-environment.json。
- Windows 证据对应本次页面迁移前的快照；不据此证明原生界面手工操作、真实系统代理接管、原生启动或 JavaScript IPC 性能。1.7.3/1.7.4 保持未勾选。
- 完成 2.4.4：代理、规则、设置、配置导入、连接日志、运行时反馈与关联读取/模式切换 hook 保留规范化 AppError，移除不安全的 Partial<BackendError> 转型。页面和弹窗复用 ErrorAlert，保留字段、恢复建议和错误编号；延迟结果保留完整错误并在详情提示中展示。
- 编辑失败在表单弹窗内展示，删除失败在确认弹窗内展示，避免背景重复提示。凭据读取保留主动重试；运行时保留明确的模式重试与网络恢复操作，增加诊断入口。错误展示按钮使用 type=button，避免表单内点击重试意外提交。
- 同一错误同时出现在 IPC 和状态快照时保留富上下文版本。新增两项真实页面删除失败详情测试、一项运行时上下文去重测试；更新 hook 的错误对象断言。
- 本地 pnpm check 全部通过（139 项前端测试、lint、类型检查、Rust fmt/Clippy、IPC 契约），pnpm build 和 git diff --check 通过；本轮源文件均小于 400 行。
- 当前 51/183 项完成，下一项 2.5.1 诊断类型字段；目标保持 active。

## 2026-10-04：诊断持久化、聚合、导出与阶段二验证

- 完成 2.5.1–2.5.3、2.5.5：数据库 schema 5 增加独立的可选 error_type/operation，原 severity 保留 info/warning/error 并验证写入。旧结构化摘要提取类型与操作，旧文本及配置 v2 保持原样；迁移中断回滚测试扩展到新步骤。
- 同一错误编号幂等记录，不覆盖原始信息；不同编号按类型/操作/严重级别聚合，报告次数与首次/最近时间。原始记录保留，过滤语义与分页一致。
- 新增聚合与 JSON Lines 导出 IPC；导出通过一次 SQLite 读取构成完整过滤快照，覆盖 105 条记录和多行转义，界面分页上限不影响导出。实际 Tauri 命令测试验证类型、操作、次数、完整导出和无效筛选。
- 设置页实现诊断查看、详情、聚合、刷新及级别筛选。初始浏览器链接下载未取得原生落盘证据，点击后桌面自动化持续 timeoutReached/noWindowsAvailable；应用进程仍存在。随后改为官方 tauri-plugin-dialog 2.6.0 的系统保存选择器（不增加通用前端文件系统权限），Rust 原子写入选定路径；取消不写文件，失败保留原文件且不泄露本地路径。新增原子保存、失败保留、应用边界身份、取消与重复点击测试。2.5.4 因原生保存验收尚未完成，保持未勾选。
- 完成 2.6.2/2.6.3：原生隔离应用触发无效代理字段错误，弹窗显示字段、恢复建议与编号；设置页诊断和日志与该编号一致，具有根因类型、操作、级别和脱敏符号。详见 reports/phase2-verification.md。取消未新增代理；未操作普通应用配置，也未清理隔离验收记录。
- 完成 2.6.4：基线与当前无编译负载交替采样三轮，配置读取 +0.07%、测试宿主初始化 +2.36%、Rust Tauri 分发 -1.52%、成功代理保存 -1.80%、成功规则保存 +0.54%。这些不是原生启动/JS 往返或 Windows 系统代理接管证据，1.7.4 仍未完成。
- 完成 2.6.5：新增 docs/error-handling.md，说明类型转换、完整错误状态、诊断隐私、同类聚合、原生保存、schema 5 及安全回退约束；README 与设计文档同步。
- 初版诊断代码本地 188 项 Rust 测试/142 项前端测试通过，诊断相关行覆盖 95.9%；Windows 快照通过 198 项 Rust 测试/142 项前端测试、2 项文档测试、固定内核回归和桌面构建，详见 reports/windows-phase2-diagnostics.json。该快照不证明后续原生保存修正。
- 原生保存修正本地最终 191 项 Rust 测试、4 项忽略、2 项文档测试通过；pnpm check（143 项前端测试）与 pnpm build 通过。一次默认并发 Rust 回归触发已有规则进程 2 秒超时；使用 RUST_TEST_THREADS=2 完整重跑通过，未修改测试或生产超时。新保存命令使用泛型 AppHandle，实际 MockRuntime IPC 验证通过。
- 当前 59/183 项完成；2.5.4 原生保存桌面验收、2.6.1 最终错误路径审计与阶段一 Windows 手工/真实性能门禁尚未完成。最新 Windows 原生保存快照仍在验证；整个目标保持 active。

- 最新原生保存快照 Windows 验证完成：201 项 Rust 测试、6 项忽略、2 项文档测试、143 项前端测试全部通过，固定内核回归、完整质量检查和 debug 桌面构建成功，见 reports/windows-native-export.json。新增保存相关行覆盖见 reports/phase2-native-export-coverage.json；原生选择器成功/取消仍不由 CLI 测试证明。

## 2026-10-04：错误路径审计与后端状态机接入

- 完成 2.6.1：补充 33 项领域错误变体上下文矩阵、无效诊断级别无写入测试；与已有服务、适配器、事务故障及实际 IPC 测试共同审计错误类型。详见 reports/phase2-error-audit.md。
- 完成 3.1.1–3.1.5：纯状态机集中定义节点、事件、转换及不变量；候选与已应用会话分离，转换不执行 I/O，无效事件不消费旧节点。
- 完成 3.2.1/3.2.2/3.2.4/3.2.5：ManagedRuntime 使用节点生成 phase/health/applied_mode/uptime；模式切换、配置提交、退出监测和恢复通过事件驱动。保存完整旧节点实现持久化回滚；元数据提交不重启内核。操作错误与生命周期错误仍区分，退出不覆盖历史操作结果。
- 历史记录最近 128 次事件，包含时间、操作编号、前后阶段和接受/拒绝状态，不记录会话身份、配置或错误文本；回滚不丢失过渡历史。已有 log debug 转换记录就绪，tracing 集成尚未完成，3.2.3 保持未勾选。
- 无效候选会话会先撤销再返回不变量错误；撤销失败撤销健康声明。新增候选撤销、持久化回滚、无候选兼容恢复及快照恢复不变量测试，原并发过渡测试仍验证读取不阻塞、旧模式不提前改变。
- 完成 3.3.3/3.3.4/3.3.5：不变量与并发测试绿色，docs/runtime-state-machine.md 增加转换图与事务说明。完整有效/无效事件矩阵 3.3.1/3.3.2 继续保留待完成。
- 最终覆盖率运行 205 项 Rust 测试通过、4 项忽略，详见 reports/phase3-runtime-coverage.json；完整回归此前 203 项及 2 项文档测试通过。pnpm check（143 项前端测试）通过，新增 Rust 测试随后通过覆盖率运行，未改变 IPC 契约。
- 完成 3.4.1：添加 Zustand 5.0.15 依赖，为前端统一 store 迁移准备；尚未替换旧 hooks。当前 73/183 项完成，全部重构目标保持 active，原生验收门禁保持待完成。

## 2026-10-04：前端统一状态与独立集成测试

- 完成 3.3.1/3.3.2：12 种生命周期形态 × 17 个事件组成 204 项转换矩阵，验证有效/拒绝事件、原节点保留及转换后不变量。
- 完成 3.4.2–3.4.5、3.5.1–3.5.5、3.6.1–3.6.3/3.6.5：Zustand store 接管共享快照、代理、规则、能力、连接、资源版本、订阅、模式队列和可见性轮询。移除旧 BackendContext、useBackendObservation/useModeSwitching/useRuntimePolling，所有页面采用 store 选择器。开发模式启用 devtools；凭据只作为调用参数，不进入共享状态。
- 代理、规则、设置、导入与网络恢复操作通过 store actions 刷新共享数据；页面草稿与错误反馈仍局部持有。页面卸载后的已提交写操作刷新应用级数据，同时不发布卸载页面的反馈。完整前端 147 项测试通过，覆盖资源竞态、队列、旧订阅清理、StrictMode、选择器隔离、凭据不留存、错误保留与页面操作。
- 新增 docs/frontend-state.md，设计文档同步。pnpm check 全部通过，pnpm build 通过；旧 hooks 测试已迁移到 store，保留实质竞态断言。CUA 重试读取原生应用仍返回 timeoutReached，3.6.4 原生手工验收保持未完成。
- 完成 4.1.1–4.1.5：tests/support 提供 TestApp、MockBackend、内存凭据；使用独立临时 SQLite 数据库，ApplicationService 和真实 ManagedRuntime 协作。新增 services/adapters 公共组合边界、ManagedRuntime::new 获取独占会话所有权，供独立测试注入适配器。
- 完成 4.2.1–4.2.4、4.4.1/4.4.2/4.4.4/4.4.5：7 项独立集成测试覆盖代理生命周期、多代理切换、凭据更新/验证、删除保护、模式循环、计划变更重启/元数据不重启和并发写一致性。崩溃测试验证恢复网络及显式重试，不据此勾选 4.4.3 的自动恢复需求。
- 最新完整 Rust 回归 206 项单元测试、7 项独立集成测试、2 项文档测试通过，4 项平台相关忽略。所有新增/实质修改源文件均小于 400 行。
- Windows Zustand 快照验证脚本遇到原生 stderr 被 PowerShell ErrorActionPreference=Stop 当作终止错误，未得到测试完成证据。已修正隔离验证脚本按原生命令退出码判断，保留 stdout/stderr；待以最新快照重跑，不将先前 Windows 绿色结果用于本轮新代码。
- 当前 102/183 项完成；目标保持 active，继续阶段四补充测试与阶段五追踪，原生验收与真实启动/IPC 性能门禁仍待完成。

## 2026-10-04：属性测试、覆盖率与 CI

- 完成 4.6.1–4.6.3/4.6.5：领域服务行覆盖率已达到 80% 目标；领域错误转换及 33 项变体矩阵保持全覆盖。新增 proptest 1.9.0，256 组随机事件序列验证不变量、拒绝保留旧节点、候选不提前生效；候选模式不匹配的属性测试验证旧健康会话不丢失。
- 完整 LLVM 覆盖率运行包含单元和独立集成测试，整体行覆盖率 85.45%，摘要见 reports/architecture-coverage.json；保留平台边界，不证明 Windows 分支。前端最新 149 项测试通过，store 行覆盖率 98.22%、分支 82.81%，见 reports/frontend-store-coverage.json。
- 完成 4.7.1–4.7.5：Windows Quality 流程明确运行单元、集成、属性及文档测试；新增 coverage job 生成 Rust LCOV 与前端 store 报告并上传 Actions artifact。CI 配置已编写，未触发远程 GitHub Actions，不声称远程工作流已运行。
- 新增 pnpm test:coverage、Vitest V8 插件、覆盖率产物 ignore 及 lint 排除，避免生成 HTML/JS 引入噪声。新增 docs/testing.md 包含独立测试示例、临时资源/凭据规则、竞态测试、属性复现和真实平台验收边界；README 增加 Quality 徽章和指南链接。
- pnpm check（149 项前端测试、fmt、Clippy、IPC 契约）通过；覆盖率生成目录曾被 ESLint 扫描而产生生成文件警告，加入排除后 lint 绿色。前端构建成功，git diff --check 通过。
- 当前 111/183 项完成。目标保持 active；最新 Windows 快照待完整重跑，阶段四恢复/E2E 与阶段五尚未完成。

## 2026-10-04：配置恢复集成测试

- 完成 4.3.1/4.3.2/4.3.4/4.3.5：正常停止后重新打开持久配置；隔离子进程在恢复意图保存后/原子提交后直接 exit(86)，父进程验证修订、提交标记及完整前后配置，重建应用后确认恢复阻塞和恢复完成后的日志清理。
- 恢复失败测试验证可操作的错误编号与建议、健康对账后的 RecoveryRequired 和重新恢复成功；不将 mock 恢复当作真实 Windows 系统代理接管。4.3.3 保持未完成。
- 独立集成套件 10 项通过、1 项忽略（由父测试显式调用的崩溃子进程），Clippy 全目标通过。当前 115/183 项完成。
- 4.4.3 的“自动恢复”存在行为定义差异：现有实现自动恢复网络设置但不自动重启代理，已请求用户选择。等待答复期间继续其他已授权任务，不擅自改变崩溃后的代理启动策略。

## 2026-10-04：阶段五追踪、指标和诊断工具

- 完成 5.1.1–5.1.5、5.2.1–5.2.5：配置生产 JSON Lines/开发终端 subscriber、RUST_LOG 回退及应用目标白名单；服务参数全部跳过，事务、运行时、系统代理和错误路径记录固定字段。每个日志文件最多 1 MiB、4 个备份，Windows 重命名前关闭句柄，失败重开，不删除无关文件。
- 使用 tracing-appender 有界异步队列，最多 1024 条记录，满队列丢弃并在快照暴露累计数量；正常退出刷新。新增阻塞写入器测试准确验证队列溢出计数，以及 100 条 JSON 的退出刷新。服务/事务细节和不改变阶段的元数据提交默认 debug，生命周期和事务最终结果保留 info；所有状态事件仍保留于有界历史。
- 完成 5.3.1/5.3.3–5.3.5：单调耗时、成功/失败累计与最近 128 次 P50/P95，覆盖配置加载/应用、运行时启动/切换/停止和系统代理接管/恢复。已收集 Rust run 到 setup 的 startup_bootstrap，但这不是原生进程/首屏启动，5.3.2 保持未完成。
- 完成 5.4.1–5.4.3/5.4.5：JSON Lines 完整导出，错误上下文持久化 trace_id/span_id（无 subscriber 时 span_id 缺省）、根因与操作；跨领域转换保留身份。诊断编号/摘要/类型/操作的字面搜索最长 128 字符，与级别/时间筛选取交集，列表/聚合/导出/确认清理语义一致。新增 SQL 注入与百分号/下划线字面查询回归；设置页复用 Input，过期请求由 generation 丢弃。
- 完成 5.5.1–5.5.5：运行时状态/历史/指标快照、配置只读校验 IPC、流式日志查询 CLI、脱敏 folded 栈与 Inferno SVG 工具。火焰图生成使用原子替换，输入失败不覆盖现有文件。使用文档见 docs/observability.md。
- 完成 5.6.1–5.6.4：JSON/span 上下文、环境级别、第三方目标隔离、轮转/重开、指标精度及脱敏测试通过。本地完整质量检查通过（150 项前端测试、3 项工具测试、Clippy/格式/类型/IPC 契约），前端构建成功；Rust 完整回归 225 项单元、10 项独立集成、1 项工具示例、2 项文档测试通过。
- 5.6.5 尚未通过：同步/异步/默认级别/元数据 debug 四版三轮交替采样报告全部保留。最新 debug 测量代理保存 +6.41%，规则保存 +4.60%；其他读/分发指标低于 5%。不调整门槛、不将优化构建结果混入 debug 数据；优化构建补测进行中。
- 原生应用自动化恢复可用，但当前启动的旧 bundle 仍使用最初浏览器下载实现，不能据此证明后续原生选择器。最新隔离 bundle 和 Windows 快照验证进行中，2.5.4/5.4.4 保持未勾选。
- 当前 142/183 项完成。目标保持 active，未归档、提交或发布。

### 原生验收与生产追踪性能补测

- 最新隔离 debug bundle 原生诊断搜索/系统 Save sheet 的取消和成功路径通过；实际文件恰好包含搜索匹配的一条 JSON 记录。完成 2.5.4/5.4.4，证据见 reports/native-diagnostic-export.md。
- 优化 release 二进制默认 info 追踪三轮交替采样通过：代理保存 +4.17%、规则保存 +4.48%，读取/宿主初始化/Rust 分发没有测得增加。完成 5.6.5 的默认生产追踪门禁，见 reports/tracing-performance-release.json；debug 代理保存 +6.41% 的失败证据保留，不声称 debug/详细日志/火焰图满足同一上限。
- 完成最终文档验收五项：docs/architecture.md 与 docs/architecture-migration.md，更新错误/状态/测试/观测指南，新增 Unreleased CHANGELOG。只描述当前已实现行为，并明确最终平台验收尚未结束。
- 已审查应用源代码、脚本和文档中的 TODO/FIXME，目前没有遗留项；deprecated 别名仍待发布前移除。
- 当前 151/183 项完成。Windows 最新验证曾因隔离仓库根目录遗留 windows-phase3-desktop-status.json 格式拦截，将该验证产物移到仓库外后重跑；质量检查/真实内核回归已完成，桌面构建进行中。

### Windows 真实代理恢复与最新覆盖率

- 最新 Windows 快照完整验证通过（235 项 Rust 单元、10 项集成、1 项示例、2 项文档、150 项前端和 3 项日志工具测试）；固定内核 1.14.1 回归、完整质量检查、前端及 debug 桌面构建成功。证据见 reports/windows-observability.json，凭据未保存。
- 完成 4.3.3：显式 ignored 测试持有生产会话锁，真实 HKCU 用户系统代理指向存在的本地监听端口；停止和重新打开 SQLite/adapter 后完全还原原设置，所有权记录清理。真实运行 0.07 秒通过，Windows Clippy 全目标通过，见 reports/windows-live-proxy.json。此测试不证明原生 UI 或进程强制退出。
- 生成最新完整覆盖率报告：Rust 行 86.42%，前端行 90.74%、分支 80.75%；观测模块行覆盖均高于 94%，指标 100%，前端 store 98.22%。原始 JSON 与按仓库路径汇总见 reports/final-current-coverage.json。macOS Rust 不包括 Windows 专属分支，LLVM 分支零计数表示未采集。
- 本地最终 pnpm check 和 diff 检查通过，最新前端覆盖率运行 150 项测试通过。所有新增/实质修改源文件均低于 400 行，未提交或归档。
- OpenSpec 严格校验发现原始变更缺少 specs/ 增量文件（proposal 声明了新/修改能力）；未以 skip_specs 绕过。该规划缺口仍需补齐。
- 当前 153/183 项完成。尚待原生 Windows E2E/系统代理用户场景、真实性能与稳定性、自动恢复语义答复及发布收尾；目标保持 active。

## 2026-10-04：正式规格、延迟生命周期及原生配置往返

- 补齐 proposal 声明的 10 项能力 delta specs，建立首次正式行为契约；无主规格基线的能力均以 ADDED 记录。openspec validate architecture-refactoring-2024 --strict 通过，规划缺口已解决，没有跳过规格验证。
- 完成 3.2.3：状态转换 tracing 已实现并经阶段五测试验证，补记任务状态。完成 4.2.5：真实临时 SQLite + ApplicationService + 可控延迟执行器验证任务共享、单订阅释放、版本捕获、成功/失败和配置提交后的旧结果丢弃，延迟操作不改变运行时。
- 配置导出从 Blob 下载迁移为 save_configuration 原生保存命令，原 export_configuration 签名保留。复用原子写入，取消静默返回，保存失败保留原文件；新增单次触发、重复点击、取消及失败重试的前端测试，以及脱敏配置与存储上下文后端测试。
- 修正后的本地 pnpm check 通过（153 项前端测试）；原生 debug bundle 构建通过；顺序运行 Rust 回归 227 项单元、10 项集成、1 项示例和 2 项文档测试通过。曾将桌面构建与 Rust 测试并发导致 objc2 rlib 格式冲突，顺序重跑已通过，后续不并发运行这两类构建。
- 完成最终配置导入导出测试：CUA 原生保存取消/成功、编辑配置、原生文件选择/预览确认、代理与规则恢复、再导出完整 JSON 相等。证据见 reports/native-configuration-roundtrip.md；不据此勾选 Windows E2E 4.5.4。
- 用户对 4.4.3 的自动重启语义和 Windows WebDriver UI 技术授权尚未答复；不擅自执行依赖操作，继续独立任务。

- 完成发布收尾中的旧代码清理：删除 configuration_service 过渡模块，IPC 契约、基准测试和健康检查测试统一依赖 services；迁移指南与 CHANGELOG 明确 Rust 源码导入迁移，既有 IPC 与配置仍兼容。仓库源代码中无 deprecated/allow(deprecated) 遗留。
- 清理后 pnpm check（153 项前端测试）与完整 Rust 回归（227 项单元、10 项集成、1 项示例、2 项文档）通过；git diff --check 通过。
- 创建并切换到任务要求的本地 refactoring-v1 分支，完整未提交工作区保留；未提交、推送、归档或发布。当前 158/183 项完成，目标保持 active。

- Windows 最新快照通过完整质量检查（153 项前端测试）、237 项 Rust 单元、10 项集成、1 项示例和 2 项文档测试、固定内核 1.14.1 回归与 debug 桌面构建。证据见 reports/windows-configuration-roundtrip.json；不含正在追加的原生就绪启动指标，也不证明 Windows UI E2E。

### 原生就绪启动指标

- 完成 5.3.2：新增 startup_frontend_ready，后端从 Rust run 入口单调计时；初始快照可用、页面挂载且跨两个可见帧后发送无参数 IPC 确认。并发确认通过原子门禁每个进程只计一次；隐藏/卸载不产生虚假样本，浏览器预览跳过，观测失败不重试业务操作。
- 新增 2 项 Rust 测试（并发幂等与实际 MockRuntime 命令）和 4 项前端生命周期测试。pnpm check（157 项前端测试）、完整 Rust 回归（229 项单元、10 项集成、1 项示例、2 项文档）和隔离 debug app 构建通过。
- CUA 启动最新隔离 bundle，观察加载提示转为完整状态页；实际 info JSON 记录 startup_frontend_ready = 1011.523 ms，当前进程仅一条。证据见 reports/native-startup-ready.json。测量区间明确排除 OS 进程创建和像素首绘；单次测量不代表基线 +10% 门禁或 Windows 原生启动。
- 当前 159/183 项完成；最新 Windows 启动指标快照正在验证，目标保持 active。

- 启动指标版本 Windows 快照验证完成：239 项 Rust 单元、10 项集成、1 项示例、2 项文档、157 项前端测试通过；完整质量检查、固定内核回归、前端与 debug 桌面构建成功。证据见 reports/windows-startup-ready.json，未宣称 Windows 原生启动测量或 UI E2E 已完成。
- 重新生成完整 Rust LLVM 和前端 V8 覆盖率，final-current-coverage.json 已覆盖原生配置保存、延迟生命周期与启动指标新增代码；平台证据边界维持不变。

## 2026-10-04：配置加载性能、版本收尾与稳定性工具

- 完成最终配置加载基准：无并行本地编译负载下 3 轮顺序配对，原基线 0d93dd1 中位数 58.000 μs、当前 0.2.0 中位数 56.875 μs，变化 -1.94%。每轮 100 次预热和 1000 次配置加载，10 个档案、临时 SQLite。证据见 reports/final-configuration-performance.json；附带 Rust 分发数据不替代原生 JavaScript IPC 往返。
- 完成版本与依赖收尾：package.json、Cargo.toml/Cargo.lock、Tauri 清单统一 0.2.0，界面版本来自 package.json；增加版本一致性工具测试。新增所需 Zustand、tracing、dialog、proptest、覆盖率及火焰图依赖均已写入锁文件，保留固定 sing-box 1.14.1 与既有依赖兼容策略，不进行无关升级。该版本保持 Unreleased，没有发布。
- 本地 pnpm check（157 项前端、4 项工具测试）与 pnpm build 通过；Windows 同步快照通过完整质量检查、239 项 Rust 单元（9 项忽略）、10 项集成、1 项示例、2 项文档、固定内核回归、前端与 debug 桌面构建，见 reports/windows-version-soak-build.json。
- 新增显式 ignored 的真实内核稳定性测试：临时 SQLite、合成 HTTP CONNECT 上游、双向有效载荷、Global/Rules 模式循环、元数据提交和周期停止/重启，验证修订/健康/运行目录上限及停止后的完整清理。默认 1800 秒，独立代理适配器不会接管用户系统代理；不将此测试替代原生桌面/WebView 稳定性。
- macOS 100 秒短测通过：87 次双向探测、3 次模式转换和提交，运行目录最多 1 个，停止后为 0。初始夹具未显式清除 accept 继承的非阻塞属性导致读取失败，已修正为带超时的阻塞连接。
- 新增 Windows PowerShell 启动/内存采样工具 run-core-soak.ps1。首次 Windows 5 秒测试本身通过，但 Start-Process 返回空 ExitCode，工具正确拒绝生成绿色结果；显式缓存进程句柄并使用 UTF-8 BOM 修正后短测与采样工具通过，30 分钟真实内核验证正在运行。
- 根据官方 Tauri WebDriver 手动环境文档准备独立 E2E 配置和工具安装脚本，固定稳定 tauri-driver 2.1.0，Edge Driver 与本机版本匹配并验证 Microsoft 数字签名。仅安装/构建/检查服务状态；尚未获其他 UI 自动化技术授权，不执行 WebDriver 界面操作。
- 当前 161/183 项完成，目标保持 active。原生 E2E、完整启动/JavaScript IPC/桌面内存基准、完整稳定性、崩溃恢复语义及最终验收仍待完成。

- 完成 4.5.1 原生 E2E 环境：Windows 官方 tauri-driver 2.1.0 与 Edge 154.0.4258.53 匹配驱动安装完成，Microsoft 签名有效；独立 architecture-e2e 应用标识构建成功。驱动 `/status` ready=true，检查监听端口属于本次进程，正常清理；重复预检通过。见 reports/windows-e2e-environment.json。未创建 UI 会话，4.5.2–4.5.5 与 E2E 套件保持待完成。
- Windows 30 分钟稳定性当前已通过 404 秒、360 次探测和 13 次模式转换/提交，实际测试宿主 PID 45332 仍运行；不提前勾选长时间稳定性。当前 162/183 项完成。

- 将已经实际执行的最终单元/集成/前端/完整质量四项标记完成，证据为当前 0.2.0 Windows 完整快照与本地回归：229 项本地 Rust 单元、10 项集成、1 项示例、2 项文档；Windows 239 项单元、10 项集成、157 项前端、4 项工具、pnpm check 全部通过。后续源代码变更仍需重跑相应门禁，不将本次结果代替 E2E。当前 166/183 项完成。

- 新增 E2E 环境与稳定性工具后的本地 pnpm check 再次通过，git diff --check 无错误。Windows 长时测试最新已验证 PID 45332 活跃，进度 468.4 秒、420 次探测、15 次模式转换/提交；运行句柄保留为本地 exec session 79096，报告完成前不勾选稳定性或桌面内存基准。

## 2026-10-04：原生性能配对采样

- 完成 1.7.4 与最终 IPC/内存基准及性能对比报告：分别从重构前基线和当前工作区准备临时快照，加入完全相同的 run/首屏/无参数 ping 插桩，使用独立 benchmark-baseline/current 应用标识与同样的空配置。通过 CUA 原生启动/退出，首次创建库样本仅预热；确认本地编译均结束后执行三轮顺序配对。
- 原生就绪启动基线中位数 827.415 ms，当前 988.161 ms，+19.43%，不满足 +10% 门槛，最终启动门禁保持未勾选。未删除或混入额外样本，完整失败证据见 reports/native-performance-initial.json；当前正在分段诊断与修复，不能据此发布。
- 实际 WebView JavaScript IPC 20 次预热/200 次无参数 bool 往返，三轮累计/次数平均值中位数基线 0.745 ms、当前 0.475 ms。performance.now 约 1 ms 量化，单次中位数可能为 0，明确不声称亚毫秒单次精度。
- IPC 完成后原生应用进程每秒 RSS 采样 10 次，三轮中位数基线 114,655,232 B、当前 121,511,936 B，+5.98%。校验实际 PID/可执行路径，保留全进程树读数；OS 独立共享 WebKit 服务不在树内，不据此宣称完整桌面私有内存。新的采样脚本不启动或自动操作界面。
- reports/performance-comparison.md 汇总配置、原生启动、真实 IPC、明确范围的内存与生产追踪指标；源插桩与相同协议保存在 reports/native-benchmark-protocol.json。临时计时命令不加入生产 IPC。
- 当前 170/183 项完成，启动性能回退仍待修复，目标保持 active；30 分钟 Windows 内核测试仍在运行。

## 2026-10-04：启动回归修复与 Windows 长时内核结果

- Windows 30 分钟真实内核测试已结束，测试宿主退出码 0：1800.875 秒、1635 次双向探测、55 次模式切换及元数据提交，运行目录最多 1 个、停止后 0 个。固定 sing-box 1.14.1 SHA 校验通过，180 次每 10 秒资源采样见 reports/windows-runtime-soak.json 与 reports/windows-runtime-soak-memory.json。
- 服务/内核长时稳定性证据已取得；测试使用隔离系统代理适配器，不含 GUI/WebView，最终桌面长时间运行稳定性暂不勾选。首个宿主内存样本包含初始化，不把首尾差值解释为泄漏结论。
- 启动诊断显示同步加载首页将前端就绪从 483 ms 降至 181 ms；工作区 App.tsx 改为静态导入 StatusPage，其他路由保留延迟加载。本地 pnpm check（157 前端、4 工具）通过；三轮无编译负载配对验证进行中，保留原始失败报告。

- 修复后正式三轮配对通过：基线启动中位数 644.738 ms、当前 612.878 ms，-4.94%，完成最终 +10% 启动门禁。首次失败报告完整保留，新报告 reports/native-performance-eager.json 保存全部启动、IPC 和 RSS 原始样本；两版诊断协议源码相等。范围为 macOS debug 原生就绪，不代表 Windows 原生启动或像素首绘。当前 171/183 项完成，目标保持 active。

- 生产工作区 pnpm build 通过，状态页进入主入口静态模块，其余四个页面保持独立按需 chunk。Windows 已同步 App.tsx 与 .gitignore；最初命令受远端多层 PowerShell 引号影响，随后包装脚本又将 Vitest 正常 stderr 视为终止错误，均未计为测试通过。改为 cmd 内部捕获 stdout/stderr 并检查实际退出码后重跑。驱动产物由已有 .e2e-tools 忽略规则排除，临时 verification-status.json 已移到仓库外保留。
- Windows 临时验证脚本还需显式注入已有 Cargo bin 并指定已安装 stable 工具链；SSH 默认环境缺少 Cargo，加入路径后默认选中旧 1.78 无法解析 edition2024 依赖。已采用此前完整 Windows 验证相同的 stable 环境重跑，没有修改用户全局工具链。

- Windows stable 环境最终退出码 0，首页同步加载修复后的完整 pnpm check（157 前端、4 工具）及 pnpm build 通过；IPC 契约未漂移。报告 reports/windows-eager-startup-check.json 保留源哈希及日志摘要；这不是 Windows UI E2E 或启动耗时样本。当前仍 171/183 项完成。

## 2026-10-04：最新原生生产构建与管理场景

- 最新生产工作区隔离 debug app bundle 构建通过；真实 startup_frontend_ready 单次指标为 631.292 ms，不含临时基准 IPC。见 reports/native-startup-eager-production.json。
- CUA 实测代理添加/编辑/启停/默认选择、无效端口保留输入和重试；规则新增/编辑/启停/排序及两次实际规则匹配测试通过，取消删除保留数据。原生恢复备份后清理本次临时资源，导出完整配置与原有验收配置 JSON 及 SHA 完全相等。见 reports/native-management-manual.md。
- macOS 模式切换/内核启动、国内直连及网络恢复按钮按平台能力禁用；未将本机管理验证替代 Windows 成功运行或竞态证明，相关任务保持待验收。待答复的 Windows UI 技术授权与崩溃自动恢复语义不重复询问。当前 171/183，目标保持 active。

## 2026-10-04：Windows 原生用户套件实现（尚未执行 UI）

- 新增 scripts/e2e/ 的 W3C 客户端及四项真实页面场景、scripts/run-native-e2e.ps1 显式 UI 入口。默认拒绝创建 UI；必须使用独立 architecture-e2e 构建且与预检二进制 SHA 匹配。预检工具补充已安装 stable 环境与哈希，不运行套件。
- 场景通过页面元素执行代理添加/默认选择/规则启动、用户规则/国内直连/模式切换、文件输入和预览导入及网络恢复。配置导出保留两次真实原生窗口人工保存检查点，不直接调用业务 IPC；旧导出文件拒绝计为成功，两次完整 JSON 相等且不含密码才通过。
- PowerShell 包装器记录用户系统代理五字段的存在性、类型和值，结束后严格相等才允许通过；失败先走真实界面恢复，保存截图/失败报告，不覆盖未知外部设置。相关本机副本可能含 PAC/代理地址，保存在唯一临时目录，不加入仓库。
- Windows PowerShell 5 Parser.ParseFile 语法检查通过，未创建 UI 会话。新增 4 项协议/门禁/原生保存检查点工具测试通过；完整 pnpm check 通过（157 前端、8 工具），IPC 契约未漂移。首次 lint 对顺序 UI 轮询提出 no-await-in-loop，补充局部依赖说明后通过，未并发操作同一表单。
- 文档 docs/testing.md 与报告 reports/native-e2e-suite-preparation.json 明确人工检查点及未执行状态。实际 Windows UI 授权仍待答复，4.5.2–4.5.5、最终 E2E/Windows 用户场景保持未勾选，当前 171/183，目标保持 active。

## 2026-10-04：E2E 来源门禁与追踪测试隔离

- 修复 SkipBuild 来源门禁：旧记录/被其他构建覆盖的二进制不能重新获得隔离标识。启动驱动前检查既有成功记录、固定 identifier、应用 SHA 和配置 SHA；完整构建才生成新记录。Windows 实际验证旧记录拒绝、完整隔离构建/就绪通过、重复 SkipBuild 通过，并逐项注入应用哈希/配置哈希/标识不匹配全部拒绝，finally 恢复完整记录。全程没有 POST session 或 UI 操作。见 reports/windows-e2e-provenance.json。
- 默认并行 LLVM 覆盖率重跑发现 tracing 标识测试未取得 span ID，228 成功/1 失败，失败日志完整保留。单独运行原测试通过；改为独立子进程使用生产 subscriber，并额外核对两次错误日志与持久化诊断中 trace/span 标识一致。生产代码未改。macOS 和 Windows 定向测试均通过。见 reports/tracing-identity-isolation.json。
- 完整 Rust 回归通过：229 单元（7 忽略）、10 集成、1 示例、2 文档；完整 pnpm check 通过（157 前端、8 工具）。重新生成默认完整 LLVM 与前端 V8 报告，Rust 行 86.34%、前端行 92.91%、分支 81.08%；reports/final-current-coverage.json 更新当前数据，显式长时/Windows/UI 流程不由默认覆盖率证明。
- Windows 原生 UI 授权及自动恢复语义尚待答复，对应未完成项未改；当前 171/183，目标 active。

## 2026-10-04：真实固定内核强制退出恢复证据

- 新增 Windows 显式 ignored 的 runtime_crash_tests，实际 ApplicationService + ManagedRuntime + SingBoxRuntimeBackend、临时 SQLite 和独立租约。只终止实际会话返回的本测试内核 PID，隔离代理适配器不接管用户网络。
- Windows 固定 sing-box SHA 验证通过，强制退出后 251.553 ms 检测为 Failed/Exited、旧监听端口释放、网络恢复恰好一次；所选 Rules 与持久修订保留。显式重试启动健康的新 PID，停止后运行目录为 0。定向 Rust 测试退出 0，Windows 全目标 Clippy 通过，本地完整 pnpm check 通过。见 reports/windows-fixed-core-crash.json。
- 测试证明当前检测后网络恢复和显式重试行为，不证明自动重启、桌面宿主崩溃、真实 HKCU 或 UI 事件循环。4.4.3 自动恢复语义的待答复问题仍未替用户作决定，相关任务不勾选。当前 171/183，目标 active。

## 2026-10-04：剩余验收依赖核对

- 当前 12 项未完成要求逐项映射现有证据、缺失的原生平台证据及必要决定，见 reports/remaining-acceptance.md；未缩小任务范围，未增加勾选。核对 E2E 准备报告中全部五个源码 SHA 与当前文件一致。OpenSpec strict 与 diff 检查通过。
- 当前任务输入请求已明确列出两项必要答复：官方 Tauri WebDriver 技术指定及两次原生保存可用性；崩溃恢复为网络恢复加显式重试，或有限自动重启。CUA 工具规则限制其他 UI 技术，不能从 SSH 授权推导；套件与具体报告已准备，等待决定后实际执行。当前目标保持 active，未认定完成或阻塞达到三轮。

## 2026-10-04：用户确认 UI 技术与恢复语义

- 用户明确允许 WebDriver，并选择“自动恢复网络，手动重试启动”。继续执行 Windows 原生套件，两次保存保留操作员检查点。
- design 与 runtime-coordination delta spec 明确内核退出后的 Failed/Exited、自动网络恢复及显式重试，保留宿主持久恢复和代理归属保护。已通过的真实固定内核退出、恢复一次、新会话重试证据满足此语义，4.4.3 完成；原生 UI 错误流程仍单独待验收。当前 172/183。
- SSH 启动 Windows PowerShell 5 时继承 PowerShell 7 模块路径，导致哈希与签名命令加载失败。临时启动器固定本次进程的 Windows PowerShell 模块路径后进入隔离构建；未修改用户全局环境。
- SSH 非交互桌面创建 UI 会话超时，未执行场景；保留原始失败报告。随后通过仅本次验证用的交互登录计划任务，在用户 Session 3 启动同一隔离应用。实际 WebDriver 4.5.2/4.5.3 通过，报告 windows-native-e2e-partial.json；原生截图核对规则运行、健康会话、真实系统代理覆盖以及直连停止状态。当前 174/183，套件仍等待两次人工保存，未将局部通过认定为整体成功。
- 修正 PowerShell 5 读取 Node UTF-8 result.json 的编码，避免中文内容被误解码导致 JSON 解析失败。相关工具测试 4 项通过，OpenSpec strict 与 diff 检查通过。
- 原生保存等待五分钟后超时，目标目录未出现 before.json；用户报告第一次保存，实际文件位置仍待核对。最终 windows-native-e2e-result.json 保留失败原因、两项通过结果、界面恢复成功和 user_proxy_restored=true。真实用户五个系统代理字段的存在性、类型和值与运行前完全相等，敏感原始快照仍只存远端临时目录。
- 已核对真实原生截图中的规则模式健康/系统代理接管，以及连续添加→启动→规则配置→全局→直连过程，最终完整用户场景和 Windows 系统代理接管门禁完成。完整 E2E、配置往返、从运行中主动恢复、原生错误/手工竞态与 GUI 长时验证未认定完成。当前 176/183。
- 测试应用及驱动、Node 进程已退出；本次临时交互计划任务已删除。仍保留失败报告，不把人工保存超时转为成功。

## 2026-10-04：原生配置重跑与运行时验收扩展

- 用户愿意重新辅助保存，交互 Session 3 重跑现有来源校验通过的隔离应用。仅通过页面清理本套件的一条代理与规则，清理拒绝其他档案；本轮 before.json 已实际生成，临时编辑及真实文件输入/导入确认完成，仍等待 after.json。
- 新增 runtime 原生场景及只读进程树/注册表检查器：无效配置输入保留、快速模式操作/页面切换/保存一致性、固定摘要且监听端口归属匹配的直接内核子进程退出注入、五字段网络自动还原、显式重试，以及持续界面/隧道/资源采样。尚未执行运行时场景，不提前勾选。
- 增补套件重跑数据保护和实际 CONNECT/GET 双向探测工具测试，6 项 E2E 工具测试通过；完整 pnpm check 通过，Windows PS5 两个脚本语法解析通过。只读检查器已实际核对当前独立原生应用和全部 WebView 子进程，报告网络与本轮前一致，不输出原代理/PAC 地址。
- 第二次人工保存超时，未生成 after.json；完整套件仍失败且经页面清理后网络五字段完全还原，见 windows-native-e2e-assisted-retry.json。第一次导出/真实导入已执行，但完整往返相等未证明，不勾选 4.5.4。
- 原生 runtime 套件进入实际执行：无效端口反馈与输入保留、快速请求最终状态/页面切换/元数据不重启、真实固定内核退出后页面 Failed/Exited 及“系统代理已恢复”反馈、真实 HKCU 自动还原、新会话显式重试与双向代理隧道均通过。windows-native-runtime-partial.json 保留四项已通过的截图路径及观测；完整错误场景完成，当前 177/183。
- 强制退出旧 PID 7352，显式重试后 PID 48512；退出后运行目录 0，等待期间无自动重启。reported detected_in_ui_ms 包含页面检查和只读进程检查耗时，不能解释为精确退出检测延迟。
- 30 分钟 GUI/WebView/真实用户系统代理稳定性正在 Session 3 实际运行；远端报告目录 socks-native-e2e-946f225b5d2647b0b78dd4e1a872387a。未完成前不勾选稳定性及最终主动恢复场景。
- 人工核对 Windows 实际原生截图的规则运行/健康/页签/底栏一致和全局直连停止/未接管，补齐此前 macOS CUA 管理验收所缺的平台运行证据。native-management-manual.md 记录具体证据组合，1.7.3 完成；用户未被要求再批准已授权的界面验证。当前 178/183。状态竞态和长时验收仍待本次完整结果。
- 原生快速模式选择→页面切换→最终状态核对、运行中名称保存→列表/页脚/状态一致→PID 保持，以及持续 16 次模式/8 次元数据转换均通过；人工审查实际截图并保存 native-state-sync-manual.md 和两张原生图。实际验收结合既有并发回归满足 3.6.4，不必等待独立的 30 分钟稳定性要求才认定这些场景。当前 179/183，长时测试仍在运行。
- 本轮原生稳定性完整通过：1801.555 秒、1262 隧道探测、59 模式转换、29 元数据保存、180 资源采样。所有样本单内核/单运行目录，最终原生主动恢复后内核与运行目录为 0、用户系统代理五字段前后完全相等，测试/驱动进程退出。完整结果、统计与人工终态截图见 windows-native-runtime-final.json、windows-native-runtime-summary.json、windows-native-stability.md、windows-native-network-recovery.png。4.5.5 和最终长时稳定性完成，当前 181/183；内存观察范围和局限明确保留。
- 用户回复“我准备好了”，已直接启动独立 journey 重测，来源校验复用有效预检，每次人工保存等待改为十五分钟。本轮目录 socks-native-e2e-9be528846b28453a874a070bb9d24cb0，第一次保存输入请求已发出。完整配置往返和完整套件仍等待实际终态，不提前勾选。

- 本轮 before.json 实际生成 831 字节，schema v2、无密码字段；真实导入后原生页面代理名恢复，第二次 after.json 仍等待人工保存，会话 e79148bb398930978e2dc20fcd334b16 确认在线。未将局部结果计为配置往返通过。
- 最终审查发现验收工具 4 条 ESLint 抑制注释未匹配 ESLint 规则，已改为 Oxlint 专属指令，保持顺序 UI/轮询行为。完整 pnpm check 退出 0，157 前端/10 工具、类型/格式/lint/Rust fmt/clippy/IPC 契约全部通过且无 lint 告警，日志 /tmp/architecture-final-no-warning-check.log。当前原生运行使用改注释前脚本，行为相同，既有报告来源哈希仍记录实际执行版本。

## 2026-10-04：最终 Windows 原生往返与完整验收

- 用户完成第二次原生保存，本轮 before.json/after.json 均实际生成 831 字节。套件真实临时修改→文件输入/确认导入→再次导出，完整 JSON 和原始字节相等、v2/无密码字段；独立本地再次比较和 SHA 校验通过。
- journey 四项全部 passed，外层 NATIVE_E2E_PASSED，user_proxy_restored=true；windows-native-e2e-final.json、final-summary.json、windows-native-journey.md 及两张终态截图保存证据。早期失败报告保持。
- 独立只读复查五字段网络还原、测试应用/内核/驱动/Node 退出、运行目录 0；临时计划任务在 Ready 后删除，复查 absent。原始用户代理/PAC 字段值仅留远端临时目录。
- 4.5.4 和最终完整 E2E 完成，当前 183/183。runtime 六项完整通过及 30 分钟稳定性证据保持；最新质量检查无告警、OpenSpec strict 和格式/diff 最终核对进行中。没有提交、推送、发布或归档。

- 最终 OpenSpec strict、全仓格式及 git diff --check 通过。已复查全部 183 项任务、分阶段实现/测试证据索引、最新覆盖率与性能门禁，以及两个完整原生套件和独立清理审计。最新质量和完整 Rust 日志使用 .txt 保留，避免被 *.log 忽略；最终进度 183/183。
