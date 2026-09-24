use crate::error::{AppError, FieldError};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

pub const CONFIG_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyProtocol {
    Socks5,
    Http,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeMode {
    Rules,
    Global,
    Direct,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProxyProfile {
    pub id: String,
    pub name: String,
    pub protocol: ProxyProtocol,
    pub host: String,
    pub port: u16,
    pub authentication_enabled: bool,
    pub credential_ref: Option<String>,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleMatcher {
    Domain,
    DomainSuffix,
    IpCidr,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleAction {
    Proxy,
    Direct,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoutingRule {
    pub id: String,
    pub name: String,
    pub matcher: RuleMatcher,
    pub target: String,
    pub port_start: Option<u16>,
    pub port_end: Option<u16>,
    pub action: RuleAction,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionPolicy {
    Days7,
    Days30,
    Days90,
    Permanent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppSettings {
    pub launch_at_login: bool,
    pub diagnostic_retention: RetentionPolicy,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            launch_at_login: false,
            diagnostic_retention: RetentionPolicy::Days30,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PersistedConfiguration {
    pub schema_version: u32,
    pub profiles: Vec<ProxyProfile>,
    pub rules: Vec<RoutingRule>,
    pub active_profile_id: Option<String>,
    pub settings: AppSettings,
}

impl Default for PersistedConfiguration {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            profiles: Vec::new(),
            rules: Vec::new(),
            active_profile_id: None,
            settings: AppSettings::default(),
        }
    }
}

impl PersistedConfiguration {
    pub fn validate(&self) -> Result<(), AppError> {
        let mut errors = Vec::new();
        if self.schema_version != CONFIG_SCHEMA_VERSION {
            errors.push(field_error("schema_version", "不支持的配置版本"));
        }

        let mut profile_ids = std::collections::HashSet::new();
        let mut profile_names = std::collections::HashSet::new();
        for (index, profile) in self.profiles.iter().enumerate() {
            let prefix = format!("profiles[{index}]");
            if profile.id.trim().is_empty() || !profile_ids.insert(profile.id.as_str()) {
                errors.push(field_error(
                    format!("{prefix}.id"),
                    "标识不能为空且必须唯一",
                ));
            }
            if profile.name.trim().is_empty() {
                errors.push(field_error(format!("{prefix}.name"), "名称不能为空"));
            } else if !profile_names.insert(profile.name.trim().to_lowercase()) {
                errors.push(field_error(format!("{prefix}.name"), "代理名称不能重复"));
            }
            if profile.host.trim().is_empty() {
                errors.push(field_error(format!("{prefix}.host"), "服务器不能为空"));
            }
            if profile.port == 0 {
                errors.push(field_error(
                    format!("{prefix}.port"),
                    "端口必须在 1 到 65535 之间",
                ));
            }
            if profile.authentication_enabled != profile.credential_ref.is_some() {
                errors.push(field_error(
                    format!("{prefix}.authentication_enabled"),
                    "认证状态与凭据引用不一致",
                ));
            }
        }

        let mut rule_ids = std::collections::HashSet::new();
        let mut rule_names = std::collections::HashSet::new();
        for (index, rule) in self.rules.iter().enumerate() {
            let prefix = format!("rules[{index}]");
            if rule.id.trim().is_empty() || !rule_ids.insert(rule.id.as_str()) {
                errors.push(field_error(
                    format!("{prefix}.id"),
                    "标识不能为空且必须唯一",
                ));
            }
            if rule.name.trim().is_empty() {
                errors.push(field_error(format!("{prefix}.name"), "名称不能为空"));
            } else if !rule_names.insert(rule.name.trim().to_lowercase()) {
                errors.push(field_error(format!("{prefix}.name"), "规则名称不能重复"));
            }
            if !valid_rule_target(rule.matcher, rule.target.trim()) {
                errors.push(field_error(
                    format!("{prefix}.target"),
                    "目标格式与匹配类型不符",
                ));
            }
            if rule.port_start == Some(0) || rule.port_end == Some(0) {
                errors.push(field_error(
                    format!("{prefix}.port"),
                    "端口必须在 1 到 65535 之间",
                ));
            }
            if let (Some(start), Some(end)) = (rule.port_start, rule.port_end) {
                if start > end {
                    errors.push(field_error(
                        format!("{prefix}.port"),
                        "端口范围起始值不能大于结束值",
                    ));
                }
            }
        }

        if let Some(active_id) = &self.active_profile_id {
            if !self
                .profiles
                .iter()
                .any(|profile| profile.id == *active_id && profile.enabled)
            {
                errors.push(field_error(
                    "active_profile_id",
                    "活动代理必须指向已启用的档案",
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(AppError::validation(errors))
        }
    }
}

fn field_error(field: impl Into<String>, message: impl Into<String>) -> FieldError {
    FieldError {
        field: field.into(),
        message: message.into(),
    }
}

fn valid_rule_target(matcher: RuleMatcher, target: &str) -> bool {
    match matcher {
        RuleMatcher::Domain | RuleMatcher::DomainSuffix => valid_domain(target),
        RuleMatcher::IpCidr => valid_cidr(target),
    }
}

fn valid_domain(domain: &str) -> bool {
    let domain = domain.strip_suffix('.').unwrap_or(domain);
    !domain.is_empty()
        && domain.len() <= 253
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label.as_bytes()[0].is_ascii_alphanumeric()
                && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

fn valid_cidr(value: &str) -> bool {
    let Some((address, prefix)) = value.split_once('/') else {
        return false;
    };
    let Ok(address) = address.parse::<IpAddr>() else {
        return false;
    };
    let Ok(prefix) = prefix.parse::<u8>() else {
        return false;
    };
    prefix <= if address.is_ipv4() { 32 } else { 128 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: &str, name: &str, port: u16) -> ProxyProfile {
        ProxyProfile {
            id: id.into(),
            name: name.into(),
            protocol: ProxyProtocol::Socks5,
            host: "127.0.0.1".into(),
            port,
            authentication_enabled: false,
            credential_ref: None,
            enabled: true,
        }
    }

    fn rule(matcher: RuleMatcher, target: &str) -> RoutingRule {
        RoutingRule {
            id: "rule-id".into(),
            name: "rule".into(),
            matcher,
            target: target.into(),
            port_start: None,
            port_end: None,
            action: RuleAction::Proxy,
            enabled: true,
        }
    }

    #[test]
    fn accepts_stable_profile_ids_and_valid_configuration() {
        let mut config = PersistedConfiguration::default();
        config.profiles.push(profile("stable-id", "Primary", 1080));
        config.active_profile_id = Some("stable-id".into());
        config
            .rules
            .push(rule(RuleMatcher::DomainSuffix, "example.com"));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn rejects_duplicate_profile_names_and_invalid_ports() {
        let mut config = PersistedConfiguration::default();
        config.profiles.push(profile("one", "Primary", 0));
        config.profiles.push(profile("two", "primary", 1080));
        let error = config.validate().unwrap_err();
        assert!(error
            .fields
            .iter()
            .any(|item| item.field == "profiles[0].port"));
        assert!(error
            .fields
            .iter()
            .any(|item| item.field == "profiles[1].name"));
    }

    #[test]
    fn validates_domain_and_cidr_targets_by_matcher() {
        assert!(valid_rule_target(RuleMatcher::Domain, "api.example.com"));
        assert!(!valid_rule_target(
            RuleMatcher::Domain,
            "https://example.com"
        ));
        assert!(valid_rule_target(RuleMatcher::IpCidr, "192.168.1.0/24"));
        assert!(!valid_rule_target(RuleMatcher::IpCidr, "192.168.1.0/44"));
    }
}
