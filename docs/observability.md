# 运行时追踪

应用获得会话所有权并创建独立应用数据目录后安装 tracing subscriber。日志写到该应用数据目录的 `logs/runtime.jsonl`，每行一个 JSON 事件，包含级别、时间、目标、字段和 span 上下文。开发构建额外在终端输出美化日志；生产构建只写 JSON Lines 文件。`RUST_LOG` 控制级别与目标过滤，未配置或无法解析时使用 info。

文件写入使用最多 1024 条记录的异步队列；队列满时丢弃追踪记录，避免阻塞业务线程。快照中的 trace_dropped_records 显示本次进程累计丢弃数量。正常退出由管理的 WorkerGuard 刷新队列；进程强制终止可能丢失尚未写入的记录。

日志文件超过 1 MiB 时轮转，保留当前文件和最多 4 个备份（runtime.1.jsonl 到 runtime.4.jsonl）。轮转只操作固定日志文件，不删除目录内其他文件。Windows 重命名前关闭文件句柄；轮转失败会尝试重新打开当前文件。初始化失败输出固定说明，保留应用的配置访问与网络恢复能力。

服务方法使用 debug 级别、skip_all 的 instrument span，配置事务在 debug 记录 begin/commit，info 记录 cleanup 及结果，warning 记录 rollback，状态机在 info 记录生命周期转换，在 debug 记录不改变阶段的 metadata_committed；有界转换历史始终保留所有事件。日志记录事件、前后阶段、操作编号及接受/拒绝结果。系统代理接管/恢复使用专用 span。应用错误记录错误编号、根因领域、变体及操作名；ErrorContext 中的 trace_id 标识本地错误因果链，span_id 标识关联的本进程 span，跨领域转换保持原身份。未安装 subscriber 时 span_id 缺省；旧记录保持原格式。错误诊断的结构化 summary 中保存这些上下文，不收集外部请求的追踪头。诊断写入失败使用同一编号关联。

追踪不记录函数参数、配置对象、地址、代理名称、字段值、错误消息、凭据或恢复记录内容。只允许固定操作名称、阶段、计数、修订和随机事务/错误编号。新增 span 必须使用 skip_all，并在脱敏审查后显式添加字段；不要使用 instrument 的 err/ret 自动记录业务值。

单元测试验证 JSON/span 上下文、过滤回退、文件重开追加、备份数量、完整 JSON 记录和无关文件保留；真实 Windows 文件写入及原生日志验收须单独记录。

## 耗时指标

| 操作                                          | 测量区间                                                                            |
| --------------------------------------------- | ----------------------------------------------------------------------------------- |
| startup_frontend_ready                        | 进入 Rust run 到初始快照加载、页面挂载且跨两个可见帧后 Rust 收到就绪确认            |
| startup_bootstrap                             | 进入 Rust run 到 Tauri setup 完成                                                   |
| configuration_load                            | SQLite 配置读取及解码                                                               |
| configuration_apply                           | 配置校验、持久化、运行时确认与资源清理                                              |
| runtime_start / runtime_switch / runtime_stop | 请求校验、后端候选准备及即时确认；持久配置事务的最终提交由 configuration_apply 覆盖 |
| system_proxy_enable / system_proxy_restore    | 所有权读取、设备设置与日志提交/清理                                                 |

使用单调 Instant 测量毫秒，成功和失败均记录。每种固定操作保存累计次数、失败次数、总耗时和最大耗时；P50/P95 来自最近 128 次样本。读指标不触发 I/O，重启后重置。debug 性能事件包含 operation、elapsed_ms 和 succeeded。startup_frontend_ready 由后端 Instant 计时，包含 WebView 加载、初始数据读取和就绪确认 IPC；每个原生进程只记录首个确认，窗口重载不覆盖。初始化失败或持续隐藏时不会产生虚假的就绪样本。两个可见帧是就绪协议，不能证明实际像素首绘，也不包含操作系统创建进程的耗时。startup_bootstrap 不包括操作系统进程创建、页面加载、首屏就绪或 JavaScript IPC 往返，不替代真实性能验收。

## 只读诊断工具

新增 Tauri 命令 `export_runtime_snapshot` 返回 JSON 字符串，含运行时快照、最近 128 次转换和耗时指标；`validate_configuration` 校验持久配置并报告配置修订、恢复阻塞及运行时健康状态。原命令签名保持不变。两项工具不读取凭据或导出代理输入，不执行配置修改；配置有效不等于代理联网成功。

设置页支持按级别与字面文本搜索诊断编号、摘要、错误类型或操作；最多 128 个字符，ASCII 英文字母不区分大小写。列表、聚合、导出和明确确认的清理共用筛选语义，百分号和下划线不作为通配符。

查询日志：`node scripts/query-runtime-log.mjs <runtime.jsonl> --level ERROR --error-id <id>`。支持 operation、event/stage、since ISO 时间与 contains 字面搜索，多条件取交集。使用流式读取，坏 JSON 行计数并跳过；输出仍为 JSON Lines，不执行记录里的内容。脚本测试纳入 pnpm check。

即使 RUST_LOG=trace，也只接受本应用目标的事件，第三方网络库日志被过滤，避免其自行记录请求地址或参数。

## 火焰图

设置 SOCKS_PROXY_PROFILE=1 后启动开发应用，在固定的 logs/profile.folded 写入 tracing span 的 folded 栈；退出应用时管理的 guard 刷新缓冲。只记录 span 名及线程/模块结构，不记录参数或源码路径。默认关闭该采集，profile.folded 是本次分析产物，每次启用重新创建。

使用标准 Inferno 工具生成 SVG：`cargo run --manifest-path src-tauri/Cargo.toml --example runtime-flamegraph -- <profile.folded> <flamegraph.svg>`。生成器使用临时文件原子替换输出，读取失败保留原文件。图中的权重是 span 间经过的纳秒时间；它反映被 instrument 覆盖的工作与等待，不等同于 CPU 采样或完整调用栈。
