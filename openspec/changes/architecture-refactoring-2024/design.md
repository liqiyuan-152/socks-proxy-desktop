## Context

当前代码库包含 91 个 Rust 源文件和 64 个 TypeScript/TSX 文件，实现了一个完整的 SOCKS5/HTTP 代理管理桌面应用。核心架构包括：

- **配置服务层**：`ConfigurationService` 统一管理所有业务逻辑
- **运行时管理**：`ManagedRuntime` 管理代理进程生命周期
- **数据持久化**：SQLite + 系统凭据存储
- **前后端通信**：Tauri IPC + TypeScript 类型生成

现有架构存在的主要问题：

1. **服务职责不清**：`ConfigurationService` 混合了配置、凭据、运行时、延迟测试等多种职责
2. **状态管理分散**：运行时状态转换分散在 `runtime_manager.rs`、`runtime_transition.rs` 等多个文件
3. **错误类型宽泛**：`AppError { code, message, fields }` 缺少类型安全和上下文
4. **前端状态混乱**：多个自定义 hooks 导致状态同步复杂
5. **测试覆盖不足**：缺少集成测试和端到端测试

这些问题在功能扩展时会放大，需要系统性重构。

## Goals / Non-Goals

**Goals:**

- **服务解耦**：按领域拆分服务，每个服务有清晰的职责边界
- **类型安全**：使用领域特定错误类型，提供结构化上下文
- **状态可预测**：显式状态机，明确的转换规则和不变量
- **测试完备**：集成测试覆盖关键场景，单元测试覆盖率 80%+
- **可观测**：结构化日志、性能指标、诊断工具
- **向后兼容**：保持 IPC 接口和配置格式兼容

**Non-Goals:**

- 不改变用户可见功能
- 不迁移配置格式（v2 已经稳定）
- 不改变 IPC 接口签名（内部实现可以重构）
- 不引入新的外部依赖（除了 tracing、zustand 等必要工具）
- 不重写已经工作良好的模块（如 sing-box 集成）

## Decisions

### 1. 服务拆分架构

采用领域驱动设计（DDD）原则，按业务能力拆分服务：

```rust
// 代理档案服务：管理代理配置和凭据
pub struct ProxyProfileService {
    store: Arc<dyn ConfigurationStore>,
    credentials: Arc<dyn CredentialStore>,
    validator: ProfileValidator,
}

impl ProxyProfileService {
    pub fn list_profiles(&self) -> Result<Vec<ProfileView>, ProxyError>;
    pub fn save_profile(&self, input: ProfileInput) -> Result<ProfileView, ProxyError>;
    pub fn delete_profile(&self, id: &str) -> Result<(), ProxyError>;
    pub fn get_credential(&self, id: &str) -> Result<ProfileCredential, ProxyError>;
}

// 路由服务：管理路由规则和国内直连
pub struct RoutingService {
    store: Arc<dyn ConfigurationStore>,
    china_rules: Option<Arc<ChinaRulesProvider>>,
}

impl RoutingService {
    pub fn list_rules(&self) -> Result<Vec<RoutingRule>, RoutingError>;
    pub fn replace_rules(&self, rules: Vec<RoutingRule>) -> Result<(), RoutingError>;
    pub fn test_route(&self, target: &str, port: u16) -> Result<RouteTestResult, RoutingError>;
    pub fn get_china_status(&self) -> Result<ChinaDirectStatus, RoutingError>;
}

// 运行时服务：管理代理运行时状态
pub struct RuntimeService {
    runtime: Arc<dyn RuntimeCoordinator>,
    mode_store: Arc<dyn ConfigurationStore>,
}

impl RuntimeService {
    pub fn snapshot(&self) -> RuntimeSnapshot;
    pub fn set_mode(&self, mode: RuntimeMode) -> Result<RuntimeSnapshot, RuntimeError>;
    pub fn stop(&self) -> Result<RuntimeSnapshot, RuntimeError>;
    pub fn recover_network(&self) -> Result<RuntimeSnapshot, RuntimeError>;
}

// 应用服务：统一 facade
pub struct ApplicationService {
    profiles: Arc<ProxyProfileService>,
    routing: Arc<RoutingService>,
    runtime: Arc<RuntimeService>,
    settings: Arc<SettingsService>,
}

impl ApplicationService {
    // 委托给具体服务
    pub fn list_profiles(&self) -> Result<Vec<ProfileView>, AppError> {
        self.profiles.list_profiles().map_err(Into::into)
    }
}
```

**优势**：

- 每个服务职责单一，易于测试和维护
- 服务间通过接口依赖，可以独立演进
- Facade 保持向后兼容，IPC 层无需大改

**权衡**：

- 增加了代码行数和文件数量
- 需要明确服务边界和依赖关系

### 2. 领域特定错误类型

阶段一实施时保留现有跨资源持久化协议：`ApplicationService` 持有共享的 `ConfigurationContext`，向领域服务注入该对象。对象集中持有通过 `Arc<dyn Trait>` 注入的配置存储、凭据存储、启动项与运行时适配器，以及共享串行锁、恢复阻塞标记和持久化事务协调逻辑。领域服务不调用其他领域服务，也不依赖旧 `ConfigurationService`；所有配置变更复用同一恢复日志边界，避免独立服务各自持锁或直接保存数据库造成凭据与运行时状态分裂。上面的构造函数示例表达领域职责，实际构造还需注入此共享事务边界。接口位于 `services/interfaces.rs`，具体实现位于各领域模块，以免 trait 与具体结构同名冲突。

使用 `thiserror` 定义每个服务的错误类型：

```rust
#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("代理档案不存在: {id}")]
    NotFound { id: String },

    #[error("代理正在被使用，无法删除")]
    InUse {
        references: Vec<ReferenceLocation>,
    },

    #[error("凭据验证失败")]
    CredentialInvalid(#[from] CredentialError),

    #[error("代理档案验证失败")]
    ValidationFailed(#[from] ValidationError),

    #[error("存储操作失败: {0}")]
    StorageFailed(#[from] StorageError),
}

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("运行时正在恢复中，操作被拒绝")]
    RecoveryInProgress,

    #[error("进程启动失败: {reason}")]
    ProcessStartFailed { reason: String },

    #[error("无效的状态转换: {from:?} -> {to:?}")]
    InvalidTransition {
        from: RuntimePhase,
        to: RuntimePhase,
    },

    #[error("系统代理设置失败: {0}")]
    SystemProxyFailed(String),
}

// 顶层错误转换
impl From<ProxyError> for AppError {
    fn from(error: ProxyError) -> Self {
        match error {
            ProxyError::NotFound { id } => AppError {
                code: "proxy_not_found".into(),
                message: format!("代理档案不存在: {}", id),
                fields: vec![],
            },
            ProxyError::InUse { references } => AppError {
                code: "proxy_in_use".into(),
                message: "代理正在被使用，无法删除".into(),
                fields: references.into_iter().map(|r| FieldError {
                    field: r.field,
                    message: r.description,
                }).collect(),
            },
            // ... 其他转换
        }
    }
}
```

阶段二实施约定：`AppError` 保留原有 `code`、`message`、`fields`，新增可选的 `context`。上下文包含错误 ID、UTC 毫秒、根因领域及变体名称、操作名称和恢复建议；同一错误的嵌套转换保留身份。上下文使用 `Box` 控制错误值大小，未补充上下文的旧错误仍序列化为原有三字段结构。服务接口返回领域错误，facade 通过 `Into<AppError>` 统一转换；未迁移适配器按稳定错误码分类，无法确定的根因保留原始错误，不按消息文字猜测。

堆栈只收集 Rust 符号名，过滤代码路径、地址和超长符号，最多 16 帧；它仅进入本地诊断，不进入 IPC。应用错误日志及持久化诊断不记录用户输入、错误消息、字段值或凭据。诊断使用与返回错误相同的 ID，写入失败仅记录警告，不替代业务错误；模式切换和恢复的既有诊断路径避免为同一次失败生成重复身份。诊断 summary 保留安全的 JSON 详情；阶段 2.5 将数据库升级到 schema 5，新增可选 error_type 与 operation 独立字段，迁移旧结构化摘要但保留原始记录与配置 v2 文档。按错误编号幂等写入，按类型/操作/级别聚合不同编号，JSON Lines 导出通过单次 SQLite 读取保留完整过滤快照与各次身份。旧二进制沿用版本保护；回退需匹配数据库及受保护凭据备份。

**优势**：

- 类型安全，编译时检查错误处理
- 结构化上下文，便于诊断和恢复
- 前端可以根据错误类型做精准处理

**权衡**：

- 需要定义多个错误枚举
- 需要实现错误转换逻辑

### 3. 显式状态机

将运行时状态建模为显式状态节点和转换：

```rust
// 状态节点
pub enum RuntimeStateNode {
    Stopped {
        last_error: Option<String>,
    },
    Starting {
        session: BackendSession,
        started_at: Instant,
    },
    Running {
        session: BackendSession,
        started_at: Instant,
        applied_mode: RuntimeMode,
    },
    Switching {
        current: BackendSession,
        candidate: BackendSession,
    },
    Recovering {
        failed_session: Option<BackendSession>,
        attempt: u32,
    },
    Failed {
        error: String,
        last_session: Option<BackendSession>,
    },
}

// 状态转换事件
pub enum RuntimeEvent {
    StartRequested {
        config: PersistedConfiguration,
        mode: RuntimeMode,
    },
    ProcessStarted {
        session: BackendSession,
    },
    HealthCheckPassed,
    ModeSwitchRequested {
        mode: RuntimeMode,
    },
    ProcessExited {
        code: Option<i32>,
    },
    RecoverySucceeded {
        session: BackendSession,
    },
    RecoveryFailed {
        error: String,
    },
    StopRequested,
}

// 状态转换逻辑
impl RuntimeStateNode {
    pub fn transition(
        self,
        event: RuntimeEvent,
        backend: &dyn RuntimeBackend,
    ) -> Result<Self, RuntimeError> {
        match (self, event) {
            // Stopped -> Starting
            (Self::Stopped { .. }, RuntimeEvent::StartRequested { config, mode }) => {
                let session = backend.transition(None, &config, mode, 0)?;
                Ok(Self::Starting {
                    session: session.expect("新启动必须有 session"),
                    started_at: Instant::now(),
                })
            }

            // Starting -> Running
            (Self::Starting { session, started_at }, RuntimeEvent::HealthCheckPassed) => {
                Ok(Self::Running {
                    session,
                    started_at,
                    applied_mode: mode,
                })
            }

            // Running -> Recovering
            (Self::Running { session, .. }, RuntimeEvent::ProcessExited { .. }) => {
                Ok(Self::Recovering {
                    failed_session: Some(session),
                    attempt: 0,
                })
            }

            // 无效转换
            (state, event) => Err(RuntimeError::InvalidTransition {
                from: state.phase(),
                to: event.target_phase(),
            }),
        }
    }

    // 状态不变量验证
    pub fn verify_invariants(&self) -> Result<(), RuntimeError> {
        match self {
            Self::Running { session, .. } => {
                if session.process_id == 0 {
                    return Err(RuntimeError::InvariantViolated(
                        "Running 状态必须有有效的 process_id".into()
                    ));
                }
            }
            // ... 其他不变量
        }
        Ok(())
    }
}
```

**优势**：

- 状态转换逻辑集中，易于验证正确性
- 不可能的状态转换在编译时被拒绝
- 状态不变量可以自动验证

**权衡**：

- 需要重构现有状态管理代码
- 状态节点增加了内存开销（可忽略）

阶段三实施约定：现有运行时先发布 Starting/Switching 再执行后端 I/O，配置更新的候选会话还需等待持久化确认。因此节点使用可选候选会话，ProcessStarted 仅保存已准备的候选，协调器在后端健康检查及持久化确认完成后才提交 HealthCheckPassed。状态转换不调用 RuntimeBackend；协调器保留原有 prepare/confirm/revert 协议。ActiveSession 保存会话、已应用模式与起始时间；失败节点允许保留健康的旧会话，退出或恢复失败则撤销已应用会话。Stopped 区分已应用 Direct 与显式停止的 None。事件转换借用旧节点，校验前后不变量，错误时不消费旧状态。原 IPC 快照契约保持不变；ManagedRuntime 已通过事件更新节点；候选配置保存完整旧节点用于回滚。诊断历史记录最近 128 次脱敏事件，详见 docs/runtime-state-machine.md。

### 4. 前端统一状态管理

使用 Zustand 替代多个自定义 hooks：

```typescript
// store/backend-store.ts
interface BackendStore {
  // 状态
  snapshot: RuntimeSnapshot | null;
  profiles: ProxyProfile[];
  rules: RoutingRule[];
  connections: ActiveConnectionsSnapshot;
  loading: boolean;
  error: BackendError | null;

  // 计算属性
  selectedMode: ProxyMode | null;
  isRunning: boolean;

  // 操作
  initialize: () => Promise<void>;
  refreshSnapshot: () => Promise<void>;
  refreshProfiles: () => Promise<void>;
  switchMode: (mode: ProxyMode) => Promise<void>;
  saveProfile: (input: ProfileInput) => Promise<void>;
  deleteProfile: (id: string) => Promise<void>;

  // 内部方法
  _applySnapshot: (snapshot: RuntimeSnapshot) => void;
  _setError: (error: BackendError | null) => void;
}

export const useBackendStore = create<BackendStore>((set, get) => ({
  snapshot: null,
  profiles: [],
  rules: [],
  connections: { available: false, connections: [] },
  loading: false,
  error: null,

  get selectedMode() {
    return get().snapshot?.selected_mode ?? null;
  },

  get isRunning() {
    return get().snapshot?.phase === 'running';
  },

  initialize: async () => {
    set({ loading: true });
    try {
      const [snapshot, profiles, rules] = await Promise.all([
        getRuntimeSnapshot(),
        listProfiles(),
        listRules(),
      ]);
      set({ snapshot, profiles, rules, error: null });

      // 订阅运行时快照更新
      await onRuntimeSnapshot((next) => {
        get()._applySnapshot(next);
      });
    } catch (error) {
      set({ error: error as BackendError });
    } finally {
      set({ loading: false });
    }
  },

  switchMode: async (mode) => {
    set({ loading: true, error: null });
    try {
      const snapshot = await setRuntimeMode(mode);
      set({ snapshot, loading: false });
    } catch (error) {
      set({ error: error as BackendError, loading: false });
    }
  },

  _applySnapshot: (snapshot) => {
    set({ snapshot });
  },
}));

// 组件中使用
function StatusPage() {
  const { snapshot, selectedMode, isRunning, switchMode } = useBackendStore();

  return (
    <div>
      <p>当前模式: {selectedMode}</p>
      <p>运行状态: {isRunning ? '运行中' : '已停止'}</p>
      <button onClick={() => switchMode('global')}>
        切换到全局代理
      </button>
    </div>
  );
}
```

**优势**：

- 单一状态源，消除状态同步问题
- 选择器自动计算派生状态
- 易于调试和追踪状态变化

**权衡**：

- 需要迁移现有 hooks 逻辑
- 学习新的状态管理模式

阶段三前端实施约定：store 按 BackendProvider 隔离，使用 Zustand vanilla store 和 React 选择器，跨组件共享状态但不跨窗口共享生命周期。异步读取使用资源版本与生命周期校验；模式切换保留串行队列，订阅和可见性轮询由 store 初始化/清理管理。代理、规则、设置和导入写操作由 store actions 协调刷新；表单草稿与凭据不存入 store。开发模式启用 devtools。详见 docs/frontend-state.md。

### 5. 集成测试框架

创建测试辅助工具和集成测试：

```rust
// tests/support/test_app.rs
pub struct TestApp {
    store: Arc<SqliteConfigurationStore>,
    service: Arc<ApplicationService>,
    runtime: Arc<TestRuntime>,
}

impl TestApp {
    pub fn new() -> Self {
        let store = Arc::new(SqliteConfigurationStore::open_in_memory().unwrap());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        let runtime = Arc::new(TestRuntime::new());

        let profiles = Arc::new(ProxyProfileService::new(
            store.clone(),
            credentials.clone(),
        ));
        let routing = Arc::new(RoutingService::new(store.clone(), None));
        let runtime_service = Arc::new(RuntimeService::new(
            runtime.clone(),
            store.clone(),
        ));

        let service = Arc::new(ApplicationService::new(
            profiles,
            routing,
            runtime_service,
        ));

        Self { store, service, runtime }
    }

    pub fn add_profile(&self, name: &str) -> ProfileView {
        self.service.save_profile(ProfileInput {
            id: None,
            name: name.into(),
            protocol: ProxyProtocol::Socks5,
            host: "127.0.0.1".into(),
            port: 1080,
            authentication_enabled: false,
            enabled: true,
            credential: None,
        }).unwrap()
    }

    pub fn start_proxy(&self, mode: RuntimeMode) {
        self.service.set_mode(mode).unwrap();
    }

    pub fn runtime_phase(&self) -> RuntimePhase {
        self.service.snapshot().phase
    }

    pub fn kill_backend(&self) {
        self.runtime.simulate_exit(0);
    }
}

// tests/integration/proxy_lifecycle.rs
#[test]
fn test_complete_proxy_lifecycle() {
    let app = TestApp::new();

    // 添加代理
    let profile = app.add_profile("测试代理");
    app.service.select_profile(Some(profile.id.clone())).unwrap();

    // 启动规则模式
    app.start_proxy(RuntimeMode::Rules);
    assert_eq!(app.runtime_phase(), RuntimePhase::Running);

    // 切换到全局模式
    app.service.set_mode(RuntimeMode::Global).unwrap();
    assert_eq!(app.runtime_phase(), RuntimePhase::Running);

    // 停止
    app.service.stop().unwrap();
    assert_eq!(app.runtime_phase(), RuntimePhase::Stopped);
}

#[test]
// 2026-10-04 用户确认：自动恢复网络，用户手动重试启动。
// Recovering 示例仅描述网络恢复过程；实际稳定状态为 Failed/Exited。
// 保留已提交配置、所选模式及宿主崩溃后的持久恢复契约。
fn test_network_restoration_and_explicit_retry_on_crash() {
    let app = TestApp::new();

    let profile = app.add_profile("测试代理");
    app.service.select_profile(Some(profile.id)).unwrap();
    app.start_proxy(RuntimeMode::Rules);

    // 模拟进程崩溃
    app.kill_backend();

    // 检测内核退出后自动恢复网络，等待用户显式重试。
    app.wait_for_backend_exit();
    assert_eq!(app.runtime_phase(), RuntimePhase::Failed);
    assert!(app.original_network_restored());
    app.start_proxy(RuntimeMode::Rules);
    assert_eq!(app.runtime_phase(), RuntimePhase::Running);
}
```

**优势**：

- 测试真实的用户场景
- 发现单元测试无法发现的集成问题
- 提供回归测试保护

**权衡**：

- 测试运行时间较长
- 需要维护测试基础设施

### 6. 结构化可观测性

使用 `tracing` 提供结构化日志和追踪：

```rust
use tracing::{info, warn, error, instrument, Span};

impl RuntimeService {
    #[instrument(skip(self), fields(mode = ?mode))]
    pub fn set_mode(&self, mode: RuntimeMode) -> Result<RuntimeSnapshot, RuntimeError> {
        info!("开始切换代理模式");

        let start = Instant::now();
        let result = self.mode_store.save_mode(mode)
            .map_err(RuntimeError::from)?;

        match self.runtime.request_mode(mode) {
            Ok(snapshot) => {
                let elapsed = start.elapsed();
                info!(
                    elapsed_ms = elapsed.as_millis(),
                    phase = ?snapshot.phase,
                    "代理模式切换成功"
                );
                Ok(snapshot)
            }
            Err(error) => {
                error!(
                    error = %error,
                    "代理模式切换失败"
                );
                Err(error)
            }
        }
    }
}

// 初始化 tracing subscriber
fn init_tracing() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
}
```

**优势**：

- 结构化日志便于查询和分析
- 自动追踪调用链路
- 可以根据环境调整日志级别

**权衡**：

- 轻微的性能开销（通常可忽略）
- 需要合理选择追踪点

## Risks / Trade-offs

### 重构规模大

**风险**：五个阶段涉及大量代码变更，可能引入新 bug。

**缓解措施**：

- 分阶段进行，每个阶段保持独立可发布
- 每个阶段完成后运行完整测试套件
- 使用 feature flag 控制新代码路径
- 保持向后兼容，允许回滚

### 性能影响

**风险**：新增的服务抽象、错误转换、日志追踪可能影响性能。

**缓解措施**：

- 在关键路径进行性能基准测试
- 使用 `tracing` 的动态级别控制
- 优化热路径代码
- 监控启动时间、配置应用时间等关键指标

### 团队学习成本

**风险**：新的架构模式需要团队学习和适应。

**缓解措施**：

- 编写详细的架构文档和示例
- 提供开发者指南和最佳实践
- 在代码审查中传播知识
- 保留现有模式一段时间，逐步迁移

### 测试维护成本

**风险**：新增大量测试代码，维护成本增加。

**缓解措施**：

- 使用测试辅助工具减少重复代码
- 编写清晰的测试文档
- 定期审查和重构测试代码
- 使用快照测试减少手工断言

## Migration Plan

### 阶段一：服务解耦（第 1-2 周）

**目标**：拆分 `ConfigurationService`，建立新的服务架构。

**步骤**：

1. 创建 `services/` 模块，定义服务接口
2. 实现 `ProxyProfileService`、`RoutingService`、`RuntimeService`
3. 实现 `ApplicationService` facade
4. 更新 IPC 层调用新服务（保持接口不变）
5. 运行完整测试套件验证功能不变
6. 标记旧 `ConfigurationService` 为 deprecated

**验证**：

- 所有现有测试通过
- 手工验收测试覆盖关键场景
- 性能基准测试无显著退化

### 阶段二：错误处理改进（第 3 周）

**目标**：引入领域特定错误类型，提升错误诊断能力。

**步骤**：

1. 定义 `ProxyError`、`RuntimeError`、`RoutingError`
2. 实现错误类型之间的转换
3. 更新服务方法返回新错误类型
4. 更新前端错误处理和展示
5. 增强诊断日志

**验证**：

- 错误处理测试覆盖所有错误路径
- 前端正确展示不同类型错误
- 诊断信息更清晰可操作

### 阶段三：状态管理优化（第 4 周）

**目标**：实现显式状态机，统一前端状态管理。

**步骤**：

1. 定义 `RuntimeStateNode` 和 `RuntimeEvent`
2. 实现状态转换逻辑和不变量验证
3. 重构 `ManagedRuntime` 使用新状态机
4. 前端迁移到 Zustand
5. 消除旧的自定义 hooks

**验证**：

- 状态转换测试覆盖所有路径
- 不变量验证捕获非法状态
- 前端状态同步无竞态条件

### 阶段四：测试补全（第 5-6 周）

**目标**：建立集成测试框架，提升测试覆盖率。

**步骤**：

1. 创建 `tests/support/` 测试辅助工具
2. 实现 `TestApp`、`MockBackend` 等
3. 编写集成测试用例（代理生命周期、配置恢复等）
4. 编写端到端测试用例
5. 补充单元测试，覆盖率达到 80%+

**验证**：

- 集成测试覆盖关键用户场景
- 端到端测试可在 CI 中运行
- 代码覆盖率报告显示 80%+

### 阶段五：可观测性增强（第 7 周）

**目标**：集成结构化日志和性能指标。

**步骤**：

1. 集成 `tracing` 和 `tracing-subscriber`
2. 在关键路径添加 span 和 event
3. 实现性能指标收集
4. 改进运行时诊断导出
5. 提供开发者诊断工具

**验证**：

- 日志输出结构化且可查询
- 性能指标准确反映系统状态
- 诊断工具帮助定位问题

### 最终验收

在所有阶段完成后：

1. 运行完整测试套件（单元、集成、端到端）
2. 执行性能基准测试，确认无退化
3. 进行手工验收测试
4. 更新架构文档和开发者指南
5. 发布 changelog 和迁移指南（针对贡献者）
