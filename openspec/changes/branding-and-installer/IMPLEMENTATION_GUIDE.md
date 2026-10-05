# 品牌图标与中文安装实施指南

本指南依据 2026-10-05 用户决策更新：Windows 只发布中文 NSIS EXE；保留现有许可，新增中文第三方说明。执行范围以 proposal.md、DESIGN_SPEC.md、design.md、specs 和 tasks.md 为准。

## 准备工作

执行 pnpm 前核验 Node >=22.22.0、pnpm >=10.33.0，保留无关未提交文件。先读文档，再运行 OpenSpec status 和 apply instructions；缺少标准文档时补齐，不绕过 blocked 状态。

## 阶段 1：图标设计与生成

1. 在 src-tauri/icons 维护原创盾牌网络 SVG，按极简、简化、标准、完整四种级别分别生成资源，渐变、节点、留白按 DESIGN_SPEC.md。
2. 使用官方 `pnpm tauri icon` 生成平台资源；ICO/ICNS 内各尺寸须采用对应简化版本，不只缩放大图。
3. 提供 16–1024 PNG、彩色托盘和 macOS 黑色透明模板图标，集成 favicon。检查 RGBA、透明边缘、包内尺寸及每个文件小于 500KB。
4. 渲染浅色、深色及灰度预览，检查全部尺寸清晰度。集成后运行相关测试、typecheck、lint、Rust fmt/clippy；阶段完成后提交。

## 阶段 2：安装资源

1. 在 src-tauri/installer/windows 制作 NSIS 顶部图 150×57 和侧栏 164×314，生成 PNG 及 RGB BMP；使用原创品牌素材，留出系统文字区域，不写死版本。
2. 制作中文 RTF 第三方说明：保留 UNLICENSED 现状，注明 Windows sing-box 固定版本、GPLv3、源码及许可证位置；不添加授权、限制或未经核验的隐私承诺。
3. 在 macos 目录制作 660×400 和 1320×800 背景，应用 (150,150)、Applications (450,150)，箭头及 y=320 中文拖拽提示不遮挡系统图标及名称。
4. 实际渲染素材并检查尺寸、字体和布局，验证后提交。

## 阶段 3：配置与自动构建

1. 依据当前 Tauri 2 schema 配置 bundle.windows.nsis，语言 SimpChinese、品牌图标、顶部及侧栏、中文说明，保持安装身份及更新兼容。
2. 配置 DMG 背景及 660×400 窗口；提供 Apple Silicon、Intel 和保留 Universal 构建。
3. 提供 Linux 中文 Desktop Entry（Network;Settings; 分类及关键词）、AppStream 和 DEB/RPM 描述。使用真实项目 URL、准确许可与实际功能说明，不声明未实现的 URI handler。
4. 扩展 CI 至全平台，发布汇总明确验证全部安装包、SHA256、版本及源码；增加拒绝缺包及篡改的测试。执行 pnpm check 与相应构建，阶段完成后提交。

## 阶段 4：真实平台验收

Windows 验证所有中文安装和卸载页面、侧栏与顶部素材、中文说明、桌面快捷方式选项、开始菜单、完成后启动、托盘图标及菜单。

macOS 验证各架构 DMG 构建、背景和 Retina 显示、拖拽安装、Dock、Launchpad、应用信息及明暗主题菜单栏。

Linux 验证 AppImage 实际启动、DEB 安装和中文桌面元数据；按 2026-10-05 用户决定，本次无需人工验收桌面菜单、菜单启动及系统图标视觉。

所有平台验证旧版升级保留配置、卸载移除安装文件和快捷方式但保留配置。静态检查不替代运行验收；修复问题后重测并记录真实证据。完成质量检查后提交验收记录。

## 阶段 5：发布准备

更新 README 品牌图标与平台说明、CHANGELOG。统一命名：Windows `Socks-Proxy_v{version}_Windows_x64_安装包.exe`；macOS `Socks-Proxy_v{version}_macOS_Apple-Silicon.dmg`、`..._Intel.dmg`、`..._Universal.dmg`；Linux `Socks-Proxy_v{version}_Linux_x64.AppImage`、`socks-proxy_{version}_amd64.deb`，RPM 使用清晰的 Linux x64 标识。

核验完整 SHA256SUMS、VERSION、SOURCE_REVISION。确定新的未发布版本号并创建 GitHub Release，保持现有 v0.2.1 标签不变。实际核验下载后逐项审计规格及 tasks，全部完成才宣告实施完成。

## 阶段提交规范

每阶段独立提交，采用 `feat(scope): [阶段名称] 简要描述`，正文列出具体修改、实际通过的测试及验证项。不得提交 dist、依赖、本地工具配置或密钥。
