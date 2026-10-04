## Environment

2026-10-04 使用用户提供的 OpenSSH 访问 Windows x64 验收机。Node 22.22.2、pnpm 10.33.0、PowerShell 5.1.26100.9444、Windows 10.0.26200.0、WebView2 154.0.4258.53。安装 Rust stable 1.99.0 及 rustfmt/clippy，验证进程使用 `RUSTUP_TOOLCHAIN=stable`，未修改全局默认 Rust 工具链。

验收目录为 `C:\Users\lqy15\socks-proxy-acceptance\harden-7c25d8c\repo`。当前工作区复制到本地临时克隆后生成 Git bundle；未复制 `.cargo/config.toml`，快照阶段未向 GitHub 推送；交付分支和 Actions 记录将在下文补充。远端以 `git -c core.autocrlf=true clone` 干净检出，初始 `git status --porcelain` 无输出。快照提交只存在于验收克隆，不是原工作区的交付提交。

## Completed Evidence

- 首轮快照 `7c25d8c64eb13c54846339a254be15e02117fdf9`：Windows `pnpm format:check` 通过，`git ls-files --eol` 无 CRLF/mixed 工作区文本；19 个二进制资源哈希全部匹配基线 `binary-hashes.json`。1.1 已完成。
- 固定 sing-box 1.14.1 的路径、版本、EXE SHA-256 校验通过；资源准备校验 DLL。两项门禁负向测试通过。
- 首轮真实内核 Cargo：164 passed、0 failed、4 ignored 辅助测试。六个配置中断边界及 Job 宿主退出测试实际通过，验证受管进程退出、端口释放、目录回收和恢复幂等。5.5 已完成。凭据及启动项采用合成适配器，这与实际 OS 适配器验收分开记录。
- 首轮前端默认并发导致 6 项异步 UI 等待失败；限制 2 个 DOM worker 后相关 25 项及完整 99 项通过。固定进程较多的 Rust 门禁同样限制 2 个测试线程，CI 与真实内核入口采用一致配置。
- 修复 Windows 非测试编译中的未使用字段/方法警告：控制端口观测字段和测试便捷请求方法仅在测试中编译。
- 真实内核扩展测试覆盖测速取消时清理测试资源且主内核保留、慢控制请求期间停止，以及候选切换失败后旧内核退出的恢复。切换时序由受控内核验证；真实内核的完整性校验和启动耗时不纳入控制锁等待断言。
- 真实上游请求到达顺序不能由本地 CONNECT 应答顺序推断。国内直连测试验证两个预期代理目标均到达上游，避免错误的固定到达顺序假设。

## Current Verification

最新验收快照 `00e72c0e1f849f859ad046abc75cd898c59de7ff` 已通过完整 `pnpm check`（99 项前端测试及全部静态/契约检查）、单元 Cargo 和固定真实内核门禁。真实内核为 165 passed、0 failed、5 ignored；日志见 [windows-real-core.txt](evidence/windows-real-core.txt)。没有把可选内核缺失时返回的单元测试算作真实内核证据。

Windows 交互式用户会话 ID 3 显式执行 `configuration_startup_recovery::windows_acceptance::actual_keyring_and_startup_recover_both_commit_sides_and_preserve_external_value`，结果 1 passed、0 ignored、退出码 0。使用真实 Windows 凭据库与 SystemStartupAdapter，重开数据库后验证旧/新提交边界、外部修改保留及幂等；唯一命名的合成凭据和临时启动项清理完成。日志见 [windows-os-recovery.txt](evidence/windows-os-recovery.txt)。8.2 已完成。

Windows 生产不打包构建与 NSIS 构建均退出 0，日志见 `evidence/windows-build.txt` 和 `evidence/windows-nsis.txt`。NSIS 安装至独立验收目录退出 0，安装后固定内核哈希匹配。安装包为 `Socks Proxy_0.1.0_x64-setup.exe`，SHA-256 为 `43b34e644ea5dfbb00e2c499ee6203be145ca8add2ae857600128d5b545c88d3`，完整元数据见 `evidence/windows-installer.json`。

安装后的应用在用户会话 3 中运行，真实 WebView2 的 UI Automation 树包含状态页、默认档案、运行时健康和模式控件；见 `evidence/windows-launch.json`。首个夹具错误使用 HTTP 测速地址导致启动校验拒绝，改为合法 HTTPS 后启动成功，没有放宽生产校验。

桌面验收已完成，8.3 已勾选。WTSInfoEx 确认会话 3 为 active/unlocked（sessionFlags=1）；早先仅凭 LockApp 或捕获画面判断锁屏不准确。实际自动化问题是 DPI 坐标和原生弹出菜单事件：校准 DPI 后使用 WebView UIA 模式控件，托盘退出使用原生菜单物理点击（UIA Invoke 不会返回 TrackPopupMenu 选择）。没有将壁纸或锁屏捕获作为应用截图。

- 安装启动：安装包应用在交互式会话 3 启动，真实 WebView 显示合成档案和状态；实际选中全局模式后健康提交并启动固定内核。见 `evidence/desktop-launch-result.json` 和 `evidence/desktop-global-result.json`。
- 正常托盘退出：实际原生菜单选择退出，持有的宿主和固定内核句柄均退出，系统代理五项的存在性、类型及原值逐项恢复，私有运行目录为空。见 `evidence/desktop-tray-result.json`。
- 异常退出：重新启动并选择规则模式后强制结束宿主；Windows Job 终止固定内核且端口可重新绑定。此时观察到遗留代理与私有目录，证明没有依赖宿主析构清理。再次启动已安装应用后，系统代理精确恢复、遗留目录清理、保存档案保留、状态为未应用且没有抢先启动新内核。见 `evidence/desktop-crash-before-restart.json`、`evidence/desktop-crash-result.json` 和 `evidence/desktop-restart-result.json`。
- 最终恢复：重启后的应用经实际托盘菜单退出，测试包卸载，临时计划任务删除；用户原 Roaming 数据库及历史备份、Local WebView 数据恢复到原位置，没有遗留验收宿主或内核，系统代理与测试前快照完全一致。原用户数据仅保留在 Windows，不作为验收证据复制。见 `evidence/windows-desktop-cleanup.json`。

1.3 的当前 GitHub Actions URL 尚待补齐，不以 SSH 的成功结果替代 Actions。整体为 29/30。
