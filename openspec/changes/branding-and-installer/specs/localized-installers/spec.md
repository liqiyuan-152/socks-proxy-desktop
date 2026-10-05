## Purpose

提供统一品牌和中文引导的跨平台安装体验，使用户能够正确选择安装包、完成安装和启动应用，并在升级和卸载过程中保留已有配置。发布产物须可追溯到对应版本和源码提交。

## ADDED Requirements

### Requirement: Windows 中文品牌安装向导

Windows NSIS EXE 安装包 SHALL 使用简体中文向导、品牌安装图标、164×314 欢迎及完成页侧栏和 150×57 顶部图。系统文字区域 MUST 保持可读，不得被素材遮挡。许可及第三方组件说明 SHALL 提供可读的中文 RTF，准确反映实际许可，保留第三方组件所授予的权利。

#### Scenario: 安装向导显示

- **WHEN** 用户打开 Windows NSIS EXE 安装包并查看欢迎、许可、安装进度和完成页面
- **THEN** 向导显示中文文字及对应品牌素材，说明内容清晰可读

### Requirement: Windows 安装后操作

Windows 安装向导 SHALL 提供桌面快捷方式选项和完成后启动应用的选项，创建可用的开始菜单入口，并提供正常工作的卸载入口。

#### Scenario: 快捷方式和启动

- **WHEN** 用户选择创建桌面快捷方式和安装后启动应用，并完成安装
- **THEN** 应用正常启动，桌面与开始菜单入口指向已安装程序并显示品牌图标

#### Scenario: 不创建桌面快捷方式

- **WHEN** 用户取消桌面快捷方式选项并完成安装
- **THEN** 安装器不创建桌面快捷方式，开始菜单入口仍可启动应用

### Requirement: macOS 中文拖拽引导

macOS DMG SHALL 提供 660×400 品牌背景及对应 Retina 资源，应用和 Applications 文件夹分别位于 (150,150) 和 (450,150)，显示方向箭头及“将应用拖拽到 Applications 文件夹安装”的中文提示。应用图标、文件夹、名称和引导 MUST 可读且不互相遮挡。

#### Scenario: 拖拽安装

- **WHEN** 用户挂载对应架构的 DMG 并按提示拖拽应用到 Applications
- **THEN** 应用安装成功，可以启动，Dock、Launchpad 和应用信息显示正确的品牌及版本

### Requirement: Linux 中文桌面集成

Linux 包 SHALL 提供中文应用名称、通用名称、描述、Network 和 Settings 分类及代理相关关键词，并正确安装品牌图标。Desktop Entry MUST 使用实际支持的启动方式，不得声明未实现的 URI 协议处理能力。AppImage 元数据和 DEB/RPM 描述 SHALL 使用准确的中文产品说明、项目地址及许可信息。

#### Scenario: Linux 安装与启动

- **WHEN** 用户安装 DEB 或运行 AppImage，并从桌面环境查看应用入口
- **THEN** 应用显示中文信息及品牌图标，应用菜单入口和 AppImage 均可正常启动

### Requirement: 升级和卸载保留配置

各平台安装包 MUST 保持应用身份和用户数据路径兼容。安装升级 SHALL 保留既有配置；卸载 SHALL 移除安装文件和对应快捷方式，同时保留用户配置。

#### Scenario: 已有配置升级

- **WHEN** 用户在已有版本和配置的环境中安装新版本
- **THEN** 新版本可读取原有配置，不生成重复的应用身份或丢失用户设置

#### Scenario: 卸载后重新安装

- **WHEN** 用户卸载后重新安装应用
- **THEN** 安装器已移除旧安装文件及快捷方式，用户配置仍可恢复使用

### Requirement: 可追溯的跨平台发布

发布 SHALL 提供 Windows x64 安装包、macOS Apple Silicon 和 Intel DMG、Linux x64 AppImage 及 DEB，使用提案规定的清晰平台和架构命名。发布 MUST 提供覆盖全部安装包的 SHA256 校验清单、版本号和源码提交号，并更新 README、CHANGELOG 和 GitHub Release。已发布版本标签 MUST 保持不变。

#### Scenario: 下载并核验安装包

- **WHEN** 用户按平台及架构下载发布安装包并核验 SHA256
- **THEN** 文件名准确表达版本和目标平台，校验值匹配，源码提交号对应发布源码

## 验收范围更新

2026-10-05 用户确认“mac 和 windows 视觉都正常，linux 不用”：macOS/Windows 视觉验收通过；本次取消 Linux 桌面菜单、系统图标及桌面交互的人工视觉验收，保留 Linux 构建、元数据、DEB 安装和 AppImage 启动验证。该决定不豁免配置保留要求，不代表首次 Windows 升级配置缺失已定因或修复。
