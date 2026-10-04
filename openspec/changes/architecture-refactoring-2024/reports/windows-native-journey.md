# Windows 原生完整用户流程验收

2026-10-04 在 Windows x64 交互 Session 3，通过官方 Tauri WebDriver 执行隔离 debug 应用。固定标识 com.socksproxy.desktop.architecture-e2e；二进制 SHA256 为 5cb02795c27e0a4a43486290fbf827ef3ff54056903248f61ca21e55cf63da4b。完整四项通过，原始结果见 windows-native-e2e-final.json；独立配置和清理复查见 windows-native-e2e-final-summary.json。

- 4.5.2：真实页面添加本地 HTTP 档案、设为默认、规则模式启动，界面会话健康。
- 4.5.3：新增域名直连规则、启用国内直连，实际路由测试核对域名规则及私有 IP；全局代理→全局直连状态符合预期。
- 4.5.4：用户完成第一次原生保存 before.json；页面临时改名后，通过真实文件输入和确认导入恢复原档案；用户完成第二次原生保存 after.json。拒绝旧目标文件，两次均为本轮新文件，逐层完整 JSON 相等且文件字节相等；各 831 字节、schema_version=2、递归无 password 字段。两次 SHA256 均为 6e99555f9e71118e0ba6c29dd4f7eb442aa65f1164aa6578e2d58d32d7979d4c。
- 4.5.5：从规则模式运行界面进入设置、确认恢复，实际终态未运行/未接管系统代理；最终清理再次经原生界面恢复。

人工查看 windows-native-configuration-roundtrip.png，代理列表和底栏恢复同一原档案名称，直连/内核未运行；windows-native-journey-recovery.png 保存最终停止/未接管状态。截图辅助核对界面，完整配置相等由实际新文件及套件断言证明。

PowerShell 外层结果 user_proxy_restored=true；完成后独立读取 HKCU 五字段与运行前逐项存在性、类型和值完全相等。再次读取进程清单和隔离 runtime 目录，测试应用/内核/驱动/Node 均已退出、运行目录为 0。临时交互计划任务在 Ready 后删除，复查 absent。原始代理/PAC 值不复制到仓库。

本轮目录 socks-native-e2e-9be528846b28453a874a070bb9d24cb0，结束时间 19:11:03。人工保存每次等待十五分钟；先前超时和会话失败证据保留。本地后续只修改 lint 抑制注释，执行源码摘要仍记录远端实际版本。

范围是合成本地 HTTP 上游和隔离应用，不能推导公网连通性。退出恢复、手动重试、竞态及 30 分钟 GUI 稳定性由独立完整 runtime 套件证明；不由四项 journey 推导。实际 CI 云端执行、发布和归档不在本轮完成声明中。
