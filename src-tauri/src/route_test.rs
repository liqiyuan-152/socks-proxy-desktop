use crate::{
    china_rules::ChinaRuleSets,
    error::{AppError, FieldError},
    models::{valid_domain, PersistedConfiguration, RuleAction},
    routing::CompiledRules,
};
use serde::Serialize;
use std::{net::IpAddr, path::Path};

#[derive(Debug, Serialize)]
pub struct RouteTestResult {
    pub stage: &'static str,
    pub action: RuleAction,
    pub proxy_profile_id: Option<String>,
    pub proxy_name: Option<String>,
    pub matched_rule_id: Option<String>,
    pub matched_rule_name: Option<String>,
    pub reason: String,
    pub data_date: Option<String>,
}

pub fn evaluate(
    configuration: &PersistedConfiguration,
    root: Option<&Path>,
    target: &str,
    port: u16,
) -> Result<RouteTestResult, AppError> {
    configuration.validate_runtime_rules()?;
    let host = target.trim();
    let host = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);
    let ip = host.parse::<IpAddr>().ok();
    if port == 0 || (ip.is_none() && !valid_domain(host)) {
        return Err(AppError::validation(vec![FieldError {
            field: if port == 0 { "port" } else { "target" }.into(),
            message: "测试目标或端口无效".into(),
        }]));
    }
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    let sets = if configuration.china_direct_enabled {
        let root = root.ok_or_else(|| AppError::unavailable("此平台未提供国内直连规则集"))?;
        Some(ChinaRuleSets::verify(root)?)
    } else {
        None
    };
    let rules = CompiledRules::compile(configuration)?;
    let decide = |stage,
                  action,
                  proxy_id: Option<&str>,
                  rule_id: Option<&str>,
                  rule_name: Option<&str>,
                  reason: &str| {
        let profile = proxy_id.and_then(|id| configuration.profiles.iter().find(|p| p.id == id));
        RouteTestResult {
            stage,
            action,
            proxy_profile_id: proxy_id.map(str::to_owned),
            proxy_name: profile.map(|p| p.name.clone()),
            matched_rule_id: rule_id.map(str::to_owned),
            matched_rule_name: rule_name.map(str::to_owned),
            reason: reason.into(),
            data_date: sets.as_ref().map(|set| set.data_date().to_owned()),
        }
    };
    if let Some(rule) = rules.matching_rule(&host, port) {
        return Ok(decide(
            "user_rule",
            rule.action,
            rule.proxy_profile_id.as_deref(),
            Some(&rule.id),
            Some(&rule.name),
            "首条匹配的用户规则",
        ));
    }
    let Some(sets) = sets.as_ref() else {
        return Ok(decide(
            "final",
            RuleAction::Direct,
            None,
            None,
            None,
            "预设关闭，未命中规则时直连",
        ));
    };
    if let Some(ip) = ip {
        if is_private(ip) {
            return Ok(decide(
                "private_ip",
                RuleAction::Direct,
                None,
                None,
                None,
                "字面私有或本地 IP",
            ));
        }
        let name = if ip.is_ipv4() {
            "china-ipv4.srs"
        } else {
            "china-ipv6.srs"
        };
        if sets.matches(name, &host)? {
            return Ok(decide(
                "china_ip",
                RuleAction::Direct,
                None,
                None,
                None,
                "字面 IP 命中中国地址集",
            ));
        }
    } else if sets.matches("china-domains.srs", &host)? {
        return Ok(decide(
            "china_domain",
            RuleAction::Direct,
            None,
            None,
            None,
            "域名命中中国域名集",
        ));
    }
    let reason = if ip.is_some() {
        "字面 IP 未命中私有或中国地址集"
    } else {
        "域名集外不按解析 IP 判断"
    };
    Ok(decide(
        "final",
        RuleAction::Proxy,
        configuration.active_profile_id.as_deref(),
        None,
        None,
        reason,
    ))
}

fn is_private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        IpAddr::V6(ip) => {
            let prefix = ip.segments()[0];
            prefix & 0xfe00 == 0xfc00 || ip.is_loopback() || prefix & 0xffc0 == 0xfe80
        }
    }
}

#[cfg(test)]
#[path = "route_test_tests.rs"]
mod tests;
