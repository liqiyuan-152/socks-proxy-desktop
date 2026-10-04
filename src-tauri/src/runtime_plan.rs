use crate::{
    error::{AppError, FieldError},
    models::{PersistedConfiguration, ProxyProfile, RoutingRule, RuntimeMode},
};
use std::collections::{BTreeMap, HashMap};

/// Effective, non-secret kernel inputs. Presentation and unrelated settings
/// never participate in this value's equality comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimePlan {
    pub mode: RuntimeMode,
    pub exits: Vec<ProxyProfile>,
    pub rules: Vec<RoutingRule>,
    pub default_profile_id: Option<String>,
    pub china_direct_enabled: bool,
    pub credential_versions: BTreeMap<String, String>,
}

impl RuntimePlan {
    pub(crate) fn build(
        configuration: &PersistedConfiguration,
        mode: RuntimeMode,
        versions: &HashMap<String, String>,
    ) -> Result<Self, AppError> {
        configuration.validate()?;
        if mode == RuntimeMode::Rules {
            configuration.validate_runtime_rules()?;
        }
        if mode == RuntimeMode::Global && configuration.active_profile_id.is_none() {
            return Err(AppError::validation(vec![FieldError {
                field: "default_profile_id".into(),
                message: "全局代理需要默认代理".into(),
            }]));
        }
        let preset = mode == RuntimeMode::Rules && configuration.china_direct_enabled;
        let default_profile_id = if mode == RuntimeMode::Global || preset {
            configuration.active_profile_id.clone()
        } else {
            None
        };
        let mut exits: Vec<_> = configuration
            .profiles
            .iter()
            .filter(|profile| {
                profile.enabled
                    && match mode {
                        RuntimeMode::Direct => false,
                        RuntimeMode::Rules => true,
                        RuntimeMode::Global => {
                            configuration.active_profile_id.as_ref() == Some(&profile.id)
                        }
                    }
            })
            .cloned()
            .map(|mut profile| {
                profile.name = profile.id.clone();
                if !profile.authentication_enabled {
                    profile.credential_ref = None;
                }
                profile
            })
            .collect();
        exits.sort_by(|a, b| a.id.cmp(&b.id));
        let credential_versions = exits
            .iter()
            .filter(|profile| profile.authentication_enabled)
            .map(|profile| {
                (
                    profile.id.clone(),
                    versions
                        .get(&profile.id)
                        .cloned()
                        .or_else(|| profile.credential_ref.clone())
                        .unwrap_or_default(),
                )
            })
            .collect();
        let rules = configuration
            .rules
            .iter()
            .filter(|rule| mode == RuntimeMode::Rules && rule.enabled)
            .cloned()
            .map(|mut rule| {
                rule.name = rule.id.clone();
                rule
            })
            .collect();
        Ok(Self {
            mode,
            exits,
            rules,
            default_profile_id,
            china_direct_enabled: preset,
            credential_versions,
        })
    }
}

/// Versions are random identifiers, never password hashes. Keep the old map
/// until the candidate has been durably committed.
pub(crate) fn candidate_versions(
    configuration: &PersistedConfiguration,
    previous: &HashMap<String, String>,
    changed: &[String],
) -> HashMap<String, String> {
    configuration
        .profiles
        .iter()
        .filter(|profile| profile.authentication_enabled)
        .map(|profile| {
            let version = if changed.contains(&profile.id) {
                uuid::Uuid::new_v4().to_string()
            } else {
                previous
                    .get(&profile.id)
                    .cloned()
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
            };
            (profile.id.clone(), version)
        })
        .collect()
}

#[cfg(test)]
#[path = "runtime_plan_tests.rs"]
mod tests;
