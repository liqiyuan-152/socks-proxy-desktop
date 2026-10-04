# 原生诊断导出验收

2026-10-04 在最新 debug app bundle（com.socksproxy.desktop.refactor-validation）通过 CUA 操作。

- 设置页搜索 save_profile，将 17 条记录筛选为 1 条；同类聚合与列表一致。
- 点击诊断导出，显示 macOS 系统 Save sheet。点击 Cancel 返回应用，未显示成功提示或错误。
- 再次导出，选择 /tmp，保存 socks-architecture-diagnostics-check.jsonl；页面显示“已保存当前筛选的全部诊断记录”。
- 读取实际落盘文件：恰好 1 行合法 JSON，operation 为 save_profile，与界面筛选一致。
- 未清理诊断、未修改常规应用数据。此验收不证明 Windows 系统保存对话框或代理接管。
