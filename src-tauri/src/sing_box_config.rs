use crate::{
    credentials::ProxyCredential,
    error::{AppError, FieldError},
    models::{PersistedConfiguration, ProxyProtocol, RuleAction, RuleMatcher, RuntimeMode},
    routing::CompiledRules,
};
use serde_json::{json, Value};

pub struct SingBoxPorts {
    pub proxy: u16,
    pub control: u16,
}

// The returned JSON contains credentials and the per-run control secret. Callers must
// only write it to a current-user-only temporary file and remove it after shutdown.
pub fn render(
    configuration: &PersistedConfiguration,
    mode: RuntimeMode,
    ports: SingBoxPorts,
    control_secret: &str,
    credential: Option<&ProxyCredential>,
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
    let profile = configuration
        .profiles
        .iter()
        .find(|profile| Some(&profile.id) == configuration.active_profile_id.as_ref())
        .ok_or_else(|| field_error("active_profile_id", "请选择活动代理档案"))?;
    if !profile.enabled {
        return Err(field_error("active_profile_id", "活动代理档案已停用"));
    }
    if profile.authentication_enabled != credential.is_some() {
        return Err(field_error("credential", "活动代理认证凭据缺失或不一致"));
    }

    let kind = match profile.protocol {
        ProxyProtocol::Socks5 => "socks",
        ProxyProtocol::Http => "http",
    };
    let mut upstream = json!({
        "type": kind,
        "tag": "selected-proxy",
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

    let rules: Vec<Value> = if mode == RuntimeMode::Rules {
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
                        RuleAction::Proxy => "selected-proxy",
                        RuleAction::Direct => "direct",
                    }),
                );
                Value::Object(matcher)
            })
            .collect()
    } else {
        Vec::new()
    };
    let final_outbound = if mode == RuntimeMode::Global {
        "selected-proxy"
    } else {
        "direct"
    };
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
        "outbounds": [upstream, { "type": "direct", "tag": "direct" }],
        "route": { "rules": rules, "final": final_outbound },
    }))
    .map_err(|_| AppError::storage("无法生成内核配置"))
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
