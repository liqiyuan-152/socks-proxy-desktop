## Execution Preconditions

本清单用于目标平台验收，不是已执行记录；实际结果见 [windows-execution.md](windows-execution.md)。目前 1.1、5.5、8.2、8.3 已完成，1.3 的 Actions 记录亦已完成，整体 30/30。要求 Windows x64、Node >=22.22.0、pnpm >=10.33.0、Rust 工具链与真实桌面会话。使用包含本变更的确定源码修订；当前未提交工作区不能由旧的 GitHub Actions 结果代替。

在干净目录克隆该修订，记录 `git rev-parse HEAD`、`git status --short`、Git 换行设置、Windows 版本、Node/pnpm/Rust 版本。不得复制开发机 `.cargo/config.toml`。Windows 默认检出下运行格式检查；另在 `core.autocrlf=true` 的干净检出复核。比较所有已跟踪二进制资源升级前后 SHA-256，不以文件大小或格式工具成功代替哈希。

## Automated Gates

PowerShell 中逐条执行并检查 `$LASTEXITCODE`；任何非零退出即失败，禁止继续执行后用最后一步成功覆盖前一步失败。

```powershell
node --version
pnpm --version
rustc --version
pnpm install --frozen-lockfile
pnpm check
cargo test --manifest-path src-tauri/Cargo.toml --locked
node --test scripts/test-real-core-checks.mjs
pnpm prepare:core
$env:SING_BOX_TEST_BIN = Join-Path (Get-Location) 'src-tauri/resources/sing-box/windows-amd64/sing-box.exe'
node scripts/test-real-core.mjs
pnpm tauri:build:check
pnpm tauri build --bundles nsis
```

固定内核必须为 1.14.1；EXE SHA-256：`b838de45bd0b2e6ddbed1977e4745622f7dffab3b293807ff4c6b1b640fed909`；DLL SHA-256：`3217c6260fbca5f16072e0b79735742f40109a63bb0ff88fd6b96dd6b54a2928`。准备脚本验证资源，真实内核门禁再次验证固定可执行文件路径/版本/哈希，并启用必需模式。缺少内核时只能记录失败或未执行，不能记录正向验收通过。

Quality 工作流负责干净检出的完整检查与不打包构建；Windows test package 工作流另外构建 NSIS，提供 `SOURCE_REVISION.txt`、`SHA256SUMS.txt`。保存两个工作流中实际对应当前源码的运行 URL、各阶段结果以及产物标识。

## Crash and Resource Scenarios

- 检查完整 Windows Cargo 输出实际执行 `interrupted_cross_resource_commit_recovers_exact_durable_boundary` 和 `abrupt_owner_exit_kills_job_child_without_rust_destructors`。被默认忽略的辅助子进程测试由父测试显式调用；只看到辅助测试 ignored 不能证明或否定父测试结果。
- 六个中断边界为恢复意图、凭据暂存、启动项应用、候选内核切换、SQLite 提交、清理前。父进程须在宿主退出前持有实际受控子进程句柄；核对 Job 子进程退出、端口释放、临时目录回收以及再次恢复幂等。Windows 分支使用受控内核，这一结果与固定真实内核场景分别记录。
- 确认完整真实内核输出执行 `fixed_core_applies_password_username_import_and_preserves_credentials_after_failure`，核对真实认证头、修订和失败后凭据保持。
- 以固定真实内核验证仅替换用户名/密码、相同非敏感配置导入不同凭据后实际生效；需要可观察认证的测试代理，记录行为而不保存真实秘密。
- 验证候选失败时旧会话继续运行，其后退出仍触发恢复；慢控制响应期间切换/停止不等待读取锁，迟到响应不能覆盖新会话。
- 单测/批测混用，取消所有订阅或更新输入修订后检查测试内核和目录已回收，主内核不受影响。
- 使用实际 Windows 凭据/启动项适配器演练恢复；出现外部启动项或代理设置修改时保留外部值和日志，恢复状态可见。合成文件资源测试不能替代这一步。

## Desktop Scenarios

记录 Windows 构建号、WebView2 Runtime 版本、安装包 SHA-256、源码修订、时间与操作结果。使用专用测试代理与测试数据，截图/日志不包含密码或控制密钥。

1. 安装并启动 NSIS 内测包，检查档案持久化、认证编辑、规则与国内直连预设；启动升级前备份和回退要求见 `rollback.md`。
2. 切换全局/规则/直连，核对实际出口、系统代理覆盖说明、当前生效模式、失败反馈和托盘选中状态。外部修改系统代理后，恢复入口不得覆盖无法确认归属的值。
3. 托盘正常退出后检查应用、受管内核和私有运行目录退出/清理，系统代理恢复到应用接管前的值。
4. 活动代理期间强制结束宿主，再启动；检查 Job 子进程未遗留、启动恢复先于新接管、恢复失败阻止写入，并能从恢复入口重试。
5. 测速期间退出/重入，编辑目标/端口、保存规则/默认出口/预设后旧路由预测清除；预测界面仍说明不代表真实连接。

## Evidence Record Template

| 字段     | 待填内容                                       |
| -------- | ---------------------------------------------- |
| 源码     | 完整提交 SHA，工作区是否干净                   |
| 系统     | Windows x64 版本/构建号，WebView2 Runtime 版本 |
| 工具链   | Node、pnpm、Rust、Git 版本及换行设置           |
| 内核     | 版本、EXE/DLL 哈希、固定路径验证结果           |
| 自动检查 | 每条命令退出码、完整日志位置、对应 Actions URL |
| 安装包   | 产物标识、源码记录、SHA-256                    |
| 崩溃场景 | 每个中断点、父/子进程退出与恢复结果            |
| 桌面操作 | 每个场景实际结果、脱敏证据、未执行原因         |

证据填写后再更新 `tasks.md`；没有对应平台、源码和实际结果的条目继续保持未完成。
