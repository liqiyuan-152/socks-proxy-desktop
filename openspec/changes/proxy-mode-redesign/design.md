## Context

当前 `RuntimeMode` 定义为简单枚举 `Rules | Global | Direct`，其中 `Rules` 模式的行为受全局配置 `china_direct_enabled` 影响。这导致：

1. 模式语义不清晰：`Rules` 既可能启用国内直连，也可能不启用
2. 未匹配流量的默认行为隐式绑定到代理，无法配置
3. 国内直连配置与模式选择分离，增加理解成本

本设计将 `RuntimeMode` 重构为参数化枚举，将国内直连和默认动作整合到 `Rules` 模式的参数中。

相关约束：

- 数据库使用 SQLite，schema 版本通过 `user_version` 管理
- sing-box 配置生成逻辑在 `sing_box_config.rs` 中
- 配置修改需通过 `ConfigurationContext` 的串行锁和恢复门禁
- IPC 契约变更需运行 `pnpm ipc:generate` 更新前端类型

## Goals / Non-Goals

**Goals:**

- 将 `RuntimeMode::Rules` 改为参数化结构，支持 `use_china_direct` 和 `default_action`
- 提供从 v2 到 v3 的自动数据库迁移
- 更新 sing-box 配置生成逻辑以反映新参数
- 更新前端 UI 以支持 Rules 模式参数配置
- 保持 `Global` 和 `Direct` 模式的简单性

**Non-Goals:**

- 不支持更复杂的默认动作（如 Reject、多代理选择、负载均衡）
- 不改变用户自定义规则的语法或匹配逻辑
- 不修改国内直连规则集的内容或来源
- 不支持 Rules 模式之外的参数化（Global/Direct 保持无参数）

## Decisions

### Decision 1: 使用 Rust 枚举变体参数而非独立配置字段

**选择：** 将 Rules 模式定义为带参数的枚举变体

```rust
pub enum RuntimeMode {
    Rules { use_china_direct: bool, default_action: RuleAction },
    Global,
    Direct,
}
```

**替代方案：**

- 保持 Rules 为单纯枚举值，参数存储在 `PersistedConfiguration` 的顶层字段
- 创建独立的 `RulesModeConfig` 结构体

**理由：**

- 参数和模式绑定在一起，类型系统保证 Global/Direct 无法访问这些参数
- 序列化/反序列化逻辑更自然，避免"模式 + 配置"的同步问题
- 与 Rust 惯用法一致，利用枚举的表达能力

**影响：**

- 数据库序列化需要自定义逻辑（将枚举展平为列）
- IPC 类型生成需要支持参数化枚举（`ts-rs` 已支持）

### Decision 2: 默认动作默认值为 Proxy

**选择：** 迁移时将所有 Rules 模式的 `default_action` 设置为 `Proxy`

**理由：**

- 符合大多数用户的使用场景（绕过 GFW）
- 与现有隐式行为一致（当前未匹配流量走代理）
- 避免迁移后行为突变

**影响：**

- 少数希望默认直连的用户需要手动调整
- 文档需要说明默认行为

### Decision 3: 数据库序列化策略

**实际架构适配（用户已确认）：** SQLite 当前版本为 6，配置为 `configuration.document_json`，模式为 `selected_mode.mode`。追加数据库版本 7 的事务迁移，在 `selected_mode` 表存储 `rules_*` 列，配置文档升级到 v3，以 `runtime_mode` 保存完整枚举并移除顶层 `china_direct_enabled`。列与文档在同一配置事务中写入并读取校验；迁移同时转换持久化恢复记录中的配置，失败完整回滚。旧程序按已有版本门禁拒绝打开 v7 数据库。模式参数变更通过共享 `ConfigurationContext` 的 durable commit，不绕过恢复边界。

**选择：** 使用列展平方式存储参数化枚举

```sql
mode TEXT NOT NULL,  -- 'rules' | 'global' | 'direct'
rules_use_china_direct INTEGER,  -- NULL for non-Rules mode
rules_default_action TEXT        -- NULL for non-Rules mode
```

**替代方案：**

- 使用 JSON blob 存储整个 RuntimeMode
- 为每种模式创建独立表

**理由：**

- 保持与现有 schema 的一致性（使用列而非 JSON）
- 支持 SQL 查询和索引（虽然当前不需要）
- 类型安全：`rules_*` 列仅在 `selected_mode = 'rules'` 时有值

**迁移逻辑：**

1. 添加两列：`rules_use_china_direct INTEGER`, `rules_default_action TEXT`
2. 更新现有 `mode = 'rules'` 的行：
   - `rules_use_china_direct = china_direct_enabled`
   - `rules_default_action = 'proxy'`
3. 删除配置 JSON 中 `china_direct_enabled` 字段
4. 更新数据库 `user_version` 为 7，配置文档版本为 3

### Decision 4: sing-box 配置生成逻辑

**选择：** 在 `generate_sing_box_config()` 中根据 `RuntimeMode` 参数动态构建路由规则

**当前逻辑：**

- Rules 模式：应用用户规则 + 可选国内直连 + 隐式代理兜底
- Global 模式：单一代理规则
- Direct 模式：单一直连规则

**新逻辑：**

- `Rules { use_china_direct: true, default_action: Proxy }`：
  ```
  1. 用户自定义规则
  2. 国内直连规则集
  3. 默认走代理
  ```
- `Rules { use_china_direct: false, default_action: Direct }`：
  ```
  1. 用户自定义规则
  2. 默认直连
  ```

**实现细节：**

- 复用现有 `CompiledRules` 和 `ChinaRuleSets`
- 在路由规则数组末尾添加兜底规则：
  ```rust
  let final_rule = match mode {
      RuntimeMode::Rules { default_action, .. } => match default_action {
          RuleAction::Proxy => Route { outbound: "proxy" },
          RuleAction::Direct => Route { outbound: "direct" },
      },
      RuntimeMode::Global => Route { outbound: "proxy" },
      RuntimeMode::Direct => Route { outbound: "direct" },
  };
  ```

### Decision 5: 前端 UI 设计

**macOS 验收适配（按用户授权的推荐方案执行）：** 保持 macOS/Linux 的内核与系统代理能力为不可用，支持通过相同 durable commit 保存模式及 Rules 参数。无运行时能力时仅提交配置，不启动候选代理会话；快照的已应用模式保持未应用，界面明确说明仅保存配置。国内直连实际匹配仍依赖固定 Windows 内核，macOS 验收验证配置生成、用户规则预测和注入匹配器的优先级测试，不宣称真实代理联网成功。

**选择：** 在状态页面提供模式选择器 + Rules 模式参数配置

**UI 结构：**

1. **模式选择器**（Radio Group）：
   - 规则代理（Rules）
   - 全局代理（Global）
   - 全局直连（Direct）

2. **Rules 模式配置面板**（仅在选择 Rules 时显示）：
   - 国内直连开关（Switch）：`use_china_direct`
   - 默认动作选择（Select）：
     - 走代理（Proxy）
     - 直连（Direct）

**交互逻辑：**

- 切换模式时立即调用后端 IPC `set_runtime_mode`
- Rules 参数变更时防抖后调用 IPC
- 后端验证失败时回滚前端状态并展示错误

### Decision 6: IPC 契约变更

**选择：** 更新 `ipc_contract.rs` 中的类型定义，重新生成前端类型

**实际架构适配（用户已确认）：** `ipc_contract.rs` 复用 `models.rs` 的 `RuntimeMode`，后者已经通过 `#[cfg_attr(test, derive(ts_rs::TS))]` 实现类型导出。保留单一模型定义，在契约测试中验证 Rules 参数的序列化、反序列化和生成的 TypeScript 声明，不创建重复的 IPC 枚举。为执行任务 1.2 的 `cargo test`，提前适配旧 Rust 测试构造值；任务 8.1 仍负责完整行为覆盖验证。

**变更内容：**

```rust
// ipc_contract.rs
#[derive(Serialize, Deserialize, TS)]
pub enum RuntimeMode {
    Rules {
        use_china_direct: bool,
        default_action: RuleAction,
    },
    Global,
    Direct,
}
```

**生成流程：**

1. 修改 `ipc_contract.rs`
2. 运行 `cargo test --manifest-path src-tauri/Cargo.toml` 触发类型生成
3. 运行 `pnpm ipc:generate` 复制类型到前端
4. 更新前端使用点

## Risks / Trade-offs

### Risk 1: 迁移失败导致用户无法启动应用

**风险：** 数据库迁移过程中出现意外错误（如磁盘空间不足、权限问题）

**缓解措施：**

- 所有迁移在单个事务中执行，失败时完整回滚
- 迁移前验证数据完整性（如 `selected_mode` 值合法性）
- 记录详细日志，便于用户报告问题
- 提供降级路径：用户可手动删除数据库文件重新初始化（丢失配置）

### Risk 2: 默认动作选择不符合部分用户预期

**风险：** 迁移后统一设置为 `default_action: Proxy`，少数依赖"未匹配流量直连"的用户会发现行为变化

**缓解措施：**

- 更新日志中明确说明默认行为
- 在设置页面提供显著的默认动作配置入口
- 考虑在首次启动后展示迁移提示（可选，增加复杂度）

### Risk 3: Rules 模式参数增加用户学习成本

**风险：** 从单纯的"规则代理"变为需要理解两个参数的模式

**缓解措施：**

- UI 提供清晰的参数说明和默认推荐值
- 预设场景快捷配置（如"智能分流"预设 `use_china_direct: true, default_action: Proxy`）
- 文档中提供使用场景示例

### Trade-off 1: 灵活性 vs 简单性

**选择：** 接受 Rules 模式的复杂性以换取灵活性

**影响：**

- 用户需要理解国内直连和默认动作的含义
- 高级用户获得更强的控制能力
- 未来可扩展更多参数（如自定义规则优先级）

### Trade-off 2: 破坏性变更 vs 向后兼容

**选择：** 接受 schema 版本升级的破坏性变更

**影响：**

- 旧版本程序无法打开新数据库
- 避免维护双重配置逻辑的技术债务
- 简化实现和测试

## Migration Plan

### Phase 1: 后端实现（估计 2 天）

1. 修改 `RuntimeMode` 枚举定义
2. 实现数据库迁移逻辑（v2 → v3）
3. 更新 `store.rs` 的序列化/反序列化逻辑
4. 更新 `sing_box_config.rs` 的配置生成
5. 更新所有使用 `RuntimeMode::Rules` 的业务逻辑
6. 更新 IPC 契约

### Phase 2: 前端实现（估计 1 天）

1. 运行 `pnpm ipc:generate` 更新类型
2. 修改状态页面的模式选择器
3. 添加 Rules 模式参数配置 UI
4. 更新前端状态管理逻辑
5. 调整相关测试

### Phase 3: 测试（估计 1 天）

1. 数据库迁移测试（v2 → v3 各种配置组合）
2. sing-box 配置生成测试
3. 前端 UI 交互测试
4. 端到端真实代理测试（Windows）
5. 迁移回归测试（确保旧用户场景仍工作）

### Phase 4: 文档和发布（估计 0.5 天）

1. 更新 CHANGELOG.md 说明破坏性变更
2. 更新用户文档
3. 准备发布说明
4. 后续正式发布以 0.3.0 为目标（遵循语义化版本）；本次实施只添加未发布变更记录，不修改当前 0.2.2 应用版本，版本升级留在发布流程处理。

### Rollback Strategy

**场景 1: 发布前发现问题**

- 回滚代码到主分支
- 不涉及用户数据

**场景 2: 发布后用户报告迁移失败**

- 提供诊断工具收集错误日志
- 提供手动修复脚本（如强制重建数据库）
- 紧急发布修复版本

**场景 3: 用户需要回退到旧版本**

- 旧版本程序拒绝打开 v3 数据库（按设计）
- 用户需要手动备份配置、删除数据库、重新配置
- 文档中提供备份建议

## Open Questions

无。所有关键决策已在本设计中明确。
