## 1. 后端核心模型修改

- [x] 1.1 修改 `src-tauri/src/models.rs` 中的 `RuntimeMode` 枚举为参数化结构，添加 `Rules { use_china_direct: bool, default_action: RuleAction }` 变体，验证 `cargo check` 通过
- [x] 1.2 沿用 `models.rs` 中共享 `RuntimeMode` 的测试环境 `TS` derive，在 `src-tauri/src/ipc_contract.rs` 验证参数化枚举的序列化和类型导出；提前适配 Rust 测试中的旧 Rules 用法，验证运行 `cargo test` 后生成的 TypeScript 类型包含参数化枚举（用户已确认此任务边界调整；8.1 仍保留完整行为覆盖验证）

## 2. 数据库 Schema 升级和迁移

- [x] 2.1 在 `src-tauri/src/store.rs` 中通过 `store_migrations.rs` 定义数据库 v7 schema（配置文档 v3），添加 `rules_use_china_direct` 和 `rules_default_action` 列，验证 schema 定义符合 SQLite 语法
- [x] 2.2 实现数据库 v6 到 v7、配置文档 v2 到 v3 的迁移函数，包含添加新列、数据转换和移除配置 JSON 中旧 `china_direct_enabled` 字段的逻辑，验证迁移测试覆盖所有配置组合（Rules+启用/禁用国内直连、Global、Direct）
- [x] 2.3 更新 `CONFIG_SCHEMA_VERSION` 常量为 3，验证新安装时创建的数据库 `user_version` 为 7，配置文档版本为 3
- [x] 2.4 添加旧版本拒绝打开 v7 数据库的测试用例，验证 支持数据库 v6 的代码尝试打开 v7 数据库时返回错误

## 3. 配置序列化和反序列化

- [x] 3.1 更新 `src-tauri/src/store.rs` 的 配置读取函数以读取新的 `rules_*` 列，验证可以正确加载 Rules 模式的参数
- [x] 3.2 更新 `src-tauri/src/store.rs` 的 `save` 函数以正确写入参数化的 RuntimeMode，验证 Rules 模式参数持久化到数据库
- [x] 3.3 添加配置验证逻辑，确保 `rules_use_china_direct` 和 `rules_default_action` 仅在 `selected_mode = 'rules'` 时有值，验证测试覆盖边界情况

## 4. sing-box 配置生成逻辑

- [x] 4.1 修改 `src-tauri/src/sing_box_config.rs` 中的 `generate_sing_box_config` 函数，根据 `RuntimeMode::Rules` 的参数动态生成路由规则，验证生成的配置在 `use_china_direct: true` 时包含国内直连规则
- [x] 4.2 实现根据 `default_action` 参数生成正确的兜底路由规则，验证 `default_action: Proxy` 时最终规则走 proxy outbound，`default_action: Direct` 时走 direct outbound
- [x] 4.3 更新相关测试用例，验证不同 Rules 参数组合生成的 sing-box 配置正确

## 5. 路由规则编译逻辑

- [x] 5.1 检查 `src-tauri/src/routing.rs` 和 `src-tauri/src/route_test.rs` 中的逻辑是否需要适配新的 RuntimeMode 结构，验证规则匹配优先级符合规格要求（用户规则 > 国内直连 > 默认动作）
- [x] 5.2 更新路由测试用例以覆盖新的 Rules 模式参数场景，验证测试通过

## 6. 业务服务层适配

- [x] 6.1 更新 `src-tauri/src/services/runtime_service.rs` 中的模式切换逻辑以处理参数化 RuntimeMode，验证切换到 Rules 模式时参数正确传递
- [x] 6.2 更新 `src-tauri/src/services/application_service.rs` 中使用 RuntimeMode 的所有方法，验证参数正确传递到底层服务
- [x] 6.3 检查 `src-tauri/src/configuration*.rs` 文件中的模式相关逻辑，确保兼容新结构，验证配置验证和恢复逻辑正常工作

## 7. IPC 层更新

- [x] 7.1 更新 `src-tauri/src/ipc.rs` 中的 `set_runtime_mode` 命令签名以接受参数化 RuntimeMode，验证前端可以传递 Rules 模式参数
- [x] 7.2 更新 `src-tauri/src/ipc.rs` 中的 `get_runtime_status` 响应以包含 Rules 模式参数，验证前端可以读取当前 Rules 配置
- [x] 7.3 运行 `pnpm ipc:generate` 生成前端 TypeScript 类型，验证 `src/lib/generated/` 中包含更新后的 RuntimeMode 定义

## 8. Rust 测试更新

- [x] 8.1 更新所有使用 `RuntimeMode::Rules` 的单元测试（搜索 `RuntimeMode::Rules` 并逐个修改），验证所有 Rust 测试通过
- [x] 8.2 更新集成测试中的模式切换场景，添加 Rules 参数测试用例，验证 `cargo test --manifest-path src-tauri/Cargo.toml` 全部通过
- [x] 8.3 添加迁移回归测试，模拟旧配置迁移到新结构的各种场景，验证迁移后的配置行为符合预期

## 9. 前端状态管理

- [x] 9.1 更新 `src/store/` 中的状态定义以使用新生成的 RuntimeMode 类型，验证 TypeScript 类型检查通过
- [x] 9.2 更新 `src/lib/backend.ts` 中调用 `set_runtime_mode` 的逻辑以传递 Rules 参数，验证 IPC 调用签名正确
- [x] 9.3 更新前端测试夹具 `src/test/app-fixture.ts` 中的 RuntimeMode mock 数据，验证测试使用新结构

## 10. 前端 UI 实现

- [x] 10.1 修改 `src/pages/status/index.tsx` 的模式选择器，添加 Rules 模式的参数配置 UI（国内直连开关和默认动作选择器），验证界面在 macOS 上正确渲染
- [x] 10.2 实现 Rules 参数变更的防抖逻辑和 IPC 调用，验证参数修改后正确保存到后端
- [x] 10.3 添加参数配置的帮助文本和默认值提示，验证用户可以理解每个参数的含义
- [x] 10.4 处理后端验证错误，在前端展示错误信息并回滚状态，验证错误处理流程

## 11. 前端测试更新

- [x] 11.1 更新 `src/App.tsx` 和相关组件测试中使用 RuntimeMode 的地方，验证 `pnpm test` 通过
- [x] 11.2 添加 Rules 模式参数配置 UI 的交互测试，验证开关和选择器状态同步

## 12. 质量检查

- [x] 12.1 运行 `pnpm check` 验证前端质量检查全部通过（包括 lint、typecheck 和测试）；原工作区的 4 个无关 Markdown 格式问题保持原样，在临时副本中仅规范它们的格式并移除忽略的验证产物后，完整检查通过
- [x] 12.2 运行 `pnpm rust:fmt` 和 `pnpm rust:clippy` 验证 Rust 代码格式和 lint 通过
- [x] 12.3 运行 `cargo test --manifest-path src-tauri/Cargo.toml --locked` 验证所有 Rust 测试通过

## 13. macOS 端到端验收

- [x] 13.1 在 macOS 上运行 `pnpm tauri dev`，使用独立应用标识和 v6 数据库 / v2 文档验收数据，验证应用正常启动且迁移逻辑执行成功
- [x] 13.2 测试 Rules 模式参数配置 UI，验证国内直连开关和默认动作选择器可以正常切换
- [x] 13.3 配置 Rules 模式为 `use_china_direct: true, default_action: Proxy`，添加自定义规则，验证用户规则预测及生成配置 / 注入匹配测试的优先级（用户规则 > 国内直连 > 默认代理）；macOS 不执行 Windows 内核匹配或声明真实代理联网成功
- [x] 13.4 配置 Rules 模式为 `use_china_direct: false, default_action: Direct`，验证配置与路由预测仅应用用户规则且未匹配流量默认直连；macOS 不声明真实代理联网成功
- [x] 13.5 测试 Global 和 Direct 模式切换，验证行为保持不变
- [x] 13.6 模拟 数据库 v6 与配置文档 v2 升级场景，验证迁移后配置正确且应用可以正常使用

## 14. 文档更新

- [x] 14.1 更新 `CHANGELOG.md` 说明破坏性变更和迁移逻辑，验证变更日志格式正确
- [x] 14.2 更新 `docs/architecture.md` 中关于 RuntimeMode 的说明，验证文档与实现一致
- [x] 14.3 更新 `AGENTS.md` 中的当前项目情况描述，反映新的代理模式设计
