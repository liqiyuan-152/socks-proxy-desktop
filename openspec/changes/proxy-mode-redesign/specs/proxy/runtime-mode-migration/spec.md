## Purpose

确保从旧的 `RuntimeMode::Rules` + 全局 `china_direct_enabled` 配置平滑迁移到新的参数化 Rules 模式，保证数据完整性和向后兼容性。

## ADDED Requirements

### Requirement: 数据库 schema 版本升级

系统 SHALL 将数据库 schema 版本从 v6 升级到 v7；配置文档从 v2 升级到 v3。

#### Scenario: 新安装使用 v7 schema

- **WHEN** 用户首次安装应用
- **THEN** 系统 SHALL 创建 v7 schema 的数据库，`user_version` SHALL 为 7

#### Scenario: 旧版本程序拒绝打开新数据库

- **WHEN** 支持数据库 v6 或更早版本的程序尝试打开 v7 数据库
- **THEN** 系统 SHALL 拒绝打开并返回错误，不得降级或修改 `user_version`

### Requirement: 从 v2 到 v3 的自动迁移

系统 SHALL 在首次以 v3 运行时自动执行数据库迁移。

#### Scenario: 迁移 Rules 模式 + china_direct_enabled = true

- **WHEN** v6 数据库中 `selected_mode = 'rules'` 且 JSON 中 `china_direct_enabled = true`
- **THEN** 迁移后 SHALL 设置为 `Rules { use_china_direct: true, default_action: Proxy }`

#### Scenario: 迁移 Rules 模式 + china_direct_enabled = false

- **WHEN** v6 数据库中 `selected_mode = 'rules'` 且 JSON 中 `china_direct_enabled = false`
- **THEN** 迁移后 SHALL 设置为 `Rules { use_china_direct: false, default_action: Proxy }`

#### Scenario: 迁移 Global 和 Direct 模式

- **WHEN** v6 数据库中 `selected_mode` 为 `'global'` 或 `'direct'`
- **THEN** 迁移后 SHALL 保持为 `Global` 或 `Direct`，不受 `china_direct_enabled` 影响

#### Scenario: 迁移失败时回滚事务

- **WHEN** 数据库迁移过程中发生错误
- **THEN** 系统 SHALL 回滚整个迁移事务，保持 `user_version = 6`，并向用户报告错误

### Requirement: 默认动作的迁移策略

系统 SHALL 在迁移时为 Rules 模式设置合理的默认动作。

#### Scenario: 迁移时默认动作为 Proxy

- **WHEN** v6 数据库中 `selected_mode = 'rules'`
- **THEN** 迁移后的 `default_action` SHALL 默认为 `Proxy`，符合大多数用户的使用场景

### Requirement: 移除 china_direct_enabled 全局配置

系统 SHALL 在 v7 schema 中移除配置 JSON 中的 `china_direct_enabled` 字段。

#### Scenario: v7 数据库不包含 china_direct_enabled

- **WHEN** 迁移完成后查询数据库表结构
- **THEN** 配置 JSON SHALL 不包含 `china_direct_enabled` 字段

#### Scenario: 旧配置值已合并到 Rules 模式参数

- **WHEN** 迁移完成后读取 Rules 模式配置
- **THEN** 原 `china_direct_enabled` 的值 SHALL 已体现在 `use_china_direct` 参数中

### Requirement: 迁移过程的事务完整性

系统 SHALL 在单个事务中完成所有迁移操作。

#### Scenario: 迁移操作的原子性

- **WHEN** 执行 v2 到 v3 的迁移
- **THEN** 系统 SHALL 在同一事务中完成 schema 修改、数据转换和版本更新

#### Scenario: 部分成功不允许提交

- **WHEN** 迁移过程中任意步骤失败
- **THEN** 系统 SHALL 回滚所有已执行的修改，不得留下部分迁移状态

### Requirement: 迁移后的数据验证

系统 SHALL 在迁移完成后验证数据的一致性。

#### Scenario: 验证 Rules 模式参数有效性

- **WHEN** 迁移完成后加载配置
- **THEN** 系统 SHALL 验证 Rules 模式的 `use_china_direct` 为布尔值，`default_action` 为 `Proxy` 或 `Direct`

#### Scenario: 验证模式枚举值有效性

- **WHEN** 迁移完成后加载配置
- **THEN** 系统 SHALL 验证 `selected_mode` 为有效的枚举值（`Rules`、`Global` 或 `Direct`）

### Requirement: 迁移日志记录

系统 SHALL 记录迁移过程的关键信息。

#### Scenario: 记录迁移开始和完成

- **WHEN** 数据库迁移开始和完成
- **THEN** 系统 SHALL 记录日志，包含迁移的源版本和目标版本

#### Scenario: 记录迁移的配置转换

- **WHEN** 迁移过程中转换 Rules 模式配置
- **THEN** 系统 SHALL 记录转换前后的配置值（不包含敏感信息）
