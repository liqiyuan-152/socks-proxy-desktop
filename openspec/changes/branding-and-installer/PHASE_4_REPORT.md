# Phase 4 实际平台验收记录（进行中）

日期：2026-10-05。未完成全部系统视觉及安装交互验收，不宣告阶段完成。

## macOS 已执行

环境 macOS 26.6.2，Apple Silicon。官方 DMG 已挂载，Finder 图标视图中品牌图标、Applications 替身、中文拖拽提示及 Retina 背景可读，图标位置正确，箭头不与系统图标或标签重叠。箭头实际采用 (230,150)→(365,150)，与设计规范同步，避免覆盖 Applications 图标。

通过 Finder 复制安装，将已有 0.1.0 应用更新为 0.2.1 验收包；旧应用另行备份。更新后应用启动正常。原生 About 窗口显示品牌图标及 Version 0.2.1 (0.2.1)。用户 SQLite 的 configuration 和 selected_mode 表与启动前备份逐记录一致，未输出配置内容。

退出应用后通过 Finder 移入废纸篓，确认 Applications 中的程序移除、用户数据库仍存在且上述两张表记录一致。随后从挂载 DMG 重新复制到 Applications 并启动，记录仍保持一致。未清空废纸篓。

## 尚缺证据

- 拖拽安装证据已在后续补充；Dock、系统应用列表和菜单栏明暗主题仍待验证，4.2 尚未全部完成。
- Dock、SystemUIServer 和 macOS 26 的 App 系统应用列表无法由当前 CUA 获取，调用超时。Dock、系统应用列表及菜单栏明暗主题视觉仍待验证；本机系统已使用 App 替代旧 Launchpad。
- Windows 已通过 SSH 启动安装器，由用户辅助验收；用户确认安装页面正常、应用窗口已打开。卸载页已在后续补充验证，取消桌面快捷方式选项及托盘菜单尚未确认。
- 用户选择由助手经 SSH 执行安装，由用户辅助确认可见界面；无需安装 RDP 客户端，助手不通过 SSH 自动化点击原生界面。
- Linux 重跑 CI 已成功：元数据、DEB 安装、原生前端就绪、卸载保留配置和 AppImage 独立启动通过。桌面菜单及系统图标视觉、旧版升级和 RPM 安装仍未验证。
- 浏览器生产预览可打开，favicon 声明及 HTTP 返回内容与品牌 ICO 一致；当前标签页截图无法明确辨认图标，视觉验收仍待完成。

## 验证边界

模板检查、资源预览、代码及构建成功不能替代实际界面交互。任务清单保留尚未证明的项目，不以静态检查标记全平台验收完成。

用户确认没有可用的 Linux 图形桌面。本机检查未找到 docker、podman、multipass、qemu-system-x86_64、limactl 命令或可检索的常见虚拟机应用 Info.plist；未新增宿主运行环境。Linux 桌面菜单与系统图标视觉仍缺实际环境，4.3 不标记完成，CI 的 Xvfb 启动结果不替代该要求。

## macOS 拖拽安装补充

后续验收通过 Finder 图标视图，从已挂载 DMG 的 Socks Proxy 图标拖拽到 Applications 替身。Finder 实际显示“此位置已经存在名称为 Socks Proxy 的项目”的替换提示；选择替换后操作完成，已安装应用可以启动。读取 Applications 中的 Info.plist 确认版本为 0.2.1，界面显示相同版本。再次将用户数据库的 configuration、selected_mode 全部记录与最初备份逐记录比较，两表一致。此步骤证明当前 DMG 的拖拽替换和启动可用；其他系统图标与主题项目继续保留待验收。

本机虽已有 RustDesk，但 Windows 目标的直连端口 21118 不可连接。未更改远端服务或开放端口。后续用户选择通过 SSH 安装并辅助确认界面，该通道限制不再阻塞此协作方式。

## Windows 辅助验收与图标修复

安装包 SHA256 与 CI 的已核验 EXE 一致。安装前备份现有应用数据，安装器通过临时交互任务启动于 lqy15 的控制台会话。安装完成返回 0，注册版本 0.2.1，用户确认安装页面、品牌图和第三方说明正常，应用窗口已打开。临时安装任务已移除。

用户报告桌面快捷方式仍显示旧图标。提取已安装 EXE 的关联图标，确认其嵌入的是新盾牌网络图标；快捷方式原先没有显式图标路径。配置独立 brand-shield.ico 后，用户刷新桌面并确认新图标显示正确。

永久修复将品牌 ICO 作为 Windows 安装资源，并使用最小 NSIS hook：POSTINSTALL 更新已创建或升级的桌面与开始菜单快捷方式；onGUIEnd 处理官方完成页随后创建的可选桌面快捷方式。仅修改指向本次安装的已有链接，不创建被用户取消的快捷方式，不改写目标路径或其他属性。保留官方 AppUserModelId、升级、卸载及快捷方式选项逻辑；卸载清单包含新增图标文件。

真实 Windows 官方 makensis 已编译修复版。将两个现有快捷方式图标属性恢复到旧值后，通过 /S /UPDATE 更新模式安装，返回 0；安装器实际将两条链接设置为安装目录下的 brand-shield.ico。该结果证明 POSTINSTALL 行为，首次安装完成页的 onGUIEnd 路径与取消选项仍待交互复测。

## Windows 配置保留异常与恢复

首次安装前备份数据库 schema 5，configuration 有 1 行，含 1 个代理；安装后数据库 schema 6，configuration 为 0 行。用户明确没有选择删除应用数据或重置配置，因此不能将该首次升级标记为配置保留通过。根因尚未确定，不能仅凭更新模式成功宣告修复。

用户退出应用并确认恢复后，保留首次安装后的数据库副本，再尝试恢复原库。直接复制数据库及 WAL/SHM 的恢复方式不可靠，恢复后的库完整性检查失败，应用初始化报数据库操作失败并退出。原始备份本身完整，所有证据副本保留。

改用 node:sqlite 的 backup 接口生成原备份的一致快照，校验完整性通过；保留不一致恢复库，恢复完整快照，不复用 WAL/SHM 文件。启动后 schema 已迁移为 6，configuration 与原备份逐记录一致（1 行），应用进程持续运行于控制台会话。用户最终确认“窗口正常，原配置已显示”。所有数据库与备份留在远端隔离验收目录，不提交用户数据。

## 修复后的质量检查

包含新 Windows 配置与 hook 的隔离交付副本 pnpm check 全部通过：159 个前端测试、19 个工具测试、format、lint、typecheck、Rust fmt/clippy 与 IPC 检查。真实 Windows NSIS 构建与更新模式安装成功。根工作区无关未提交文件保持不变。阶段 4 仍未完成，不发布新版本。

## Windows 中文卸载与保留配置复测

应用退出后，使用 SQLite backup 接口保存卸载前的一致快照，完整性检查通过，configuration 为 1 行。用户确认中文卸载页面和品牌图正常，并完成卸载，未勾选“删除应用数据”。卸载临时任务返回 0；安装目录下 socks-proxy.exe、brand-shield.ico 和 uninstall.exe 均移除，当前用户桌面及开始菜单快捷方式均移除。卸载后 configuration、selected_mode 与退出后的快照逐记录一致，数据库完整性检查通过；临时卸载任务已清理。旧 Inno 安装及其公共开始菜单入口未改动。

随后核验最新验收包 SHA256 为 fe7c305742e42c233cd44d786e6f02c4d36c517420bb9a72ede9c445cd01c921，通过交互临时任务启动重新安装。该包来自 290aec3，包含快捷方式图标 hook 与代理状态标签移除。正在由用户辅助验证取消桌面快捷方式选项、启动及原配置展示；尚不将重装结果标记通过。首次安装配置缺失的根因仍待调查。

重新查询确认安装临时任务处于 Running，LastTaskResult 为 267009（任务仍在运行），安装器进程 26104 位于控制台会话 1；未因等待用户操作而重复启动安装器。

针对配置缺失检查当前 store_migrations.rs：schema 5→6 仅新增 connection_count_samples 表，不删除或重建 configuration；store.rs 的配置写入采用 UPSERT，也未发现删除 configuration 的语句。这些静态证据无法解释首次安装后 0 行配置，不能据此排除安装器或数据路径问题，也不能作为升级保留配置通过的证据。

进一步读取远端实际构建的 target/release/nsis/x64/installer.nsi：安装身份 BUNDLEID 仍为 com.socksproxy.desktop；847–860 行仅在 DeleteAppDataCheckboxState=1 且 UpdateMode<>1 时递归删除 Roaming/Local 数据目录。754–756 行解析 /UPDATE，交互重装卸载分支 350–355 行调用已有卸载入口。当前脚本没有无条件删除数据目录的代码；用户明确未勾选删除数据，而首次安装所调用的旧卸载器及当时数据目录状态尚无充分证据，因此根因仍未确定，不作已修复断言。

后续重新查询，重装任务已结束并返回 0，应用进程 2604 在控制台会话 1 运行，路径为本次 NSIS 安装目录。桌面及开始菜单快捷方式均已创建，目标路径正确，IconLocation 均为本次安装目录的 brand-shield.ico,0；该结果证明交互安装完成页创建后的图标 hook 生效。由于桌面链接实际存在，需要用户确认是否选择了创建快捷方式，不能据此将“不创建桌面快捷方式”场景标记通过。

运行中的重装应用数据库 configuration、selected_mode 与卸载前关闭后的快照逐记录一致，configuration 为 1 行，完整性检查通过。此轮卸载后重装已证明数据库内容保留；窗口展示和标签移除仍待用户确认，首次升级异常仍未定因。已移除重装临时任务，未停止应用进程。

用户随后明确确认本轮勾选了创建桌面快捷方式，并确认窗口、原配置及状态标签移除均正常。因此已有桌面链接符合用户选择，不能视为取消选项失败。开始下一轮取消选项验收前，将桌面链接移动到远端验收备份目录（不覆盖既有备份），通过 SocksBrandingUncheckedDesktop 临时交互任务再次打开同一已核验安装包，请用户明确取消桌面快捷方式选项并检查托盘图标及菜单；该轮仍待完成。

该轮任务随后结束并返回 0：当前用户桌面链接不存在，开始菜单链接存在，目标及 brand-shield.ico 图标路径正确；应用进程 22280 位于控制台会话 1。再次核验 configuration、selected_mode 与卸载前关闭后的快照逐记录一致，configuration 为 1 行，数据库完整性通过。临时任务已移除；暂存桌面链接仍保留在备份目录，未重新创建桌面入口。尚待用户明确确认取消选项及托盘图标与菜单，以完成交互视觉验收。

## 用户最终视觉确认与 Linux 验收范围调整

2026-10-05 用户确认“mac 和 windows 视觉都正常，linux 不用”：macOS/Windows 视觉验收通过；本次取消 Linux 桌面菜单、系统图标及桌面交互的人工视觉验收，保留 Linux 构建、元数据、DEB 安装和 AppImage 启动验证。该决定不豁免配置保留要求，不代表首次 Windows 升级配置缺失已定因或修复。

结合前述安装、卸载、快捷方式勾选/取消两条路径的实际核验，以及用户本轮 macOS/Windows 视觉确认，4.1 和 4.2 标记完成。4.3 按修订范围由已有 Linux CI 的元数据、DEB 安装及 AppImage 启动证据完成；未宣称执行 Linux 桌面人工视觉验收。4.4、4.5 及发布任务仍待完成。

## Windows 品牌前版本对照复测（进行中）

用户确认从托盘退出并授权复测。通过 SQLite backup 保存测试前当前数据库 current-before-rehearsal.sqlite3，并保留原 DB/WAL/SHM 文件副本；旧版独立基线 schema 5、configuration 1 行且完整性通过。原始独立快照实际位于远端工作目录根部，定位正确文件后生成 legacy-baseline.sqlite3，未复用旧 WAL/SHM。

从成功 CI 37214843646 下载品牌改动前源码 b933e99c4291feda1da2020ad034ed90bb15df9a 的 Windows 安装包，核验源码和 SHA256 c92e3fa89a2742087a1c63c0cab34f1ae0252893f20e9961fc3336dfa14e05cf。旧版安装以 /S /UPDATE 返回 0；恢复独立旧基线后在控制台启动，随后任务正常退出返回 0。数据库完整，configuration 与原旧基线逐记录一致。该源码本身支持 schema 6，因此旧版已迁移数据库至 6；selected_mode 与原旧基线不同，启动后的模式表有 1 行，保存 legacy-after-start.sqlite3 作为后续升级比较基线，不宣称该变化已完成根因分析。

确认旧版进程不存在后，打开新版 290aec3 的交互安装包进行品牌前→品牌后覆盖复测；等待用户完成。两个包均为 0.2.1，此次检验安装器及品牌资源替换，不替代未来新版本号的升级验证。测试前当前数据快照仍完整保留，复测结束须恢复该快照并验证应用。

新版安装临时任务随后结束返回 0，应用进程 8784 在控制台会话 1 运行。读取运行中数据库，与 legacy-after-start.sqlite3 比较：configuration、selected_mode 均逐记录一致，configuration 1 行，schema 6，完整性检查通过。本轮品牌前→品牌后交互覆盖安装未复现配置缺失；不将未复现等同于首次异常已定因。尚待应用退出，保留复测后的数据库证据并恢复 current-before-rehearsal.sqlite3。

随后实际确认进程已退出，使用 SQLite backup 保存 after-branded-rehearsal.sqlite3，并从 current-before-rehearsal.sqlite3 生成完整性通过的 restore-ready.sqlite3。将复测后的 DB/WAL/SHM 移到独立证据目录，恢复独立快照，不复用 WAL/SHM。启动前检查 7 张表（configuration、configuration_commit、configuration_recovery、connection_count_samples、proxy_ownership、runtime_diagnostics、selected_mode）均与测试前快照逐记录一致，表集合一致，完整性通过。移除升级临时任务并重新启动新版；等待用户确认恢复后的窗口与配置，本轮不再要求退出。

恢复后再次查询，应用进程 23048 持续运行于控制台会话 1；configuration、selected_mode 与 current-before-rehearsal.sqlite3 一致，configuration 1 行，schema 6，完整性检查通过。已清理恢复启动临时任务，保留运行中的应用。用户窗口确认仍待回复。

复查 Linux 验证脚本：现有 CI 覆盖 DEB 安装、原生前端就绪、卸载后数据库文件哈希一致和 AppImage 独立启动，但尚未安装实际旧版本后再升级；4.4 的 Linux 配置升级证据仍缺。用户取消的是桌面视觉验收，未将此静态或单次安装结果记作旧版升级通过。

## 候选 CI 验收问题修正

旧源码 CI 37266088786 的 Windows 实核测试在 read_header 的 read_exact 返回 WouldBlock。夹具监听器使用非阻塞模式，读取函数现在显式切回阻塞读取并保留原 5 秒超时；新增非阻塞连接延迟分段响应测试。本机 Rust fmt/clippy 和回归测试通过，真实 Windows 设置 SING_BOX_TEST_BIN 后执行 real_core_tests，两项均通过，包含固定 sing-box 认证、凭据更新、失败回滚验证。该改动仅影响测试夹具。

0.2.2 候选升级 CI 37266909074 的 Linux 升级步骤失败在旧版读取测试配置：夹具将 diagnostic_retention 写成 Days30，而真实 Rust 契约要求 days30。修正为共享 JSON 夹具，Node 种入数据库与 Rust 模型测试读取同一文件；Rust 实际反序列化、validate、非空代理及默认代理断言通过。此次失败是人工生成的验收数据错误，未涉及用户数据库；修复后仍须重新执行真实 Linux 升级 CI，不以本机模型测试代替。

上述修复的准确暂存源码在隔离副本完整 pnpm check 通过（159 前端、19 工具测试及静态/Rust/IPC 检查），另行 cargo test --locked 通过：233 单元测试、10 集成测试、1 示例测试及 2 文档测试；保留正常的隔离子进程/手动测试忽略项。本机未设置 SING_BOX_TEST_BIN 的条件测试不作为真实内核证据，真实内核依据为前述远端 Windows 两项执行结果。
