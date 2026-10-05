# Phase 2 验证记录

日期：2026-10-05。

## 交付与重建

`src-tauri/installer/windows` 提供 150×57 顶部图及 164×314 侧栏的 PNG/RGB BMP，使用原创品牌图标，不写死版本号；macos 提供 660×400 和 1320×800 中文拖拽背景。实际应用及 Applications 图标由系统绘制，素材不重复绘制。

重建：安装 Pillow 后运行 `python3 scripts/generate-installer-assets.py --font <中文字体文件>`。本次使用系统 Arial Unicode 字体渲染，不随仓库分发字体。图片尺寸及 RGB 格式已用 Pillow 检查，中文素材已查看合成预览。DMG 箭头位于图标之间，文字位于 y=320。

中文第三方说明同时提供 UTF-8 TXT 与 ASCII Unicode 转义 RTF。macOS textutil 将 RTF 转回文本后与 TXT 原文完全一致。上游版本、GPLv3、源码提交和链接与 scripts/sing-box-release-manifest.md 一致；许可证安装路径与 tauri.windows.conf.json 资源映射一致。项目保留 UNLICENSED，说明不新增授权、不限制 GPLv3 权利、不替代许可证原文。

## 验证边界

本阶段交付静态素材，未宣称实际安装向导、DMG 或系统文字布局通过；这些将在 Phase 3 构建及 Phase 4 实际安装时检查。中文说明保留本项目许可现状，不是全部依赖许可证的完整汇编。
