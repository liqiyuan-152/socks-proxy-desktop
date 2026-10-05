# Phase 3 构建与质量检查记录

日期：2026-10-05。阶段仍在进行：Linux 构建及完整安装验收尚待 CI 结果。

## 已实现

Tauri 2 Windows 覆盖配置仅发布 NSIS，使用 SimpChinese、品牌图标、顶部/侧栏及中文 RTF。macOS 使用 Retina 背景与 660×400 窗口，应用及文件夹位置为 (150,150)、(450,150)。Linux 提供中文 Desktop Entry、AppStream 元数据、DEB/RPM 与 AppImage 配置，不注册未支持的 URI handler。

发布矩阵扩展为 Windows x64、macOS Apple Silicon/Intel/Universal、Linux x64，共七种安装包。汇总脚本核验数量、格式、版本、源码提交及 SHA256，拒绝未知平台、重复格式、未核验文件、路径穿越和已有输出文件。Linux CI 安装依赖并验证元数据、DEB 安装与卸载、AppImage/DEB 的真实 WebView 就绪确认；此自动检查不代表应用菜单视觉验收已完成。

## 构建证据

- 本机 Apple Silicon、Intel、Universal DMG 均由官方 Tauri CLI 构建成功，当前验收版本为 0.2.1，不覆盖发布标签。
- Windows 10.0.26200.9550 的隔离目录使用 Node 22.22.2、pnpm 10.33.0、已有 stable Cargo 1.99.0，官方 makensis 成功生成 Socks Proxy_0.2.1_x64-setup.exe。
- Windows 原默认 Cargo 1.78 不支持部分锁定依赖的 edition2024，采用进程级 RUSTUP_TOOLCHAIN=stable，不更改全局工具链。
- 为避免 PowerShell pnpm 包装器吞掉参数分隔符，CI 构建直接调用官方 tauri.js；beforeBuildCommand 保持原有 pnpm 工作流。
- macOS 顶层 category 使用 Tauri 支持的 Utility；Linux 桌面分类明确配置 Network;Settings;。

## 质量证据

完整 pnpm check 已在只包含交付源码的临时副本中通过，159 个前端测试、19 个工具测试、格式、lint、typecheck、Rust fmt/clippy 及 IPC 检查均成功。副本使用原依赖和目标缓存，并复制本机未提交的 registry 环境配置修复已失效的全局 USTC 源；该环境配置不提交。工作区的三份无关未提交文档保持不变。新增 Linux 验证脚本通过 bash -n，后续实际运行结果另行记录。

## 后续

等待 Linux CI 构建、检查产物后完成 3.3/3.5。Windows 安装向导的实际中文和快捷方式交互、macOS 拖拽更新及系统图标、Linux 桌面菜单视觉、各平台配置保留仍属于 Phase 4，不能以构建成功替代。

## 中文说明页补充

官方 NSIS 模板支持 installerHooks 在页面声明前加载附加定义。新增 notice-page.nsh，仅修改许可说明页的按钮与提示为“下一步”和第三方说明，避免把现有许可说明误呈现为新增 EULA。Windows 官方 makensis 已重新构建成功；生成的 installer.nsi 在 MUI_PAGE_LICENSE 前加载该文件。完整 UI 显示仍待 Phase 4。
