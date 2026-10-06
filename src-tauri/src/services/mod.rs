//! 按业务能力划分的服务边界。
//!
//! 接口位于 [`interfaces`]，实现分别位于代理档案、路由和运行时模块。
//! 领域接口返回类型化错误，应用 facade 统一转换为 [`AppError`]，IPC 数据类型保持兼容。
//! 配置写入必须保持共享串行锁、恢复检查和持久化事务，不能直接调用
//! 存储的 `save` 方法绕过凭据暂存、运行时确认或失败回滚。
//!
//! # 使用示例
//!
//! ```
//! use socks_proxy_lib::services::{interfaces::ProfileService, AppError, ProfileView};
//! use socks_proxy_lib::domain_errors::{ProxyError, RoutingError, RuntimeError};
//! use std::sync::Arc;
//!
//! fn read_profiles(service: Arc<dyn ProfileService>) -> Result<Vec<ProfileView>, ProxyError> {
//!     service.list_profiles()
//! }
//! ```
//!
//! ```
//! use socks_proxy_lib::services::{
//!     interfaces::{RoutingService, RuntimeService},
//!     AppError, RouteTestResult, RuntimeMode, RuntimeSnapshot,
//! };
//! use socks_proxy_lib::domain_errors::{ProxyError, RoutingError, RuntimeError};
//! use std::sync::Arc;
//!
//! fn predict_route(service: Arc<dyn RoutingService>) -> Result<RouteTestResult, RoutingError> {
//!     service.test_route("example.com", 443)
//! }
//!
//! fn stop_proxy(service: Arc<dyn RuntimeService>) -> Result<RuntimeSnapshot, RuntimeError> {
//!     service.stop()
//! }
//!
//! fn change_mode(service: Arc<dyn RuntimeService>) -> Result<RuntimeSnapshot, RuntimeError> {
//!     service.set_mode(RuntimeMode::Rules {
//!         use_china_direct: false,
//!         default_action: socks_proxy_lib::services::RuleAction::Proxy,
//!     })
//! }
//! ```

pub mod adapters;
mod application_diagnostics;
mod application_errors;
pub mod application_service;
mod diagnostic_export;
pub use application_diagnostics::{HistoryUnavailable, NetworkRecoveryResult, ServiceHealth};
mod application_transfer;
pub(crate) mod context;
pub mod interfaces;
pub mod profile_service;
pub(crate) mod profile_types;
pub mod routing_service;
pub(crate) mod routing_types;
mod runtime_report;
pub mod runtime_service;
mod settings_service;

pub use crate::{
    error::AppError,
    models::{RoutingRule, RuleAction, RuntimeMode},
    route_test::RouteTestResult,
    runtime::RuntimeSnapshot,
};
pub use application_service::ApplicationService;
pub use profile_types::{ProfileCredentialView, ProfileInput, ProfileView};
pub use routing_types::ChinaDirectStatus;
