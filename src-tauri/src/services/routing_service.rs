//! 管理有序规则、国内直连规则集和离线路由评估。
use super::{
    context::ConfigurationContext, interfaces::RoutingService as RoutingServiceInterface,
    routing_types::ChinaDirectStatus,
};
use crate::{
    china_rules::ChinaRuleSets,
    domain_errors::RoutingError,
    error::{AppError, FieldError},
    models::RoutingRule,
    route_test::{self, RouteTestResult},
    routing::CompiledRules,
};
use std::sync::Arc;

/// 路由领域服务；配置写入复用应用共享的事务边界。
pub struct RoutingService {
    context: Arc<ConfigurationContext>,
}
impl RoutingService {
    pub(crate) fn new(context: Arc<ConfigurationContext>) -> Self {
        Self { context }
    }
}
impl RoutingServiceInterface for RoutingService {
    #[tracing::instrument(skip_all, level = "debug")]
    fn list_rules(&self) -> Result<Vec<RoutingRule>, RoutingError> {
        Ok(self.context.store.load()?.rules)
    }
    #[tracing::instrument(skip_all, level = "debug")]
    fn replace_rules(&self, rules: Vec<RoutingRule>) -> Result<(), RoutingError> {
        let _guard = self.context.mutation_lock()?;
        let current = self.context.store.load()?;
        let mut candidate = current.clone();
        candidate.rules = rules;
        candidate.legacy_unresolved_rule_ids.retain(|id| {
            candidate.rules.iter().any(|rule| {
                rule.id == *id
                    && rule.action == crate::models::RuleAction::Proxy
                    && rule.proxy_profile_id.is_none()
            })
        });
        CompiledRules::compile(&candidate)?;
        self.context
            .commit(&current, &candidate)
            .map_err(Into::into)
    }
    #[tracing::instrument(skip_all, level = "debug")]
    fn reorder_rules(&self, ids: &[String]) -> Result<(), RoutingError> {
        let _guard = self.context.mutation_lock()?;
        let current = self.context.store.load()?;
        if ids.len() != current.rules.len() {
            return Err(field_error("rule_ids", "排序必须包含全部规则标识"));
        }
        let mut remaining = current.rules.clone();
        let mut sorted = Vec::with_capacity(ids.len());
        for id in ids {
            let Some(index) = remaining.iter().position(|rule| &rule.id == id) else {
                return Err(field_error("rule_ids", "排序包含重复或不存在的规则标识"));
            };
            sorted.push(remaining.remove(index));
        }
        let mut candidate = current.clone();
        candidate.rules = sorted;
        CompiledRules::compile(&candidate)?;
        self.context
            .commit(&current, &candidate)
            .map_err(Into::into)
    }
    #[tracing::instrument(skip_all, level = "debug")]
    fn get_china_status(&self) -> Result<ChinaDirectStatus, RoutingError> {
        let configuration = self.context.store.load()?;
        let root = self
            .context
            .china_rule_root
            .lock()
            .map_err(|_| AppError::storage("规则集路径锁不可用"))?;
        let sets = root
            .as_deref()
            .and_then(|root| ChinaRuleSets::verify(root).ok());
        Ok(ChinaDirectStatus {
            enabled: configuration.china_direct_enabled,
            available: sets.is_some(),
            data_date: sets.map(|sets| sets.data_date().to_owned()),
        })
    }
    #[tracing::instrument(skip_all, level = "debug")]
    fn test_route(&self, target: &str, port: u16) -> Result<RouteTestResult, RoutingError> {
        let _guard = self.context.lock()?;
        let mut result = route_test::evaluate(
            &self.context.store.load()?,
            self.context
                .china_rule_root
                .lock()
                .map_err(|_| AppError::storage("规则集路径锁不可用"))?
                .as_deref(),
            target,
            port,
        )
        .map_err(|error| {
            if error.code == "validation_error"
                && error
                    .fields
                    .iter()
                    .any(|field| matches!(field.field.as_str(), "target" | "port"))
            {
                RoutingError::InvalidTarget(crate::domain_errors::ValidationError {
                    fields: error.fields,
                })
            } else {
                error.into()
            }
        })?;
        result.configuration_revision = self.context.store.recovery_revision()?;
        Ok(result)
    }
    #[tracing::instrument(skip_all, level = "debug")]
    fn set_china_direct_enabled(&self, enabled: bool) -> Result<ChinaDirectStatus, RoutingError> {
        let _guard = self.context.mutation_lock()?;
        let current = self.context.store.load()?;
        let mut candidate = current.clone();
        candidate.china_direct_enabled = enabled;
        candidate.validate()?;
        if enabled {
            let root = self
                .context
                .china_rule_root
                .lock()
                .map_err(|_| AppError::storage("规则集路径锁不可用"))?;
            let root = root.as_deref().ok_or(RoutingError::ChinaRulesUnavailable)?;
            ChinaRuleSets::verify(root).map_err(|_| RoutingError::ChinaRulesUnavailable)?;
        }
        self.context.commit(&current, &candidate)?;
        self.get_china_status()
    }
}

fn field_error(field: &str, message: &str) -> RoutingError {
    AppError::validation(vec![FieldError {
        field: field.into(),
        message: message.into(),
    }])
    .into()
}

#[cfg(test)]
#[path = "routing_service_tests.rs"]
mod tests;
