# 原生代理与规则管理验收（部分）

2026-10-04，以最新 refactoring-v1 0.2.0 工作区构建 macOS debug bundle，应用标识 com.socksproxy.desktop.refactor-validation，全部界面操作经 CUA 执行，保留已有隔离验收数据。

| 场景             | 实际结果                                                                                                                                     |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| 添加代理无效端口 | 名称“原生验收临时代理”、127.0.0.1、端口 0；提示 profiles[1].port 必须在 1–65535，保留全部输入；错误编号 e9497b30-4460-45b8-a104-2fd26671250b |
| 修正后保存       | 端口改为 1088，列表新增 SOCKS5 代理                                                                                                          |
| 编辑代理         | 名称改为“原生验收临时代理已编辑”、端口 1089，列表显示修改后的值                                                                              |
| 代理启停         | 开关停用和恢复启用均同步到行状态                                                                                                             |
| 默认出口同步     | 默认操作选中临时代理；状态页和底栏同步名称、地址与端口；取消默认后恢复未选择                                                                 |
| 添加规则         | 新增 manual-check.invalid 域名直连规则；规则测试命中该规则，配置版本 12                                                                      |
| 编辑规则         | 名称改为“原生验收临时规则已编辑”、目标 manual-edit.invalid、端口 443；重测命中编辑后的规则，配置版本 15                                      |
| 规则启停和排序   | 停用/启用同步；上移后临时规则成为首行，原规则成为第二行；规则修改后旧测试结果清除                                                            |
| 取消规则删除     | 确认对话框取消后仍保留两条规则                                                                                                               |
| 清理临时数据     | 原生导出完整无密码恢复副本后，经确认对话框删除临时规则和临时代理；保留原有一代理一规则、默认出口 null；展开侧栏恢复原外观                    |

恢复副本 /tmp/architecture-manual-crud-recovery.json 包含本次两个临时资源，可经既有配置导入流程恢复；SHA-256：9861855b44796e13449d233f229f301d10f80bdc6a3a44f4c7daeb8814d0a611。

清理后原生导出 /tmp/architecture-manual-crud-after-cleanup.json，与先前 /tmp/socks-architecture-config-after-import.json 完整 JSON 对象相等，SHA-256 同为 cfd45470d47da00a21ced8fdbfa9935188e5d11d8d0884a7a5470952b1b2fa04。没有删除原有验收资源或操作常规应用配置。

macOS 生产后端为 UnavailableRuntimeBackend；模式按钮、国内直连、延迟测试及网络恢复按平台能力禁用。本次不证明成功启动或切换内核、Windows 系统代理、国内直连、运行中竞态或 Windows E2E。因此 1.7.3、3.6.4 和完整用户场景仍未勾选；本报告是已完成场景的具体证据。

## Windows 成功模式切换补充（2026-10-04）

用户明确允许官方 WebDriver 后，使用独立 architecture-e2e 应用执行 Windows 原生界面。windows-native-e2e-assisted-retry.json 记录实际添加 HTTP 代理、默认选择、规则保存/匹配、国内直连、规则→全局→直连；windows-native-runtime-partial.json 记录快速选择后的最终规则模式、元数据保存与显式重试。人工查看实际截图确认：页签、应用模式、健康声明和底栏一致；全局直连截图显示未运行、成功、全局直连、未接管系统代理，默认代理名称保留。原生截图位于对应远端报告目录的 case-1.png/case-2.png 和 runtime case-2.png/case-4.png；本地审查副本为 /tmp/architecture-native-sync-ui.png、/tmp/architecture-native-retry-ui.png、/tmp/architecture-native-direct-ui.png。

这补齐本报告此前缺少的 Windows 原生成功启动和模式切换观察；连同前述 CUA 管理操作，1.7.3 完成。不是将模拟测试当作手工验收，也不代表配置导入导出或长时稳定性完成。状态竞态项仍需完整运行采样及最终状态核对。
