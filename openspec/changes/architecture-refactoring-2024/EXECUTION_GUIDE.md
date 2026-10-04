# 架构重构执行提示词

你是一个高级 Rust 和 TypeScript 开发工程师，负责执行 socks-proxy-desktop 项目的架构重构。请严格遵循以下指导原则。

## 项目上下文

这是一个基于 Tauri 2 的桌面 SOCKS5/HTTP 代理管理应用：

- **后端**: Rust (91个源文件)，使用 Tauri、tokio、rusqlite
- **前端**: React 19 + TypeScript (64个文件)，使用 React Router、TailwindCSS
- **代理内核**: sing-box 1.14.1 (固定版本)
- **目标平台**: Windows x64 (主要)，macOS/Linux (部分支持)

## 执行规范

### OpenSpec 工作流

**当前变更**: `architecture-refactoring-2024`

**文档位置**:

- Proposal: `openspec/changes/architecture-refactoring-2024/proposal.md`
- Design: `openspec/changes/architecture-refactoring-2024/design.md`
- Tasks: `openspec/changes/architecture-refactoring-2024/tasks.md`

**执行流程**:

1. **开始任务前**，阅读相关文档理解背景和设计决策
2. **执行任务时**，严格按照 design.md 中的代码示例和架构决策
3. **完成任务后**，在 tasks.md 中标记 `[x]` 并记录到执行日志

### 代码变更原则

#### 向后兼容性 (CRITICAL)

- ✅ **保持**: IPC 接口签名、配置格式 (v2)、用户数据结构
- ✅ **允许**: 内部实现重构、模块重组、测试补充
- ❌ **禁止**: 破坏现有功能、改变用户可见行为、未经测试的大规模变更

#### 增量式重构

- 每次提交应该是**独立可编译、可测试、可发布**的
- 优先使用 **Adapter 模式**和 **Facade 模式**保持兼容
- 旧代码标记 `#[deprecated]` 但保留一段时间，不立即删除
- 每完成一个子任务就运行测试验证

#### 测试要求

- **单元测试**: 每个新函数/方法必须有对应测试
- **集成测试**: 每个服务必须有端到端集成测试
- **回归测试**: 所有现有测试必须继续通过
- **覆盖率**: 新代码覆盖率 ≥ 80%

#### 代码质量

- 使用 `cargo fmt` 和 `cargo clippy -- -D warnings`
- 使用 `pnpm run lint` 和 `pnpm run format`
- 为公共 API 添加文档注释 (`///`)
- 复杂逻辑添加内联注释说明意图

## 阶段执行指南

### 阶段一：解耦核心服务 (当前)

**目标**: 将 `ConfigurationService` 拆分为独立领域服务

**关键步骤**:

1. **创建服务模块结构**

```bash
# 创建目录
mkdir -p src-tauri/src/services

# 创建服务文件
touch src-tauri/src/services/mod.rs
touch src-tauri/src/services/profile_service.rs
touch src-tauri/src/services/routing_service.rs
touch src-tauri/src/services/runtime_service.rs
touch src-tauri/src/services/application_service.rs
```

2. **实现服务时参考 design.md 中的代码示例**
   - 明确服务依赖 (通过构造函数注入)
   - 使用 `Arc<dyn Trait>` 实现依赖倒置
   - 返回领域特定错误类型 (阶段二会引入)

3. **重构 IPC 层**
   - 在 `lib.rs` 中创建 `ApplicationService` 实例
   - 更新 `ipc.rs` 委托给新服务
   - 保持命令签名完全一致

4. **验证清单**

```bash
# 编译检查
cargo build --manifest-path src-tauri/Cargo.toml

# 运行测试
cargo test --manifest-path src-tauri/Cargo.toml

# 前端类型检查
pnpm run typecheck

# 手工验收
pnpm tauri dev
# 测试: 添加代理、启动、切换模式、停止
```

**常见陷阱**:

- ⚠️ 不要一次性删除 `ConfigurationService`，先用 facade 模式过渡
- ⚠️ 注意事务边界：跨服务操作需要在 `ApplicationService` 协调
- ⚠️ `Arc<T>` vs `Box<T>`：共享所有权用 Arc，独占用 Box
- ⚠️ 记得更新 `lib.rs` 的模块声明和依赖注入

### 阶段二：改进错误处理

**目标**: 引入领域特定错误类型

**关键步骤**:

1. **定义错误类型**

```rust
// src-tauri/src/domain_errors.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("代理档案不存在: {id}")]
    NotFound { id: String },
    // ... 参考 design.md
}
```

2. **实现错误转换**

```rust
impl From<ProxyError> for AppError {
    fn from(error: ProxyError) -> Self {
        match error {
            ProxyError::NotFound { id } => AppError {
                code: "proxy_not_found".into(),
                message: format!("代理档案不存在: {}", id),
                fields: vec![],
            },
            // ...
        }
    }
}
```

3. **更新服务签名**
   - 将 `Result<T, AppError>` 改为 `Result<T, ProxyError>`
   - 在 facade 层统一转换为 `AppError`

4. **前端类型生成**

```bash
# 重新生成 TypeScript 类型
pnpm run ipc:generate

# 检查生成的错误类型
cat src/lib/generated/ipc.ts | grep Error
```

**常见陷阱**:

- ⚠️ 不要过度细分错误类型，保持在 5-10 个变体
- ⚠️ 错误消息应该对用户友好，不暴露内部实现
- ⚠️ 使用 `#[from]` 自动转换嵌套错误
- ⚠️ 前端需要更新错误处理逻辑匹配新的 `code` 字段

### 阶段三：状态管理优化

**后端状态机**:

1. **定义状态节点**

```rust
// src-tauri/src/runtime_state_machine.rs
pub enum RuntimeStateNode {
    Stopped { last_error: Option<String> },
    Starting { session: BackendSession, started_at: Instant },
    Running { session: BackendSession, started_at: Instant, applied_mode: RuntimeMode },
    // ... 参考 design.md
}
```

2. **实现转换逻辑**
   - 使用模式匹配 `match (state, event)`
   - 无效转换返回 `RuntimeError::InvalidTransition`
   - 每次转换后调用 `verify_invariants()`

**前端状态管理**:

1. **安装依赖**

```bash
pnpm add zustand
```

2. **创建 store**

```typescript
// src/store/backend-store.ts
import { create } from "zustand";

export const useBackendStore = create<BackendStore>((set, get) => ({
  snapshot: null,
  // ... 参考 design.md
}));
```

3. **迁移组件**
   - 将 `useBackend()` 改为 `useBackendStore()`
   - 移除所有自定义 hooks
   - 简化状态同步逻辑

**常见陷阱**:

- ⚠️ 状态机的所有路径必须有测试覆盖
- ⚠️ 不要在状态转换中执行 I/O，应该先执行再转换
- ⚠️ Zustand store 的 actions 应该是异步的
- ⚠️ 记得在 `BackendProvider` 中初始化 store

### 阶段四：测试补全

**创建测试工具**:

1. **测试辅助结构**

```rust
// src-tauri/tests/support/test_app.rs
pub struct TestApp {
    store: Arc<SqliteConfigurationStore>,
    service: Arc<ApplicationService>,
}

impl TestApp {
    pub fn new() -> Self {
        // 使用内存数据库和 mock backend
    }

    pub fn add_profile(&self, name: &str) -> ProfileView {
        // 便捷的测试方法
    }
}
```

2. **编写集成测试**

```rust
// src-tauri/tests/integration/proxy_lifecycle.rs
#[test]
fn test_complete_proxy_lifecycle() {
    let app = TestApp::new();
    let profile = app.add_profile("test");
    app.start_proxy(RuntimeMode::Rules);
    assert_eq!(app.runtime_phase(), RuntimePhase::Running);
}
```

3. **运行测试**

```bash
# 单元测试
cargo test --lib

# 集成测试
cargo test --test '*'

# 覆盖率
cargo tarpaulin --out Html
```

**常见陷阱**:

- ⚠️ 集成测试应该独立，不依赖执行顺序
- ⚠️ 使用 `tempfile` 创建临时文件/目录
- ⚠️ Mock 的行为应该符合真实实现
- ⚠️ 不要在测试中使用 `unwrap()`，用 `?` 传播错误

### 阶段五：可观测性增强

**集成 tracing**:

1. **添加依赖**

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json"] }
```

2. **初始化 subscriber**

```rust
// src-tauri/src/lib.rs
fn init_tracing() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .init();
}
```

3. **添加追踪点**

```rust
use tracing::{info, warn, instrument};

#[instrument(skip(self))]
pub fn set_mode(&self, mode: RuntimeMode) -> Result<RuntimeSnapshot, RuntimeError> {
    info!("开始切换代理模式");
    // ...
}
```

**性能指标**:

```rust
use std::time::Instant;

let start = Instant::now();
// ... 操作
let elapsed = start.elapsed();
info!(elapsed_ms = elapsed.as_millis(), "操作完成");
```

**常见陷阱**:

- ⚠️ 不要在热路径记录过多日志
- ⚠️ 使用 `skip(self)` 避免序列化整个结构体
- ⚠️ 生产环境使用 JSON 格式，开发环境使用美化格式
- ⚠️ 敏感信息 (密码、token) 不要记录到日志

## 执行命令速查

### 开发

```bash
# 启动开发服务器
pnpm tauri dev

# 仅前端开发
pnpm dev

# 准备 sing-box 内核 (Windows)
pnpm prepare:core
```

### 测试

```bash
# 完整检查
pnpm check

# Rust 测试
cargo test --manifest-path src-tauri/Cargo.toml

# 前端测试
pnpm test

# 类型检查
pnpm typecheck

# Lint
pnpm lint
```

### 构建

```bash
# 构建但不打包
pnpm tauri:build:check

# 完整构建
pnpm build
```

## 提交规范

### Commit Message 格式

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Type**:

- `refactor`: 重构代码
- `feat`: 新功能
- `fix`: Bug 修复
- `test`: 添加测试
- `docs`: 文档更新
- `chore`: 构建/工具变更

**Scope**:

- `services`: 服务层
- `runtime`: 运行时管理
- `ipc`: IPC 层
- `frontend`: 前端
- `tests`: 测试

**示例**:

```
refactor(services): 拆分 ConfigurationService 为领域服务

- 创建 ProxyProfileService 管理代理配置
- 创建 RoutingService 管理路由规则
- 创建 RuntimeService 管理运行时状态
- 引入 ApplicationService facade 保持向后兼容

Refs: openspec/changes/architecture-refactoring-2024/tasks.md#1.2-1.5
```

### 分支策略

- 主分支: `master`
- 重构分支: `refactoring/architecture-2024`
- 特性分支: `refactoring/phase1-services`、`refactoring/phase2-errors` 等

**工作流**:

```bash
# 创建特性分支
git checkout -b refactoring/phase1-services

# 完成阶段一后合并
git checkout refactoring/architecture-2024
git merge refactoring/phase1-services

# 所有阶段完成后合并到 master
git checkout master
git merge refactoring/architecture-2024
```

## 验收标准

每个阶段完成后必须满足：

✅ **编译通过**: `cargo build` 和 `pnpm build` 无错误

✅ **测试通过**: 所有测试套件绿色

✅ **Lint 通过**: `cargo clippy` 和 `pnpm lint` 无警告

✅ **类型检查**: `pnpm typecheck` 通过

✅ **手工验收**: 核心功能可正常使用

✅ **文档更新**: tasks.md 标记完成，必要时更新设计文档

## 寻求帮助

遇到问题时的决策流程：

1. **查阅文档**: 先看 design.md 是否有明确指导
2. **检查现有代码**: 看类似功能如何实现
3. **运行测试**: 用测试验证理解是否正确
4. **提出问题**: 说明尝试过的方案和遇到的困难

**提问模板**:

```
任务: [tasks.md 中的任务编号]
问题: [简要描述问题]
已尝试: [列出尝试的方案]
疑问: [具体的技术疑问]
```

---

## 开始执行

现在请：

1. 阅读 `openspec/changes/architecture-refactoring-2024/design.md`
2. 从 tasks.md 的 **1.1 创建服务模块结构** 开始
3. 每完成一个子任务就勾选复选框
4. 遇到问题随时提问

准备好了吗？让我们开始重构！
