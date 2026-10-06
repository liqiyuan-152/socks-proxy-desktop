use super::*;

impl PersistedConfiguration {
    pub fn china_direct_enabled(&self) -> bool {
        matches!(
            self.runtime_mode,
            RuntimeMode::Rules {
                use_china_direct: true,
                ..
            }
        )
    }

    pub fn migrate_v1(&mut self) -> Result<(), AppError> {
        if self.schema_version == 1 {
            if self.active_profile_id.as_ref().is_some_and(|id| {
                !self
                    .profiles
                    .iter()
                    .any(|profile| profile.id == *id && profile.enabled)
            }) {
                self.active_profile_id = None;
            }
            for rule in &mut self.rules {
                if rule.action == RuleAction::Proxy {
                    rule.proxy_profile_id = self.active_profile_id.clone();
                    if rule.proxy_profile_id.is_none() {
                        self.legacy_unresolved_rule_ids.push(rule.id.clone());
                    }
                }
            }
            self.schema_version = CONFIG_SCHEMA_VERSION;
        }
        if self.schema_version == 2 {
            self.schema_version = CONFIG_SCHEMA_VERSION;
        }
        self.validate()
    }
}
