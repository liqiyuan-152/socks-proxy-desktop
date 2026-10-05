# Socks Proxy

![Socks Proxy 品牌图标](src-tauri/icons/128x128.png)

[![Quality](https://github.com/liqiyuan-152/socks-proxy-desktop/actions/workflows/quality.yml/badge.svg)](https://github.com/liqiyuan-152/socks-proxy-desktop/actions/workflows/quality.yml)

用于管理 SOCKS5 / HTTP 代理的桌面应用，基于 Tauri 2、React、TypeScript 和 Rust，使用固定版本 sing-box 作为代理内核。

## 平台与功能

实际代理运行目前支持 **Windows x64**。macOS / Linux 可用于界面开发与部分配置管理，但不支持代理内核、系统代理接管、国内直连预设及开机启动；浏览器开发模式也不提供原生后端。

- 代理档案：认证、默认代理、代理链接导入、单个及批量延迟测试。
- 模式切换：规则代理、全局代理、全局直连，支持系统托盘操作。
- 分流：域名、域名后缀、IP CIDR、端口范围、指定代理出口及国内直连预设。
- 数据：SQLite 配置持久化、配置升级、导入导出；认证凭据保存在系统凭据存储，导出不包含密码。
- 观测：活跃连接、运行时长、运行时诊断；窗口可见时每秒刷新，重新显示时立即刷新。
- 恢复：正常退出、内核异常退出、重启及手动恢复时处理本应用拥有的系统代理设置；无法确认归属时保留现场并提示检查。

当前未启用 TUN，仅覆盖遵循 Windows 系统代理设置的应用流量。“全局代理”也受此覆盖范围限制。连接历史、已完成连接结果、成功率和失败详情暂不可用。

## 环境准备

- Node.js >=22.22.0（推荐版本见 `.node-version`）。
- pnpm >=10.33.0（推荐版本见 `package.json`）。
- Rust stable；Windows 使用 MSVC 工具链及 C++ 构建工具。
- Windows WebView2 Runtime。

```powershell
pnpm install --frozen-lockfile
pnpm tauri dev
```

Windows 开发及构建会自动下载并校验固定的 sing-box 1.14.1 内核。国内直连规则集已随仓库提供，可离线加载；首次准备内核需要网络。来源与校验记录见 `scripts/sing-box-release-manifest.md` 和 `scripts/china-rules-manifest.md`。

仅开发界面时，运行 `pnpm dev` 并打开 http://127.0.0.1:5173/ 。

## 基本使用

1. 在“代理”页面添加 SOCKS5 或 HTTP 档案，按需填写认证信息；全局代理或国内直连预设需要设置默认代理。
2. 在“规则”页面添加分流规则并选择出口。用户规则按顺序首条匹配；预设关闭时未命中直连。
3. 如启用“国内直连”，用户规则优先，然后中国域名集和字面私有/中国 IP 直连，其余走默认代理。域名集外的域名不会因 DNS 解析到中国 IP 而改走直连。
4. 使用路由测试查看规则决策，再到“状态”页面切换模式。路由测试不代表实际联网成功。
5. 关闭主窗口会隐藏到托盘；从托盘“退出”结束应用并恢复本应用接管的系统代理。恢复有问题时使用“设置”中的网络恢复入口并按提示检查。

## 质量检查

```powershell
pnpm check
cargo test --manifest-path src-tauri/Cargo.toml --locked
pnpm build
pnpm tauri:build:check
```

`pnpm check` 包含格式检查、Oxlint、ESLint、Stylelint、TypeScript、Vitest、Rust 格式和 Clippy。Rust 测试需单独运行。GitHub Quality 工作流会在 Windows 执行上述检查并准备固定内核，以运行内核集成测试。

本地 Windows 运行真实内核测试：

```powershell
pnpm prepare:core
$env:SING_BOX_TEST_BIN = (Resolve-Path "src-tauri/resources/sing-box/windows-amd64/sing-box.exe").Path
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

未设置 `SING_BOX_TEST_BIN` 时，依赖内核的条件测试会直接返回；测试数量通过不等于真实内核已验证。Windows 系统代理和托盘行为还需真实桌面验收，不能仅凭 CI 判定。

错误处理、诊断查看与导出，以及数据库回退约束见 [开发指南](docs/error-handling.md)。

## 内测打包与发布状态

Windows 本地生成 NSIS 安装包：

```powershell
pnpm tauri build --bundles nsis
```

GitHub Actions 的 [Desktop packages](https://github.com/liqiyuan-152/socks-proxy-desktop/actions/workflows/package.yml) 工作流会在 `master` 更新、推送 `v*` tag 时自动构建，也支持手动运行。五个构建目标独立完成质量检查及 Rust 测试，Windows 还执行固定内核测试。产物包括 Windows x64 中文 NSIS EXE、macOS Apple Silicon / Intel / Universal DMG，以及 Linux x64 AppImage / DEB / RPM。macOS 和 Linux 的平台功能限制与上文一致；Linux CI 会检查元数据、安装及应用启动。

在对应 Actions 运行的 Artifacts 区域下载产物；每个压缩包包含安装包、`SHA256SUMS.txt`、`SOURCE_REVISION.txt` 和 `VERSION.txt`，保留 30 天。tag 必须与应用版本一致，例如 `v0.2.1`。推送 tag 后，全部构建目标成功才会自动创建对应的 GitHub 预发布 Release，上传安装包及校验清单。上传前验证全部七个安装包的 SHA-256、版本号和源码提交一致，并拒绝缺包、重复格式或未核验文件；先创建草稿，全部附件上传成功后才公开。失败时不会公开不完整的新版本，重跑可补齐附件。主分支构建仅保存 Actions 产物。安装包仍是未签名、未公证的内测产物。

当前使用 GitHub 预发布 Release 分发内测安装包，尚未配置应用签名和自动更新；“检查更新”不可用。正式发布前需提供签名凭据、确定发布与更新渠道，并完成真实 Windows 桌面验收，以及随包 sing-box 的 GPLv3 对应源代码提供方式、许可证通知和最终安装包内容复核。具体内核分发要求见 `scripts/sing-box-release-manifest.md`。

测试框架与覆盖率命令见 [测试指南](docs/testing.md)。

服务分工见 [架构指南](docs/architecture.md)，贡献者改动路径见 [迁移指南](docs/architecture-migration.md)，日志与性能工具见 [观测指南](docs/observability.md)。

## 品牌与安装素材维护

`pnpm icon:generate` 使用项目安装的官方 Tauri CLI，从 `src-tauri/icons/source` 的原创 SVG 重建 PNG，并按实际尺寸组装 ICO/ICNS；小尺寸使用简化版本。macOS 菜单栏使用单色模板图标，前端 favicon 使用同一品牌。

安装图片由 `scripts/generate-installer-assets.py` 生成，需要 Pillow 及支持中文的系统字体：

```sh
python3 scripts/generate-installer-assets.py --font /path/to/chinese-font.ttf
```

Windows 安装向导使用简体中文，配有品牌侧栏与顶部图；中文第三方说明保留项目现有许可状态，不替代 GPLv3 原文。macOS DMG 使用中文拖拽提示及 Retina 背景。阶段验证记录与待完成的实际平台验收见 `openspec/changes/branding-and-installer/`。
