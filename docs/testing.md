# 测试指南

运行命令前确认 Node ≥22.22.0、pnpm ≥10.33.0。完整静态检查使用 `pnpm check`，Rust 单元、独立集成、属性和文档测试使用 `cargo test --manifest-path src-tauri/Cargo.toml --locked`。较慢机器可设置 `RUST_TEST_THREADS=2`，避免外部进程测试与编译争用资源。

## 独立集成测试

`src-tauri/tests/architecture.rs` 注册 `integration/` 中的场景，`support/` 提供 TestApp、MockBackend 和 InMemoryCredentialStore。TestApp 使用临时目录中的 SQLite 数据库、真实 ApplicationService 和 ManagedRuntime，测试结束自动释放数据库、独占会话和目录。MockBackend 控制失败、进程退出、确认及撤销；它遵守候选会话提交协议，不模拟真实系统代理接管。

```rust
use crate::support::TestApp;
use socks_proxy_lib::services::RuntimeMode;

#[test]
fn starts_selected_proxy() -> Result<(), Box<dyn std::error::Error>> {
    let app = TestApp::new()?;
    let profile = app.add_profile("test")?;
    app.service.select_profile(Some(profile.id))?;
    let snapshot = app.service.request_mode(RuntimeMode::Rules)?;
    assert_eq!(snapshot.applied_mode, Some(RuntimeMode::Rules));
    Ok(())
}
```

只运行该套件：`cargo test --manifest-path src-tauri/Cargo.toml --test architecture`。新增场景应验证用户行为及持久化结果，不只比较内部字段。用 Result 传播初始化失败；对预期拒绝使用 `expect_err`。每项测试使用独立 UUID 会话和临时目录，避免执行顺序依赖。凭据仅使用虚构测试值，确认序列化配置不包含密码。

## 状态机与前端

转换矩阵覆盖 12 种节点形态的 17 类事件。proptest 随机生成事件序列，验证每次接受转换的不变量、候选不提前应用和拒绝时旧节点不丢失。属性失败会保存最小复现输入；修复时先重跑失败种子。运行 `cargo test --manifest-path src-tauri/Cargo.toml runtime_state_machine`。

前端 `src/store/` 测试使用延迟 Promise 验证资源竞态、串行模式队列、卸载与订阅清理；假计时器验证窗口可见性轮询。Provider 测试包括 StrictMode 和选择器更新隔离。页面测试继续验证代理、规则、导入和错误详情。不要用固定延时替代 Promise 门控，也不要放宽生产超时来让测试通过。

## 覆盖率及平台边界

`pnpm test:coverage` 运行前端全套测试并生成 store 的文本、JSON 摘要及 LCOV 到 `coverage/`。安装 cargo-llvm-cov 和 llvm-tools-preview 后，使用 `cargo llvm-cov --manifest-path src-tauri/Cargo.toml --lcov --output-path rust-coverage.lcov` 生成 Rust 单元和集成覆盖率。新代码覆盖率目标 ≥80%；覆盖率不能证明测试断言充分，也不能证明未编译平台分支。

GitHub Quality 工作流在 Windows 运行全部 Rust 测试；独立 coverage job 上传 Rust LCOV 和前端 store 报告。真实内核测试须先 `pnpm prepare:core`，然后使用仓库 `scripts/test-real-core.mjs` 的固定版本/摘要校验。CLI 模拟 IPC、真实内核测试和原生 UI 验收应分别记录；文件选择器成功/取消、系统代理接管及原生 JavaScript IPC 往返不能由 mock 测试代替。

## 真实 Windows 用户代理恢复

仅在已授权的验收环境执行：设置 `$env:SOCKS_PROXY_LIVE_PROXY_VALIDATION = "1"`，运行 `cargo test --manifest-path src-tauri/Cargo.toml live_user_proxy_restores_original_settings_after_stop_and_reopen -- --ignored`。普通回归忽略此测试。

测试持有生产运行时会话锁，短暂修改真实 HKCU Internet Settings 到本地监听端口，并验证停止和重新创建 adapter/SQLite 后完整恢复。退出 guard 仅恢复仍可确认归属的设置，其他程序改动不会被覆盖。测试不输出原代理或 PAC 内容，不调用外部网站；它不代替真实内核、进程崩溃或原生 UI 验收。

## 原生 E2E 环境与长时验证

Windows 使用官方 tauri-driver 2.1.0 与和 Edge 精确匹配、Microsoft 数字签名有效的 msedgedriver。运行 `powershell -File scripts/check-tauri-e2e-environment.ps1` 安装驱动、使用 `src-tauri/tauri.e2e.conf.json` 构建独立应用标识，并读取 WebDriver `/status`；工具只监听 loopback，不创建 UI 会话，最终检查端口归属并关闭本次驱动。二次运行可用 `-SkipBuild`，仅限已使用隔离配置构建的二进制。驱动与本地报告保存在被忽略的 `.e2e-tools/`。服务就绪不代表用户 E2E 场景已经通过，后续仍需实际套件与原生界面授权。

`-SkipBuild` 必须已有成功预检记录，且应用标识、二进制 SHA-256 和 E2E 配置 SHA-256 均保持一致；其他构建覆盖二进制、配置变化或旧版记录缺少哈希时，在启动驱动前拒绝。重新执行完整预检才能取得新的隔离构建记录。

`powershell -File scripts/run-core-soak.ps1 -DurationSeconds 1800 -ReportDirectory C:\Temp\socks-soak` 显式运行真实 sing-box 1.14.1 与应用服务测试，并采样测试宿主/其直接内核子进程的工作集、私有内存、句柄、线程和 CPU。测试每秒验证合成 HTTP CONNECT 隧道双向数据，每 30 秒交替模式并提交元数据，每四次转换停止/重启；检查配置修订一致、健康声明及运行目录无泄漏。使用临时 SQLite、本地上游和隔离代理适配器，不写用户真实系统代理。退出或断言失败由生产内核 Drop/Windows Job 关闭其会话；运行器超时只结束自己创建的测试宿主。该报告不含 GUI/WebView，也不替代桌面内存或原生稳定性验收。

## Windows 原生用户场景套件

`scripts/run-native-e2e.ps1 -RunUi` 会真正创建 Windows WebDriver UI 会话。必须在已获得原生 UI 自动化授权后运行；不传 `-RunUi` 会拒绝执行，环境预检不会调用它。脚本重新构建 architecture-e2e 独立应用，记录二进制 SHA-256；Node 入口要求与预检记录匹配，拒绝误用常规应用构建。请先退出其他 Socks Proxy 实例并确认隔离应用没有旧代理或规则；非空配置会拒绝进入新增场景，不自动删除数据。

在用户已登录并解锁的交互桌面执行入口。SSH 非交互会话可能让 `/session` 超时；远程执行时应将测试进程放到该用户的交互会话，并让操作员能看到保存窗口。Windows PowerShell 5 若继承 PowerShell 7 的模块目录，需在测试启动进程中使用对应版本的模块路径，避免哈希/签名命令加载失败；不修改用户全局环境。Node 报告为 UTF-8，PowerShell 5 读取时显式使用 `-Encoding utf8`。

再次运行同一隔离套件可显式传入 `-ResetFixture`：只接受一条名称匹配的套件代理和规则，通过界面恢复网络、取消默认、关闭预设并删除；遇到其他数据即拒绝。`-SkipBuild` 仍必须通过预检的二进制与配置摘要门禁。

`-Suite runtime -DurationSeconds 1800` 在已有套件档案上更新本轮合成上游端口，执行无效配置/输入保留、快速模式选择、页面切换、元数据保存不重启、真实内核退出/自动恢复网络/手动重试，以及 30 分钟原生桌面与 WebView 稳定性验收。实际进程树采样包括工作集、私有内存、句柄、线程，持续验证本地 CONNECT 隧道双向数据、界面健康与代理归属，最后由页面恢复网络并核对五字段原值。崩溃注入只接受隔离应用直接创建、固定摘要匹配且拥有当前代理监听端口的内核 PID，并在终止前重新核对创建时间。原始注册表快照只留本机临时目录；报告输出是否还原，不输出原代理/PAC 地址。持续时间可缩短用于诊断，但短运行不能作为 30 分钟稳定性证据。

套件通过 W3C 元素点击/输入访问页面，不执行 JavaScript，不直接调用业务 IPC。按顺序验证添加本地 HTTP 代理及规则模式启动、域名直连规则和国内直连开关/私有 IP 匹配、规则→全局→直连、配置导入往返，以及从运行状态执行确认网络恢复。上游仅在 loopback 临时端口监听，不使用真实账号，也不证明公网代理连接。

配置导出使用真实 OS 保存窗口，操作员需要两次保存到终端打印的绝对路径。每次最多等待十五分钟；旧文件被拒绝，只有新文件可解析且界面确认保存才通过；再次导出后的完整 JSON 必须相等且不能含 password 字段。导入使用真实 file input 和预览确认。此套件属于带人工保存检查点的原生验收，不宣称原生窗口完全无人值守；文件选择窗口本身另需 CUA/手工验证。

结果、截图、驱动日志和注册表前后副本位于唯一的系统临时目录。脚本仅读取用户级五个系统代理字段，结束后严格比较字段是否存在、类型和值；失败时先通过真实界面的网络恢复操作清理，不直接覆盖注册表。副本可能含本机原代理/PAC 地址，仅留本机，不加入仓库报告。恢复失败会将结果标为 failed 并保留诊断，需继续处理该本机状态。

`pnpm test:tools` 包含本地模拟 W3C 服务测试：延迟元素/陈旧引用重试、输入读取、会话清理、不可重试错误/超时、UI 启用门禁，以及人工保存检查点拒绝旧文件/密码字段。模拟测试不启动桌面应用，不能替代 Windows 实际场景验收。进程崩溃自动恢复、运行中竞态和 GUI 长时稳定性仍需独立实际证据。

## 固定 Windows 内核强制退出

`runtime_crash_tests::fixed_core_abrupt_exit_restores_isolated_network_and_explicit_retry` 是显式 ignored 场景。Windows 设置 `SING_BOX_TEST_BIN` 为仓库固定的 sing-box.exe，并设置 `ARCHITECTURE_CRASH_OUTPUT` 为本机报告绝对路径后，用 cargo test 的 `--ignored --exact` 运行完整测试名。测试核对固定 SHA-256，使用临时 SQLite、独立会话租约、本地上游及隔离系统代理适配器。只使用实际 backend 会话返回的本次内核 PID 执行 taskkill；不终止其他程序，不操作原生 UI，也不接管用户系统代理。

测试每 250 ms 读取真实应用服务快照，验证退出后 Failed/Exited、已应用模式清空、所选模式与持久修订保留、旧监听端口释放、网络恢复仅一次。显式重试必须产生不同 PID 的健康新会话；停止后运行目录清空。它证明当前“检测后恢复网络、显式重试”的实际内核行为，不把该流程描述为自动重启，也不代替桌面宿主崩溃、真实系统代理或 UI 事件循环验收。
