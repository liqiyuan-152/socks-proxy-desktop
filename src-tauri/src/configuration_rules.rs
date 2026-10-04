use super::{field_error, ConfigurationService};
use crate::{error::AppError, models::RoutingRule, routing::CompiledRules};

impl ConfigurationService {
    pub fn replace_rules(&self, rules: Vec<RoutingRule>) -> Result<(), AppError> {
        let _guard = self.mutation_lock()?;
        let current = self.store.load()?;
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
        self.commit(&current, &candidate)
    }

    pub fn reorder_rules(&self, ids: &[String]) -> Result<(), AppError> {
        let _guard = self.mutation_lock()?;
        let current = self.store.load()?;
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
        self.commit(&current, &candidate)
    }
}
