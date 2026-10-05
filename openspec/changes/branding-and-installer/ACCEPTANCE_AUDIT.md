# 逐项验收审计

日期：2026-10-05。发布源码：b7e5dcfed0e8bfa6128dd9e0ce13dfb9cfe7fb98（v0.2.2）。五目标最终发布构建及独立 Quality 全部成功，用户确认英文命名后已修正附件与清单并重新公开；全部公开下载核验通过。

## 实现与平台证据

| 要求                                                         | 证据                                                                                                                                                                  | 当前结论                       |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------ |
| 原创统一盾牌、适配 16–1024px、透明边缘、各文件小于 500KB     | icons/source、generate-brand-icons.mjs、Phase 1；本轮 Pillow 检查 29 个图标文件大小及 PNG RGBA/透明角通过                                                             | 完成                           |
| ICO/ICNS 内部尺寸及浅色、深色、灰度清晰度                    | Phase 1 的官方 CLI 生成、iconutil/Pillow 解包及预览记录                                                                                                               | 完成                           |
| macOS 模板托盘、其他平台品牌图标、保留菜单行为               | tray.rs 模板分支、托盘状态测试；用户 macOS/Windows 最终视觉确认                                                                                                       | 完成                           |
| 浏览器 favicon                                               | Phase 4 原生 Chrome 普通标签截图确认及 ICO 小尺寸提取                                                                                                                 | 完成                           |
| Windows 全中文 NSIS、150×57 顶部及 164×314 侧栏              | Windows 配置、Phase 2、本轮尺寸复核、真实 makensis、用户安装/卸载页面确认                                                                                             | 完成                           |
| 保留现有许可、新增中文第三方说明且保留上游原文及源码         | third-party-zh-CN.txt/rtf、资源映射、Phase 2 textutil 回读及固定内核清单、用户可读性确认                                                                              | 完成                           |
| 桌面快捷方式勾选与取消、开始菜单、安装后启动                 | Phase 4 两条实际路径；最新重装两条链接指向安装程序和 brand-shield.ico,0；0.2.2 进程运行                                                                               | 完成                           |
| macOS 标准/Retina 中文 DMG、指定位置、拖拽替换及系统图标     | Phase 2 尺寸与布局；Phase 4 Finder 拖拽、Info.plist、用户视觉确认；三架构 CI 实际挂载布局检查                                                                         | 完成                           |
| Linux 中文桌面及 AppStream、DEB/RPM 描述、正确图标与启动参数 | Linux 元数据及 CI desktop-file-validate/AppStream、DEB 安装与 AppImage 原生就绪                                                                                       | 完成，用户取消桌面人工视觉验收 |
| 各平台旧版升级保留非空配置，卸载移除文件/链接但保留数据      | Phase 4：Windows 0.2.1→0.2.2、独立卸载及重装逐记录一致；macOS 临时非空档案升级及原数据恢复；Linux 真实旧 DEB 升级运行 37269117857                                     | 完成                           |
| Windows 旧卸载器 /UPDATE、新版无删除数据选项或递归删除分支   | prepare-windows-installer.mjs 固定模板哈希及 CLI 门禁、3 项工具测试、实际编译脚本检查；最新真实升级/卸载页面/重装证据                                                 | 完成                           |
| 跨模块质量、Rust 测试、真实 Windows 内核门禁                 | 应用源码最新 Quality 37281781189 和五目标 37281781145；本轮准确暂存副本 pnpm check（159 前端、22 工具）通过；最终 tag Quality 37290883574 与打包 37290883587 全部成功 | 完成                           |
| 分阶段提交、保留无关文件                                     | Git 历史 Phase 1–5；本轮 Phase 4 3d78c4c、Phase 5 6474763；无关窗口尺寸、AGENTS 和本地文档未提交                                                                      | 完成                           |

## 发布证据

- v0.2.2 标签已推送，首轮 37289501895 失败未发布；夹具修复已在 Windows 真内核通过，Desktop packages 37290883587 全部成功，Quality 37290883574 全部成功。
- 用户确认仅英文名称；实际 Windows 附件为 Socks-Proxy_v0.2.2_Windows_x64_setup.exe，label 为空，安装向导仍中文。
- 公开后重新下载全部十个附件至 /tmp/socks-release-022-public-verified，七包精确名称、SHA256、GitHub digest、大小、VERSION、SOURCE_REVISION 全部一致；发布说明与版本文档一致，Release 为非草稿 prerelease。
- v0.2.1 标签仍为 990504a7065bf43d9e00cc27a7579a531651158b；v0.2.2 为实际构建提交 b7e5dcfed0e8bfa6128dd9e0ce13dfb9cfe7fb98，公开后保持不变。文件名维护提交不改写二进制 SOURCE_REVISION。
- 发布汇总脚本 9 项测试及 ESLint 通过；OpenSpec 严格校验通过。Phase 5 最终提交 4b738b6 已完成；22 项任务均有对应证据，随后提交任务关闭记录。
- 发布地址：https://github.com/liqiyuan-152/socks-proxy-desktop/releases/tag/v0.2.2 。

## 证据边界

首次数据丢失已有新建库及 NTFS 数据目录删除时序证据，但缺少当时进程审计，具体触发操作未知。新版已消除安装器删除路径并通过实际复测，不推断用户勾选了删除。macOS/Linux 不支持代理内核，安装包未签名/未公证，没有自动更新。Linux 人工视觉验收由用户明确取消，未以 Xvfb 替代该视觉证明。所有用户数据库及快照仅保留在隔离验收目录，不进入仓库。
