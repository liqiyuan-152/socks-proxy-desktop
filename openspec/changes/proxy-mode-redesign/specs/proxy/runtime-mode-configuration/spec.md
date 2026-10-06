## Purpose

定义参数化的代理运行时模式配置，允许在 Rules 模式下灵活控制国内直连和未匹配流量的默认行为。

## ADDED Requirements

### Requirement: Rules 模式支持参数化配置

系统 SHALL 支持 Rules 模式的参数化配置，包括 `use_china_direct` 和 `default_action` 两个参数。

#### Scenario: Rules 模式启用国内直连

- **WHEN** 用户将 Rules 模式配置为 `use_china_direct: true` 且 `default_action: Proxy`
- **THEN** 系统 SHALL 应用用户自定义规则，未匹配的中国大陆 IP/域名 SHALL 直连，其他未匹配流量 SHALL 走代理

#### Scenario: Rules 模式禁用国内直连

- **WHEN** 用户将 Rules 模式配置为 `use_china_direct: false` 且 `default_action: Direct`
- **THEN** 系统 SHALL 仅应用用户自定义规则，未匹配流量 SHALL 全部直连，不应用国内直连规则集

#### Scenario: Rules 模式的规则优先级

- **WHEN** Rules 模式下同时存在用户自定义规则、国内直连规则和默认动作
- **THEN** 系统 SHALL 按以下顺序匹配：(1) 用户自定义规则，(2) 国内直连规则（如果启用），(3) 默认动作

### Requirement: Global 和 Direct 模式保持无参数

系统 SHALL 保持 Global 和 Direct 模式为无参数的枚举值。

#### Scenario: Global 模式行为不变

- **WHEN** 用户选择 Global 模式
- **THEN** 系统 SHALL 将所有流量通过默认代理发送，不应用任何规则

#### Scenario: Direct 模式行为不变

- **WHEN** 用户选择 Direct 模式
- **THEN** 系统 SHALL 将所有流量直连，不应用任何规则

### Requirement: 默认动作必须为 Proxy 或 Direct

系统 SHALL 限制 `default_action` 参数的值为 `Proxy` 或 `Direct`。

#### Scenario: 设置有效的默认动作

- **WHEN** 用户尝试设置 `default_action` 为 `Proxy` 或 `Direct`
- **THEN** 系统 SHALL 接受该配置

#### Scenario: 拒绝无效的默认动作

- **WHEN** 用户尝试设置 `default_action` 为其他值
- **THEN** 系统 SHALL 拒绝该配置并返回验证错误

### Requirement: Rules 模式配置持久化

系统 SHALL 将 Rules 模式的参数持久化到数据库中。

#### Scenario: 保存 Rules 模式配置

- **WHEN** 用户修改 Rules 模式的 `use_china_direct` 或 `default_action` 参数
- **THEN** 系统 SHALL 将新配置持久化到数据库并在重启后恢复

#### Scenario: 数据库崩溃恢复保留 Rules 配置

- **WHEN** 数据库操作在保存 Rules 配置过程中失败
- **THEN** 系统 SHALL 回滚整个事务，保持原有配置不变

### Requirement: sing-box 配置生成反映 Rules 模式参数

系统 SHALL 根据 Rules 模式的参数动态生成 sing-box 路由配置。

#### Scenario: 生成启用国内直连的配置

- **WHEN** Rules 模式配置为 `use_china_direct: true`
- **THEN** 生成的 sing-box 配置 SHALL 包含中国大陆 IP 和域名的直连规则

#### Scenario: 生成禁用国内直连的配置

- **WHEN** Rules 模式配置为 `use_china_direct: false`
- **THEN** 生成的 sing-box 配置 SHALL 不包含中国大陆规则集

#### Scenario: 生成正确的默认路由

- **WHEN** Rules 模式配置了 `default_action`
- **THEN** 生成的 sing-box 配置的最终兜底规则 SHALL 匹配该 `default_action`（Direct 对应 direct outbound，Proxy 对应 proxy outbound）

### Requirement: IPC 契约支持 Rules 模式参数

系统 SHALL 在 IPC 契约中暴露 Rules 模式的参数化结构。

#### Scenario: 前端读取 Rules 模式配置

- **WHEN** 前端通过 IPC 获取运行时状态
- **THEN** 返回的 `RuntimeMode` SHALL 包含 Rules 模式的 `use_china_direct` 和 `default_action` 字段

#### Scenario: 前端设置 Rules 模式配置

- **WHEN** 前端通过 IPC 切换到 Rules 模式并指定参数
- **THEN** 后端 SHALL 接受完整的参数化 Rules 模式配置

### Requirement: 不支持内核的平台保存配置

系统 SHALL 在 macOS/Linux 保存完整模式参数，同时保持未应用模式与未接管系统代理的真实状态。

#### Scenario: macOS 修改 Rules 参数

- **WHEN** 用户在 macOS 修改 Rules 参数
- **THEN** 系统 SHALL 使用持久化事务保存配置，不启动内核，快照不声明代理运行成功
