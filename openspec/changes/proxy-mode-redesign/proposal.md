## Why

当前的代理模式设计存在语义模糊的问题：`Rules` 模式同时承担了"应用用户自定义分流规则"和"国内直连"两种职责，导致用户难以理解其具体行为。此外，缺少针对 GFW 绕过场景的专门模式，以及未匹配流量的默认处理策略不明确。本次重新设计旨在通过参数化的 `Rules` 模式，提供更清晰、更灵活的代理行为控制。

## What Changes

- **BREAKING**: 将 `RuntimeMode::Rules` 改为参数化枚举，支持 `use_china_direct` 和 `default_action` 配置
- 移除全局的 `china_direct_enabled` 配置字段，将其整合到 `Rules` 模式参数中
- 在 Rules 模式下明确未匹配流量的默认行为（走代理或直连）
- 更新 sing-box 配置生成逻辑，根据 Rules 模式参数动态生成路由规则
- 调整前端 UI，为 Rules 模式提供国内直连开关和默认动作选择
- 提供数据库迁移逻辑，将现有配置平滑迁移到新结构
- 更新相关文档和测试用例

## Capabilities

### New Capabilities

- `proxy/runtime-mode-configuration`: 参数化的运行时模式配置，支持 Rules 模式的国内直连和默认动作设置
- `proxy/runtime-mode-migration`: 从旧的 `RuntimeMode::Rules` + 全局 `china_direct_enabled` 迁移到新的参数化模式

### Modified Capabilities

<!-- 当前无主规格，因此没有已有能力需要修改 -->

## Impact

### 后端影响

- `src-tauri/src/models.rs`: `RuntimeMode` 枚举定义
- `src-tauri/src/store.rs`: 数据库 schema 和迁移逻辑
- `src-tauri/src/configuration*.rs`: 配置读取和验证
- `src-tauri/src/sing_box_config.rs`: sing-box 配置生成
- `src-tauri/src/routing.rs`: 路由规则编译逻辑
- `src-tauri/src/services/`: 应用服务层的模式切换

### 前端影响

- `src/lib/generated/`: IPC 类型定义（重新生成）
- `src/pages/status/index.tsx`: 状态页的模式切换 UI
- `src/pages/settings/`: 设置页的 Rules 模式配置界面
- `src/store/`: 前端状态管理

### 测试影响

- 所有涉及 `RuntimeMode::Rules` 的单元测试和集成测试
- 配置迁移测试
- sing-box 配置生成测试
- 路由规则匹配测试

### 破坏性变更

- 数据库 schema 版本升级（v6 → v7），配置文档版本升级（v2 → v3）
- IPC 契约变更，需要重新生成前端类型
- 旧版本程序无法读取新配置数据库
