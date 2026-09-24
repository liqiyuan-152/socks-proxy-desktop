use crate::error::{AppError, FieldError};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

pub const CONFIG_SCHEMA_VERSION: u32 = 2;
pub const DEFAULT_LATENCY_TEST_URL: &str = "https://www.gstatic.com/generate_204";

pub fn default_latency_test_url() -> String {
    DEFAULT_LATENCY_TEST_URL.into()
}

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
#[serde(deny_unknown_fields)]
pub struct RoutingRule {
    pub id: String,
    pub name: String,
    pub matcher: RuleMatcher,
    pub target: String,
    pub port_start: Option<u16>,
    pub port_end: Option<u16>,
    pub action: RuleAction,
    #[serde(default)]
    pub proxy_profile_id: Option<String>,
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
    #[serde(default = "default_latency_test_url")]
    pub latency_test_url: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            launch_at_login: false,
            diagnostic_retention: RetentionPolicy::Days30,
            latency_test_url: default_latency_test_url(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PersistedConfiguration {
    pub schema_version: u32,
    pub profiles: Vec<ProxyProfile>,
    pub rules: Vec<RoutingRule>,
    #[serde(rename = "default_profile_id", alias = "active_profile_id")]
    pub active_profile_id: Option<String>,
    #[serde(default)]
    pub china_direct_enabled: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub legacy_unresolved_rule_ids: Vec<String>,
    pub settings: AppSettings,
}

impl Default for PersistedConfiguration {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            profiles: Vec::new(),
            rules: Vec::new(),
            active_profile_id: None,
            china_direct_enabled: false,
            legacy_unresolved_rule_ids: Vec::new(),
            settings: AppSettings::default(),
        }
    }
}

impl PersistedConfiguration {
    pub fn validate_runtime_rules(&self) -> Result<(), AppError> {
        self.validate()?;
        if let Some((index, _)) = self.rules.iter().enumerate().find(|(_, rule)| {
            rule.enabled && rule.action == RuleAction::Proxy && rule.proxy_profile_id.is_none()
        }) {
            return Err(AppError::validation(vec![field_error(
                format!("rules[{index}].proxy_profile_id"),
                "旧代理规则需要选择出口后才能启动规则模式",
            )]));
        }
        Ok(())
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
            self.china_direct_enabled = false;
            self.schema_version = CONFIG_SCHEMA_VERSION;
        }
        self.validate()
    }

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
        let legacy_ids: std::collections::HashSet<_> =
            self.legacy_unresolved_rule_ids.iter().collect();
        if legacy_ids.len() != self.legacy_unresolved_rule_ids.len() {
            errors.push(field_error(
                "legacy_unresolved_rule_ids",
                "待修复规则标识重复",
            ));
        }
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
            match (rule.action, rule.proxy_profile_id.as_deref()) {
                (RuleAction::Direct, Some(_)) => errors.push(field_error(
                    format!("{prefix}.proxy_profile_id"),
                    "直连规则不能引用代理",
                )),
                (RuleAction::Proxy, Some(id))
                    if !self.profiles.iter().any(|p| p.id == id && p.enabled) =>
                {
                    errors.push(field_error(
                        format!("{prefix}.proxy_profile_id"),
                        "规则引用的代理不存在或已停用",
                    ));
                }
                (RuleAction::Proxy, None) if !legacy_ids.contains(&rule.id) => {
                    errors.push(field_error(
                        format!("{prefix}.proxy_profile_id"),
                        "代理规则必须选择出口",
                    ));
                }
                _ => {}
            }
        }
        if self.legacy_unresolved_rule_ids.iter().any(|id| {
            !self.rules.iter().any(|rule| {
                rule.id == *id
                    && rule.action == RuleAction::Proxy
                    && rule.proxy_profile_id.is_none()
            })
        }) {
            errors.push(field_error(
                "legacy_unresolved_rule_ids",
                "待修复规则标识无效",
            ));
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
        if self.china_direct_enabled && self.active_profile_id.is_none() {
            errors.push(field_error("default_profile_id", "国内直连需要默认代理"));
        }

        if !valid_latency_test_url(&self.settings.latency_test_url) {
            errors.push(field_error(
                "settings.latency_test_url",
                "测试地址必须是公网域名的 HTTPS URL，且不能包含凭据",
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(AppError::validation(errors))
        }
    }
}

pub fn valid_latency_test_url(input: &str) -> bool {
    if input.len() > 2048 || input.trim() != input {
        return false;
    }
    let Ok(url) = url::Url::parse(input) else {
        return false;
    };
    let Some(url::Host::Domain(host)) = url.host() else {
        return false;
    };
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none()
        && host != "localhost"
        && !host.ends_with(".localhost")
        && host != "local"
        && !host.ends_with(".local")
        && host.contains('.')
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

pub(crate) fn valid_domain(domain: &str) -> bool {
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
#[path = "models_tests.rs"]
mod tests;
