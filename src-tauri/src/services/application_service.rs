//! 应用 facade：统一委托领域服务，共享跨资源事务边界。
pub use crate::services::profile_types::{ProfileCredentialView, ProfileInput, ProfileView};
pub use crate::services::routing_types::ChinaDirectStatus;
use crate::{
    credentials::{CredentialStore, CredentialUpdate},
    error::AppError,
    error_context::ErrorDomain,
    models::{AppSettings, RoutingRule, RuntimeMode},
    observability::ActiveConnectionsSnapshot,
    route_test::RouteTestResult,
    runtime::{RuntimeCoordinator, RuntimeSnapshot},
    services::{
        context::ConfigurationContext,
        interfaces::{
            ProfileService, RoutingService as RoutingServiceInterface,
            RuntimeService as RuntimeServiceInterface,
        },
        profile_service::ProxyProfileService,
        routing_service::RoutingService,
        runtime_service::RuntimeService,
        settings_service::SettingsService,
    },
    startup::StartupAdapter,
    store::ConfigurationStore,
};
#[cfg(any(windows, test))]
use std::path::PathBuf;
use std::{collections::HashMap, sync::Arc};

/// 应用业务入口，IPC、托盘和恢复流程统一使用此 facade。
pub struct ApplicationService {
    pub(crate) context: Arc<ConfigurationContext>,
    profiles: Arc<dyn ProfileService>,
    routing: Arc<dyn RoutingServiceInterface>,
    runtime: Arc<dyn RuntimeServiceInterface>,
    settings: Arc<SettingsService>,
}

impl ApplicationService {
    /// 注入存储、凭据、启动与运行时适配器，创建共享事务边界。
    pub fn new(
        store: Box<dyn ConfigurationStore>,
        credentials: Box<dyn CredentialStore>,
        startup: Box<dyn StartupAdapter>,
        runtime: Box<dyn RuntimeCoordinator>,
    ) -> Self {
        let context = Arc::new(ConfigurationContext::new(
            Arc::from(store),
            Arc::from(credentials),
            Arc::from(startup),
            Arc::from(runtime),
        ));
        let profiles = Arc::new(ProxyProfileService::new(context.clone()));
        let routing = Arc::new(RoutingService::new(context.clone()));
        let runtime = Arc::new(RuntimeService::new(context.clone()));
        let settings = Arc::new(SettingsService::new(context.clone()));
        Self {
            settings,
            runtime,
            context,
            profiles,
            routing,
        }
    }

    /// 继承启动恢复阻塞标记，防止覆盖未完成的事务。
    pub(crate) fn with_startup_recovery_pending(self, pending: bool) -> Self {
        self.context
            .startup_recovery_pending
            .store(pending, std::sync::atomic::Ordering::Relaxed);
        self
    }
    #[cfg(any(windows, test))]
    /// 注入离线国内直连规则集路径。
    pub fn with_china_rule_root(self, root: PathBuf) -> Self {
        *self
            .context
            .china_rule_root
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = Some(root);
        self
    }
    #[cfg(any(windows, test))]
    /// 注入异步测速注册表，配置提交后使旧任务失效。
    pub(crate) fn with_latency_tasks(
        self,
        registry: Arc<crate::latency_tasks::LatencyTaskRegistry>,
    ) -> Self {
        *self
            .context
            .latency_tasks
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = Some(registry);
        self
    }
    /// 返回不含凭据的代理档案列表。
    pub fn list_profiles(&self) -> Result<Vec<ProfileView>, AppError> {
        self.finish(
            self.profiles.list_profiles(),
            ErrorDomain::Proxy,
            "list_profiles",
        )
    }
    /// 显式读取指定档案的当前版本凭据。
    pub fn profile_credential(&self, id: &str) -> Result<ProfileCredentialView, AppError> {
        self.finish(
            self.profiles.get_credential(id),
            ErrorDomain::Proxy,
            "get_profile_credential",
        )
    }
    /// 校验并事务性保存代理档案和凭据。
    pub fn save_profile(&self, input: ProfileInput) -> Result<ProfileView, AppError> {
        self.finish(
            self.profiles.save_profile(input),
            ErrorDomain::Proxy,
            "save_profile",
        )
    }
    /// 删除未被引用的档案，通过共享事务应用运行时变更。
    pub fn delete_profile(&self, id: &str) -> Result<(), AppError> {
        self.finish(
            self.profiles.delete_profile(id),
            ErrorDomain::Proxy,
            "delete_profile",
        )
    }
    /// 设置或清除默认代理，并验证配置能否应用。
    pub fn select_profile(&self, id: Option<String>) -> Result<(), AppError> {
        self.finish(
            self.profiles.select_profile(id),
            ErrorDomain::Proxy,
            "select_profile",
        )
    }
    /// 按匹配优先级读取路由规则。
    pub fn list_rules(&self) -> Result<Vec<RoutingRule>, AppError> {
        self.finish(
            self.routing.list_rules(),
            ErrorDomain::Routing,
            "list_rules",
        )
    }
    /// 读取配置并同步系统实际启动项状态。
    pub fn settings(&self) -> Result<AppSettings, AppError> {
        self.finish(
            self.settings.settings(),
            ErrorDomain::Application,
            "get_settings",
        )
    }
    /// 获取底层运行时当前快照，不主动启动代理。
    pub fn runtime_snapshot(&self) -> RuntimeSnapshot {
        self.runtime.snapshot()
    }
    /// 获取内核可验证的活跃连接，能力不足时降级。
    pub fn active_connections(&self) -> ActiveConnectionsSnapshot {
        self.runtime.active_connections()
    }
    /// 校验连接可观测性后复制详情，并统一记录失败上下文。
    pub fn copy_active_connection_detail(&self, id: &str) -> Result<String, AppError> {
        self.finish(
            self.active_connections().copy_detail(id),
            ErrorDomain::Runtime,
            "copy_active_connection_detail",
        )
    }
    /// 先持久化用户选择，再请求运行时切换模式。
    pub fn request_mode(&self, mode: RuntimeMode) -> Result<RuntimeSnapshot, AppError> {
        self.finish(
            self.runtime.set_mode(mode),
            ErrorDomain::Runtime,
            "set_runtime_mode",
        )
    }
    /// 供 IPC 使用的模式切换，记录与返回错误拥有相同身份的诊断。
    pub fn request_mode_with_diagnostics(
        &self,
        mode: RuntimeMode,
    ) -> Result<RuntimeSnapshot, AppError> {
        let result = self.runtime.set_mode(mode).map_err(|error| {
            super::application_errors::annotate(
                error.into(),
                ErrorDomain::Runtime,
                "set_runtime_mode",
            )
        });
        if let Err(error) = &result {
            self.record_mode_failure(mode, error);
        }
        result
    }
    /// 持久化直连模式并停止代理。
    pub fn stop_runtime(&self) -> Result<RuntimeSnapshot, AppError> {
        self.finish(self.runtime.stop(), ErrorDomain::Runtime, "stop_runtime")
    }
    /// 恢复网络和未完成配置事务，失败时保持写入阻塞。
    pub fn recover_network(&self) -> Result<RuntimeSnapshot, AppError> {
        self.finish(
            self.runtime.recover_network(),
            ErrorDomain::Runtime,
            "recover_network",
        )
    }
    /// 通过共享事务协调配置和系统启动项。
    pub fn update_settings(&self, settings: AppSettings) -> Result<AppSettings, AppError> {
        self.finish(
            self.settings.update_settings(settings),
            ErrorDomain::Application,
            "update_settings",
        )
    }
    /// 导出兼容 v2 格式且不含凭据的配置。
    pub fn export(&self) -> Result<String, AppError> {
        self.finish(
            (|| crate::transfer::export_configuration_json(&self.context.store.load()?))(),
            ErrorDomain::Application,
            "export_configuration",
        )
    }
    /// 验证代理引用后替换全部路由规则。
    pub fn replace_rules(&self, rules: Vec<RoutingRule>) -> Result<(), AppError> {
        self.finish(
            self.routing.replace_rules(rules),
            ErrorDomain::Routing,
            "replace_rules",
        )
    }
    /// 用完整且唯一的标识序列调整规则优先级。
    pub fn reorder_rules(&self, ids: &[String]) -> Result<(), AppError> {
        self.finish(
            self.routing.reorder_rules(ids),
            ErrorDomain::Routing,
            "reorder_rules",
        )
    }
    /// 读取国内直连开关和规则集可用性。
    pub fn china_direct_status(&self) -> Result<ChinaDirectStatus, AppError> {
        self.finish(
            self.routing.get_china_status(),
            ErrorDomain::Routing,
            "get_china_direct_status",
        )
    }
    /// 在一致配置版本上离线评估目标路由。
    pub fn test_route(&self, target: &str, port: u16) -> Result<RouteTestResult, AppError> {
        self.finish(
            self.routing.test_route(target, port),
            ErrorDomain::Routing,
            "test_route",
        )
    }
    /// 验证规则集和默认代理后切换国内直连开关。
    pub fn set_china_direct_enabled(&self, enabled: bool) -> Result<ChinaDirectStatus, AppError> {
        self.finish(
            self.routing.set_china_direct_enabled(enabled),
            ErrorDomain::Routing,
            "set_china_direct_enabled",
        )
    }
    /// 协调导入配置、凭据、启动项和运行时的持久化事务。
    pub fn import(
        &self,
        json: &str,
        updates: HashMap<String, CredentialUpdate>,
    ) -> Result<(), AppError> {
        self.finish(
            self.import_transaction(json, updates),
            ErrorDomain::Application,
            "import_configuration",
        )
    }
    /// 委托对应服务操作，保持原有配置和 IPC 语义。
    pub(crate) fn upgrade_legacy_credentials(&self) -> Result<(), AppError> {
        self.context.upgrade_legacy_credentials()
    }
    /// 委托对应服务操作，保持原有配置和 IPC 语义。
    pub(crate) fn report_configuration_recovery_issue(&self, error: &AppError) {
        self.context.report_configuration_recovery_issue(error);
    }
    /// 委托对应服务操作，保持原有配置和 IPC 语义。
    pub(crate) fn start_latency_task(
        &self,
        id: &str,
    ) -> Result<crate::latency_tasks::LatencySubscription, AppError> {
        self.finish(
            self.context.start_latency_task(id),
            ErrorDomain::Proxy,
            "start_proxy_latency_task",
        )
    }
    /// 委托对应服务操作，保持原有配置和 IPC 语义。
    pub(crate) fn latency_task_snapshot(
        &self,
        subscription_id: &str,
    ) -> Result<crate::latency_tasks::LatencyTaskSnapshot, AppError> {
        self.finish(
            self.context.latency_task_snapshot(subscription_id),
            ErrorDomain::Proxy,
            "get_proxy_latency_task",
        )
    }
    /// 委托对应服务操作，保持原有配置和 IPC 语义。
    pub(crate) fn release_latency_task(&self, subscription_id: &str) -> Result<(), AppError> {
        self.finish(
            self.context.release_latency_task(subscription_id),
            ErrorDomain::Proxy,
            "release_proxy_latency_task",
        )
    }
    /// 委托对应服务操作，保持原有配置和 IPC 语义。
    pub(crate) fn test_latency_compat(
        &self,
        id: &str,
    ) -> Result<crate::latency_tasks::LatencyResult, AppError> {
        self.finish(
            self.context.test_latency_compat(id),
            ErrorDomain::Proxy,
            "test_proxy_latency",
        )
    }
}

#[cfg(test)]
#[path = "application_service_tests.rs"]
pub(crate) mod tests;
