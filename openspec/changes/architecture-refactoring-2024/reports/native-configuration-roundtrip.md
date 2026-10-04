# 原生配置导入导出验收

2026-10-04 使用 CUA 操作隔离 macOS debug 应用 com.socksproxy.desktop.refactor-validation，未操作常规应用数据。

- 原生保存选择器取消后没有成功或错误提示。
- 保存 /tmp/socks-architecture-config-roundtrip.json，实际文件为 schema_version 2，包含 1 个代理和 1 条规则，不含 password 字段。
- 在代理编辑界面将“架构验收代理”名称修改为“导入前临时修改”并保存。
- 设置页文件选择器选择上述导出文件，预览后确认导入。界面显示“配置已导入；密码没有从导出文件恢复”。
- 代理页名称恢复为“架构验收代理”，服务器 proxy.example.com、端口 1081、SOCKS5、启用状态保持；规则页“架构验收规则”匹配 example.com 并引用恢复后的代理。
- 再次通过原生保存选择器保存 /tmp/socks-architecture-config-after-import.json。两个文件解析后的完整 JSON 对象相等；首个文件 SHA-256 为 cfd45470d47da00a21ced8fdbfa9935188e5d11d8d0884a7a5470952b1b2fa04。

该证据覆盖无认证配置的真实原生往返、保存取消和完整配置相等。认证导入隔离、失败回滚及保存失败保留由独立后端测试覆盖；这不是 Windows 原生选择器或自动化 E2E 套件证据，4.5.4 保持未勾选。
