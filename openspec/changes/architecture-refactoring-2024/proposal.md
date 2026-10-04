## Why

当前代码库在功能完整性上已达到产品可用状态，但存在以下架构和工程质量问题：

- **服务耦合度高**：`ConfigurationService` 承担了配置管理、凭据存储、运行时协调、延迟测试等多重职责，违反单一职责原则，测试和维护成本高。
- **状态管理分散**：运行时状态转换逻辑分散在多个文件中（`runtime_manager.rs`、`runtime_transition.rs`、`runtime_recovery_tests.rs`），状态机语义不清晰，难以验证转换正确性。
- **错误处理粗糙**：使用宽泛的 `AppError` 结构，前端难以做精准的错误恢复和用户提示，诊断信息不足。
- **前端状态混乱**：多个自定义 hooks 管理状态（`useBackendObservation`、`useModeSwitching`、`useRuntimePolling`），状态同步复杂，容易产生竞态条件。
- **测试覆盖不足**：虽然有大量单元测试文件，但缺少集成测试和端到端测试，无法验证完整用户场景。
- **可观测性缺失**：缺少结构化日志、性能指标和运行时追踪，生产环境问题难以定位。

这些问题会导致：功能扩展时改动范围大、引入回归的风险高、新贡献者上手困难、生产问题难以诊断。需要系统性重构来提升代码质量、可维护性和可扩展性。

## What Changes

分五个阶段进行架构重构和工程质量提升：

### 阶段一：解耦核心服务 (1-2周)

- 将 `ConfigurationService` 拆分为独立的领域服务：`ProxyProfileService`、`RoutingService`、`RuntimeService`
- 引入 `ApplicationService` facade 统一对外接口
- 重构 IPC 层调用新服务架构
- 保持向后兼容，确保现有功能不受影响

### 阶段二：改进错误处理 (1周)

- 使用 `thiserror` 定义领域特定错误类型（`ProxyError`、`RuntimeError`、`StorageError`）
- 为每种错误添加结构化上下文和恢复建议
- 更新前端错误展示和恢复逻辑
- 增强诊断信息的可操作性

### 阶段三：状态管理优化 (1周)

- 后端：实现显式状态机，明确状态转换图和不变量
- 前端：迁移到 Zustand 统一状态管理，消除 hooks 竞态
- 简化运行时快照同步逻辑
- 添加状态转换追踪和验证

### 阶段四：测试补全 (1-2周)

- 添加集成测试框架和测试用例（代理生命周期、配置恢复、系统代理接管）
- 添加端到端测试（完整用户场景）
- 提高单元测试覆盖率到 80%+
- 增强测试可读性和维护性

### 阶段五：可观测性增强 (1周)

- 集成 `tracing` 提供结构化日志
- 添加性能指标收集（启动时间、配置应用时间、系统代理切换延迟）
- 改进运行时诊断导出（JSON Lines 格式、字段标准化）
- 提供开发者诊断工具（状态快照导出、日志过滤）

## Capabilities

### New Capabilities

- `service-decoupling`: 领域驱动的服务拆分、依赖注入、清晰的服务边界
- `structured-errors`: 领域特定错误类型、错误上下文、恢复建议
- `explicit-state-machine`: 显式状态转换图、不变量验证、状态追踪
- `integration-testing`: 集成测试框架、端到端测试、场景覆盖
- `structured-observability`: 结构化日志、性能指标、诊断工具

### Modified Capabilities

- `configuration-service`: 拆分为多个领域服务，通过 facade 保持向后兼容
- `runtime-coordination`: 状态机重构，转换逻辑更清晰
- `frontend-state-management`: 统一状态管理，消除多 hooks 竞态
- `error-handling`: 从宽泛错误到领域特定错误
- `diagnostics`: 从文本消息到结构化日志和指标

## Impact

### Rust 后端

- **模块结构**：新增 `services/` 目录，拆分 `configuration_service.rs` 为多个文件
- **错误类型**：新增 `domain_errors.rs`，定义领域错误枚举
- **状态机**：重构 `runtime.rs`、`runtime_manager.rs`，引入显式状态节点
- **测试**：新增 `tests/integration/` 目录，添加集成测试
- **可观测性**：在关键路径添加 `tracing` span 和 event
- **依赖**：添加 `tracing`、`tracing-subscriber`、`thiserror`（已有）

### 前端

- **状态管理**：新增 `store/` 目录，迁移到 Zustand
- **错误处理**：更新错误展示组件，添加恢复操作
- **类型**：更新生成的 IPC 类型，添加错误枚举
- **依赖**：添加 `zustand`、`zod`、`react-hook-form`

### 测试基础设施

- 新增 `tests/` 目录用于集成测试和端到端测试
- 新增测试辅助工具（`TestApp`、`MockBackend`）
- 更新 CI 工作流，运行新测试套件

### 文档

- 更新架构文档，说明服务拆分和状态机
- 添加开发者指南，解释新的错误处理和状态管理模式
- 添加测试指南，说明如何编写集成测试

### 向后兼容性

- **配置格式**：不变
- **IPC 接口**：保持兼容，内部实现重构
- **用户数据**：无迁移需求
- **现有功能**：所有功能保持不变

### 风险

- **重构规模大**：分阶段进行，每阶段保持可发布状态
- **测试覆盖变更**：可能发现隐藏 bug，需要及时修复
- **性能影响**：新增的追踪和日志可能有轻微性能开销，需要性能测试
