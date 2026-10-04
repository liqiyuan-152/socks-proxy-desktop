//! 应用组合所需的适配器契约与数据类型，供独立集成测试及替代实现注入。
pub use crate::{
    configuration_recovery::{RecoveryIntent, RecoveryRecord, StartupEntry},
    credentials::{CredentialStore, CredentialUpdate, ProxyCredential},
    models::{PersistedConfiguration, ProxyProtocol, RuleAction, RuleMatcher},
    runtime::{BackendSession, RuntimeBackend, RuntimeCoordinator, RuntimePhase, SessionHealth},
    startup::StartupAdapter,
    store::{ConfigurationStore, SqliteConfigurationStore},
};
