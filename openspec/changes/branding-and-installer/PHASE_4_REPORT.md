# Phase 4 实际平台验收记录（进行中）

日期：2026-10-05。未完成全部系统视觉及安装交互验收，不宣告阶段完成。

## macOS 已执行

环境 macOS 26.6.2，Apple Silicon。官方 DMG 已挂载，Finder 图标视图中品牌图标、Applications 替身、中文拖拽提示及 Retina 背景可读，图标位置正确，箭头不与系统图标或标签重叠。箭头实际采用 (230,150)→(365,150)，与设计规范同步，避免覆盖 Applications 图标。

通过 Finder 复制安装，将已有 0.1.0 应用更新为 0.2.1 验收包；旧应用另行备份。更新后应用启动正常。原生 About 窗口显示品牌图标及 Version 0.2.1 (0.2.1)。用户 SQLite 的 configuration 和 selected_mode 表与启动前备份逐记录一致，未输出配置内容。

退出应用后通过 Finder 移入废纸篓，确认 Applications 中的程序移除、用户数据库仍存在且上述两张表记录一致。随后从挂载 DMG 重新复制到 Applications 并启动，记录仍保持一致。未清空废纸篓。

## 尚缺证据

- 拖拽自动化调用未得到安装结果；复制安装成功不能记作拖拽安装通过。
- Dock、SystemUIServer 和 macOS 26 的 App 系统应用列表无法由当前 CUA 获取，调用超时。Dock、系统应用列表及菜单栏明暗主题视觉仍待验证；本机系统已使用 App 替代旧 Launchpad。
- Windows 官方中文 EXE 构建成功，生成模板使用 SimpChinese、品牌素材、可选桌面快捷方式和启动选项；安装与卸载页面、升级及配置保留的实际交互仍待 Windows 桌面连接。
- 当前本机只有 Apple Remote Desktop（面向 Mac 的 ARD，不是 Windows RDP 客户端）。安装 Microsoft Windows App 的授权已通过异步问题请求，尚未收到答复；未擅自安装。
- Linux 重跑 CI 已成功：元数据、DEB 安装、原生前端就绪、卸载保留配置和 AppImage 独立启动通过。桌面菜单及系统图标视觉、旧版升级和 RPM 安装仍未验证。
- 浏览器生产预览可打开，favicon 声明及 HTTP 返回内容与品牌 ICO 一致；当前标签页截图无法明确辨认图标，视觉验收仍待完成。

## 验证边界

模板检查、资源预览、代码及构建成功不能替代实际界面交互。任务清单保留尚未证明的项目，不以静态检查标记全平台验收完成。
