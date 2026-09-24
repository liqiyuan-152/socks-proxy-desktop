use crate::{
    china_rules::ChinaRuleSets,
    credentials::ProxyCredential,
    error::{AppError, FieldError},
    models::{PersistedConfiguration, ProxyProtocol, RuleAction, RuleMatcher, RuntimeMode},
    routing::CompiledRules,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub struct SingBoxPorts {
    pub proxy: u16,
    pub control: u16,
}

// The returned JSON contains credentials and the per-run control secret. Callers must
// only write it to a current-user-only temporary file and remove it after shutdown.
#[cfg(test)]
pub fn render(
    configuration: &PersistedConfiguration,
    mode: RuntimeMode,
    ports: SingBoxPorts,
    control_secret: &str,
    credentials: &HashMap<String, ProxyCredential>,
) -> Result<String, AppError> {
    render_with_rules(
        configuration,
        mode,
        ports,
        control_secret,
        credentials,
        None,
    )
}

pub fn render_with_rules(
    configuration: &PersistedConfiguration,
    mode: RuntimeMode,
    ports: SingBoxPorts,
    control_secret: &str,
    credentials: &HashMap<String, ProxyCredential>,
    china_rules: Option<&ChinaRuleSets>,
) -> Result<String, AppError> {
    if mode == RuntimeMode::Direct {
        return Err(field_error("mode", "直连模式无需启动代理内核"));
    }
    if ports.proxy == 0 || ports.control == 0 || ports.proxy == ports.control {
        return Err(field_error("ports", "内核监听端口无效或冲突"));
    }
    if control_secret.is_empty() {
        return Err(field_error("control_secret", "控制接口密钥不能为空"));
    }
    let compiled = CompiledRules::compile(configuration)?;
    if mode == RuntimeMode::Rules {
        configuration.validate_runtime_rules()?;
    }
    let default_tag = configuration
        .active_profile_id
        .as_ref()
        .map(|id| proxy_tag(id));
    if mode == RuntimeMode::Global && default_tag.is_none() {
        return Err(field_error("default_profile_id", "全局代理需要默认代理"));
    }
    let mut outbounds = Vec::new();
    for (index, profile) in configuration
        .profiles
        .iter()
        .enumerate()
        .filter(|(_, p)| p.enabled)
    {
        let credential = credentials.get(&profile.id);
        if profile.authentication_enabled != credential.is_some() {
            return Err(field_error(
                &format!("profiles[{index}].credential"),
                "代理认证凭据缺失或不一致",
            ));
        }
        let kind = match profile.protocol {
            ProxyProtocol::Socks5 => "socks",
            ProxyProtocol::Http => "http",
        };
        let mut upstream = json!({
            "type": kind,
            "tag": proxy_tag(&profile.id),
            "server": profile.host,
            "server_port": profile.port,
        });
        if kind == "socks" {
            upstream["version"] = json!("5");
        }
        if let Some(credential) = credential {
            upstream["username"] = json!(credential.username);
            upstream["password"] = json!(credential.password);
        }
        outbounds.push(upstream);
    }
    outbounds.push(json!({ "type": "direct", "tag": "direct" }));

    let mut rules: Vec<Value> = if mode == RuntimeMode::Rules {
        compiled
            .ordered_rules()
            .iter()
            .filter(|rule| rule.enabled)
            .map(|rule| {
                let mut matcher = serde_json::Map::new();
                let key = match rule.matcher {
                    RuleMatcher::Domain => "domain",
                    RuleMatcher::DomainSuffix => "domain_suffix",
                    RuleMatcher::IpCidr => "ip_cidr",
                };
                matcher.insert(key.into(), json!([rule.target]));
                match (rule.port_start, rule.port_end) {
                    (Some(start), Some(end)) if start != end => {
                        matcher.insert("port_range".into(), json!([format!("{start}:{end}")]));
                    }
                    (Some(start), _) => {
                        matcher.insert("port".into(), json!([start]));
                    }
                    (None, Some(end)) => {
                        matcher.insert("port".into(), json!([end]));
                    }
                    (None, None) => {}
                }
                matcher.insert("action".into(), json!("route"));
                matcher.insert(
                    "outbound".into(),
                    json!(match rule.action {
                        RuleAction::Proxy => proxy_tag(
                            rule.proxy_profile_id
                                .as_deref()
                                .expect("validated proxy rule")
                        ),
                        RuleAction::Direct => "direct".into(),
                    }),
                );
                Value::Object(matcher)
            })
            .collect()
    } else {
        Vec::new()
    };
    let preset = mode == RuntimeMode::Rules && configuration.china_direct_enabled;
    if preset {
        china_rules.ok_or_else(|| AppError::unavailable("国内直连规则集未通过校验"))?;
        let default = default_tag
            .as_deref()
            .ok_or_else(|| field_error("default_profile_id", "国内直连需要默认代理"))?;
        rules.extend([
            json!({ "rule_set": ["cn-domain"], "action": "route", "outbound": "direct" }),
            // Lock all remaining domain targets to the default exit before IP rules.
            json!({ "domain_regex": [".+"], "action": "route", "outbound": default }),
            json!({
                "ip_cidr": ["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16",
                    "127.0.0.0/8", "169.254.0.0/16", "fc00::/7", "fe80::/10", "::1/128"],
                "action": "route", "outbound": "direct"
            }),
            json!({ "rule_set": ["cn-v4", "cn-v6"], "action": "route", "outbound": "direct" }),
        ]);
    }
    let final_outbound = if mode == RuntimeMode::Global || preset {
        default_tag.expect("validated global default")
    } else {
        "direct".into()
    };
    let mut route = json!({ "rules": rules, "final": final_outbound });
    if let Some(sets) = china_rules.filter(|_| preset) {
        route["rule_set"] = json!(sets.rule_sets());
    }
    serde_json::to_string(&json!({
        "log": { "level": "warn" },
        "experimental": { "clash_api": {
            "external_controller": format!("127.0.0.1:{}", ports.control),
            "secret": control_secret,
        } },
        "inbounds": [{
            "type": "mixed", "tag": "system-proxy-in",
            "listen": "127.0.0.1", "listen_port": ports.proxy,
        }],
        "outbounds": outbounds,
        "route": route,
    }))
    .map_err(|_| AppError::storage("无法生成内核配置"))
}

pub(crate) fn proxy_tag(id: &str) -> String {
    format!("proxy-{}", hex::encode(Sha256::digest(id.as_bytes())))
}

fn field_error(field: &str, message: &str) -> AppError {
    AppError::validation(vec![FieldError {
        field: field.into(),
        message: message.into(),
    }])
}

#[cfg(test)]
#[path = "sing_box_config_tests.rs"]
mod tests;
