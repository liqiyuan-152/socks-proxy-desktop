//! 可共享且支持动态派发的领域服务接口。
//!
//! 只暴露业务操作，不暴露底层数据库、锁或运行时事务钩子。
//! 实现应由构造函数注入依赖，调用方可持有 `Arc<dyn Trait>`。

use super::{
    ChinaDirectStatus, ProfileCredentialView, ProfileInput, ProfileView, RouteTestResult,
    RoutingRule, RuntimeMode, RuntimeSnapshot,
};
use crate::{
    domain_errors::{ProxyError, RoutingError, RuntimeError},
    observability::ActiveConnectionsSnapshot,
};

/// 管理代理档案和隔离的认证凭据。
pub trait ProfileService: Send + Sync {
    /// 返回不含凭据的档案列表，附带一致的配置版本号。
    fn list_profiles(&self) -> Result<Vec<ProfileView>, ProxyError>;

    /// 校验并保存档案；凭据和运行时应用失败时必须回滚。
    fn save_profile(&self, input: ProfileInput) -> Result<ProfileView, ProxyError>;

    /// 删除档案；仍被默认代理或路由规则引用时必须拒绝。
    fn delete_profile(&self, id: &str) -> Result<(), ProxyError>;

    /// 仅返回指定档案的凭据，未启用认证或凭据缺失时返回错误。
    fn get_credential(&self, id: &str) -> Result<ProfileCredentialView, ProxyError>;

    /// 设置默认代理，验证其存在且启用；`None` 表示清除选择。
    fn select_profile(&self, id: Option<String>) -> Result<(), ProxyError>;
}

/// 管理有序路由规则及国内直连策略。
pub trait RoutingService: Send + Sync {
    /// 按匹配优先级返回规则。
    fn list_rules(&self) -> Result<Vec<RoutingRule>, RoutingError>;

    /// 验证规则及代理引用后，以持久化事务替换规则集。
    fn replace_rules(&self, rules: Vec<RoutingRule>) -> Result<(), RoutingError>;

    /// 调整优先级；标识必须恰好包含全部规则且无重复。
    fn reorder_rules(&self, ids: &[String]) -> Result<(), RoutingError>;

    /// 在一致的配置版本上评估路由，不发起网络请求。
    fn test_route(&self, target: &str, port: u16) -> Result<RouteTestResult, RoutingError>;

    /// 返回国内直连开关、规则集可用性和数据日期。
    fn get_china_status(&self) -> Result<ChinaDirectStatus, RoutingError>;

    /// 验证规则集和默认代理后，事务性地更新国内直连开关。
    fn set_china_direct_enabled(&self, enabled: bool) -> Result<ChinaDirectStatus, RoutingError>;
}

/// 协调运行时操作及模式持久化，与底层 `RuntimeCoordinator` 区分。
pub trait RuntimeService: Send + Sync {
    /// 读取当前运行时快照。
    fn snapshot(&self) -> RuntimeSnapshot;

    /// 返回内核可验证的活跃连接，能力不足时按现有方式降级。
    fn active_connections(&self) -> ActiveConnectionsSnapshot;

    /// 先持久化用户所选模式，再请求运行时切换。
    fn set_mode(&self, mode: RuntimeMode) -> Result<RuntimeSnapshot, RuntimeError>;

    /// 持久化直连模式并停止代理，保留原有停止语义。
    fn stop(&self) -> Result<RuntimeSnapshot, RuntimeError>;

    /// 恢复网络及未完成的配置事务；失败时保持恢复阻塞状态。
    fn recover_network(&self) -> Result<RuntimeSnapshot, RuntimeError>;
}
