# 品牌图标与安装界面设计规范

本文档定义应用图标、安装界面的详细设计规范和技术标准。

---

## 1. 应用图标设计

### 1.1 设计方案：盾牌 + 网络节点

#### 核心概念

```
视觉隐喻：
🛡️ 盾牌 = 安全保护、隐私守护
🔗 网络节点 = 代理转发、全球连接
🎨 蓝色 = 可靠、专业、技术
```

#### 设计元素分解

**主体结构**：

```
┌─────────────────────┐
│   ╱◜◝╲             │  ← 盾牌顶部圆弧
│  ╱    ╲            │
│ ╱  ●───●  ╲        │  ← 内部网络节点（3-4个）
│ │   │ │   │        │  ← 节点连线
│ ╲  ●───●  ╱        │
│  ╲    ╱            │
│   ╲  ╱             │
│    ╲╱              │  ← 盾牌底部尖角
└─────────────────────┘
```

**尺寸规范**：

- 盾牌宽高比：0.85:1（略瘦的盾形）
- 内边距：总尺寸的 10%（留白区域）
- 节点直径：总尺寸的 8-10%
- 连线粗细：总尺寸的 2-3%
- 圆角半径：总尺寸的 5%

**色彩规范**：

```css
/* 主渐变 - 从左上到右下 */
.icon-gradient-primary {
  background: linear-gradient(135deg, #4a90e2 0%, #357abd 100%);
}

/* OKLCH 对应值 */
--icon-start: oklch(0.62 0.18 250);
--icon-end: oklch(0.52 0.2 245);

/* 节点颜色 */
--node-fill: #ffffff; /* 白色填充 */
--node-stroke: rgba(255, 255, 255, 0.8); /* 半透明描边 */

/* 连线颜色 */
--line-color: rgba(255, 255, 255, 0.6); /* 半透明白线 */
--line-glow: rgba(255, 255, 255, 0.3); /* 光晕效果 */
```

**光影效果**：

```css
/* 盾牌阴影（扁平风格，微妙） */
.icon-shadow {
  box-shadow:
    0 2px 8px rgba(52, 122, 189, 0.15),
    0 4px 16px rgba(52, 122, 189, 0.1);
}

/* 节点内发光 */
.node-glow {
  filter: drop-shadow(0 0 4px rgba(255, 255, 255, 0.6));
}

/* 整体高光（可选） */
.icon-highlight {
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.15) 0%, rgba(255, 255, 255, 0) 50%);
}
```

---

### 1.2 多尺寸适配规则

#### 超大尺寸（512px - 1024px）

**细节丰富版本**：

- 显示完整的节点连线
- 添加细微的光影效果
- 盾牌边缘带高光渐变

```
适用场景：
- macOS Finder 图标
- Windows 资源管理器大图标
- 网站展示
```

#### 中等尺寸（128px - 256px）

**标准版本**：

- 保留主要结构
- 简化光影细节
- 节点清晰可见

```
适用场景：
- macOS Dock 图标
- Windows 任务栏
- Linux 桌面环境
```

#### 小尺寸（32px - 64px）

**简化版本**：

- 加粗盾牌轮廓
- 减少节点数量（2-3个）
- 去除细微光影

```
适用场景：
- Windows 系统托盘
- macOS 菜单栏
- 网页 Favicon
```

#### 超小尺寸（16px - 24px）

**极简版本**：

- 只保留盾牌轮廓
- 单个中心点（可选）
- 高对比度配色

```
适用场景：
- 浏览器标签页图标
- 通知图标
```

---

### 1.3 替代设计方案

#### 方案 B: 地球仪 + 轨道线

```
主体：简化的地球球体
轨道：2-3条流畅的椭圆轨道线
配色：蓝绿渐变（#00D4AA → #0099FF）
风格：3D 等距视角
```

**优点**：

- 直观传达"全球网络"概念
- 动感强，现代化
- 适合科技产品

**缺点**：

- 小尺寸下细节丢失
- 与"安全"主题关联较弱

---

#### 方案 C: 字母 S + 网络拓扑

```
主体：大写字母 "S"
细节：字母笔画由节点和线组成
配色：单色或蓝橙双色
风格：极简几何
```

**优点**：

- 品牌识别强
- 各尺寸表现稳定
- 易于记忆

**缺点**：

- 创意相对保守
- 需要精心设计才能不显单调

---

### 1.4 深色模式适配

**浅色背景（默认）**：

```css
/* 使用标准渐变 */
background: linear-gradient(135deg, #4a90e2 0%, #357abd 100%);
```

**深色背景**：

```css
/* 加亮渐变，提升对比度 */
background: linear-gradient(135deg, #5b9fff 0%, #4a8fe2 100%);

/* 或：添加边缘光晕 */
filter: drop-shadow(0 0 8px rgba(91, 159, 255, 0.4));
```

**macOS 菜单栏图标**（Template Image）：

```
使用单色版本：
- 纯黑图形 (#000000)
- 透明背景
- 系统自动反色适配
```

---

## 2. 安装界面设计

### 2.1 Windows 安装程序（NSIS）

用户于 2026-10-05 确认仅发布中文 NSIS EXE，替代原 WiX MSI 方案。

- `languages: ["SimpChinese"]`，欢迎、完成、进度、错误和卸载文字使用简体中文。
- 顶部图 150×57px，欢迎及完成页侧栏 164×314px，提供 PNG 源素材和 RGB BMP。
- 侧栏使用蓝色渐变、盾牌标识与中文“代理管理工具”；顶部图主要展示盾牌，避免遮挡安装器原有页面标题。
- 不在素材中写死版本号，版本来自构建配置。
- 安装器与卸载器使用品牌 ICO，提供桌面快捷方式选择、开始菜单入口及完成后启动应用选项。
- 中文 RTF 展示现有许可与第三方说明，不新增项目授权，不添加“禁止逆向工程”等与 GPLv3 权利冲突的限制；注明 sing-box 固定版本、对应源码、许可证位置并保留原文。
- 保持 NSIS 安装身份、应用标识及数据目录，升级保留配置，卸载保留用户数据。

---

### 2.2 macOS 安装程序（DMG）

#### DMG 背景图（660 x 400 px，Retina: 1320 x 800 px）

**设计布局**：

```
┌───────────────────────────────────────────┐
│                                           │
│                                           │
│    [App Icon]      ────→   [Applications]│
│      120x120       箭头        文件夹图标  │
│                                           │
│    将应用拖拽到 Applications 文件夹安装    │
│                                           │
│                                           │
└───────────────────────────────────────────┘
```

**配色方案**：

```css
/* 背景 */
--bg-color: #ffffff; /* 纯白 */

/* 或：渐变背景 */
--bg-gradient: radial-gradient(circle at center, #f8fafc 0%, #ebf4ff 100%);

/* 箭头 */
--arrow-color: #4a90e2;
--arrow-style: 实线或虚线箭头，3px 粗细 /* 文字提示 */ --text-color: #64748b;
--text-size: 14px;
--text-align: center;
```

**元素位置**（标准 660x400）：

```
应用图标位置：(150, 150)
Applications 图标位置：(450, 150)
箭头起点：(230, 150)
箭头终点：(365, 150)
文字位置：居中，y=320
```

---

#### DMG 窗口配置

**窗口属性**：

```json
{
  "window": {
    "size": {
      "width": 660,
      "height": 400
    },
    "position": {
      "x": 200,
      "y": 120
    }
  },
  "icon_size": 120,
  "background": "dmg-background.png"
}
```

---

### 2.3 Linux 安装程序

#### Desktop Entry（socks-proxy.desktop）

```ini
[Desktop Entry]
Version=1.0
Type=Application
Name=Socks Proxy
GenericName=代理管理工具
GenericName[en]=Proxy Manager
Comment=专业的 SOCKS 代理桌面管理应用
Comment[en]=Professional SOCKS proxy desktop manager
Exec=socks-proxy %U
Icon=socks-proxy
Terminal=false
Categories=Network;Settings;
Keywords=proxy;socks;network;代理;网络;
StartupWMClass=socks-proxy
```

---

#### AppImage 元数据

**AppImage.appdata.xml**：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>com.socksproxy.desktop</id>
  <name>Socks Proxy</name>
  <name xml:lang="zh_CN">Socks Proxy 代理管理工具</name>

  <summary>专业的 SOCKS 代理管理应用</summary>
  <summary xml:lang="en">Professional SOCKS proxy manager</summary>

  <description>
    <p>
      Socks Proxy 是一款功能强大的 SOCKS 代理管理桌面应用，
      支持多种代理协议、智能路由规则和系统代理管理。
    </p>
    <p xml:lang="en">
      Socks Proxy is a powerful SOCKS proxy management desktop
      application with support for multiple protocols, smart
      routing rules, and system proxy management.
    </p>
  </description>

  <launchable type="desktop-id">socks-proxy.desktop</launchable>

  <url type="homepage">https://github.com/liqiyuan-152/socks-proxy-desktop</url>
  <url type="bugtracker">https://github.com/liqiyuan-152/socks-proxy-desktop/issues</url>

  <metadata_license>CC0-1.0</metadata_license>
  <project_license>LicenseRef-Proprietary</project_license>
</component>
```

---

## 3. 技术规格

### 3.1 图标文件清单

```
icons/
├── icon.png                 # 源文件 1024x1024
├── icon.icns                # macOS 图标包（自动生成）
├── icon.ico                 # Windows 图标包（自动生成）
├── 16x16.png               # 超小尺寸
├── 32x32.png               # 小尺寸
├── 32x32@2x.png            # Retina 小尺寸
├── 64x64.png               # 中小尺寸
├── 128x128.png             # 中等尺寸
├── 128x128@2x.png          # Retina 中等尺寸
├── 256x256.png             # 大尺寸
├── 512x512.png             # 超大尺寸
├── 512x512@2x.png          # Retina 超大尺寸
└── 1024x1024.png           # 最大尺寸
```

---

### 3.2 安装资源清单

实际路径使用 `src-tauri/installer/`，资源明细见 ASSETS_CHECKLIST.md。

- windows：NSIS 顶部图、侧栏 PNG/BMP、中文 RTF 第三方说明。
- macos：660×400 及 1320×800 DMG 背景。
- linux：中文 Desktop Entry 和 AppStream XML。

### 3.3 Tauri 配置

以已安装的 Tauri 2 `config.schema.json` 为准。应用标识及 productName 位于配置顶层；中文说明使用 bundle.licenseFile；Windows 使用 bundle.windows.nsis；DMG 使用 bundle.macOS.dmg；DEB/RPM 使用 bundle.linux 下的配置。禁止复制旧版字段或含占位符的模板。架构和发布矩阵参见 design.md。

---

## 4. 设计工具和流程

### 4.1 推荐设计工具

**图标设计**：

- Figma（在线，免费）
- Adobe Illustrator（专业）
- Sketch（macOS）
- Affinity Designer（一次购买）

**图标生成**：

```bash
# Tauri 官方工具
pnpm tauri icon icons/icon.png
```

**位图编辑**：

- Photoshop
- GIMP（免费）
- Affinity Photo

---

### 4.2 AI 辅助设计提示词

**Midjourney / DALL-E 提示词**：

```
A modern app icon for a SOCKS proxy manager desktop application.

Design elements:
- Main shape: Smooth shield outline (rounded corners)
- Inside: 3-4 interconnected circular nodes with lines
- Color: Blue gradient from #4A90E2 to #357ABD
- Style: Flat design with subtle gradient and shadows
- Background: Transparent or white
- Size: 1024x1024 pixels, clean vector look
- Professional, tech-focused, modern aesthetic

The icon should be recognizable at small sizes (16px-1024px).
No text, no complex details, clean and minimal.
```

**Stable Diffusion 提示词**：

```
app icon, shield shape, network nodes, blue gradient,
flat design, modern, professional, tech style,
vector art, clean lines, 1024x1024,
high quality, trending on dribbble
```

---

### 4.3 设计验证清单

**视觉质量**：

- [x] 在 1024px 下细节丰富
- [x] 在 512px 下主体清晰
- [x] 在 128px 下识别度高
- [x] 在 32px 下轮廓清晰
- [x] 在 16px 下仍可辨认

**色彩测试**：

- [x] 浅色背景下对比度足够
- [x] 深色背景下对比度足够
- [x] 与品牌色系一致
- [x] 在灰度模式下可辨认

**平台兼容**：

- [x] macOS Dock 显示正常
- [x] Windows 任务栏显示正常
- [x] Linux 元数据与包验证通过；按用户决定不进行桌面视觉验收
- [x] 浏览器 Favicon 显示正常

**文件格式**：

- [x] PNG 文件透明背景正确
- [x] ICNS 文件包含所有尺寸
- [x] ICO 文件包含所有尺寸
- [x] 文件大小合理（< 500KB）

---

## 5. 品牌使用规范

### 5.1 图标使用规范

**允许的使用**：
✅ 官方文档和教程  
✅ 应用商店展示  
✅ 社交媒体宣传  
✅ 博客文章和评测

**禁止的使用**：
❌ 修改图标颜色或形状  
❌ 拉伸或变形图标  
❌ 添加额外的元素  
❌ 用于误导性的宣传

**最小尺寸**：

- 数字媒体：16x16 px
- 印刷品：10mm x 10mm

**安全空间**：

- 四周预留图标高度的 10% 作为留白

---

### 5.2 命名规范

**应用名称标准写法**：

```
正确：Socks Proxy
正确：Socks Proxy 代理管理工具
错误：socks proxy（全小写）
错误：SOCKS PROXY（全大写）
错误：SocksProxy（无空格）
```

**安装包命名规范**：

```
格式：Socks-Proxy_v{版本}_{平台}_{架构}_{可选描述}.{扩展名}

示例：
Socks-Proxy_v0.2.0_Windows_x64_setup.exe
Socks-Proxy_v0.2.0_macOS_Apple-Silicon.dmg
Socks-Proxy_v0.2.0_macOS_Intel.dmg
Socks-Proxy_v0.2.0_Linux_x64.AppImage
socks-proxy_0.2.0_amd64.deb
```

---

## 6. 参考资源

### 6.1 设计灵感

**类似应用图标**：

- Clash for Windows（盾牌设计）
- ShadowsocksX（纸飞机）
- Surge（波浪）
- ProtonVPN（P字母+盾牌）

**设计网站**：

- Dribbble: https://dribbble.com/search/app-icon
- Behance: https://behance.net
- Icon8: https://icons8.com

---

### 6.2 技术文档

**Tauri 官方文档**：

- Icons: https://v2.tauri.app/develop/icons/
- Bundler: https://v2.tauri.app/distribute/

**平台设计指南**：

- macOS HIG: https://developer.apple.com/design/human-interface-guidelines/app-icons
- Windows Fluent: https://learn.microsoft.com/en-us/windows/apps/design/style/iconography
- Material Design: https://m3.material.io/styles/icons

---

**文档版本**: v1.0  
**最后更新**: 2024-10-05

## 验收范围更新

2026-10-05 用户确认“mac 和 windows 视觉都正常，linux 不用”：macOS/Windows 视觉验收通过；本次取消 Linux 桌面菜单、系统图标及桌面交互的人工视觉验收，保留 Linux 构建、元数据、DEB 安装和 AppImage 启动验证。该决定不豁免配置保留要求，不代表首次 Windows 升级配置缺失已定因或修复。
