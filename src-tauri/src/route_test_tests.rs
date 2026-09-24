use super::*;
use crate::models::{ProxyProfile, ProxyProtocol, RoutingRule, RuleMatcher};

fn config() -> PersistedConfiguration {
    let mut config = PersistedConfiguration::default();
    config.profiles.push(ProxyProfile {
        id: "primary".into(),
        name: "Primary".into(),
        protocol: ProxyProtocol::Socks5,
        host: "proxy.example.org".into(),
        port: 1080,
        authentication_enabled: false,
        credential_ref: None,
        enabled: true,
    });
    config.active_profile_id = Some("primary".into());
    config
}

#[test]
fn user_rule_takes_precedence_and_disabled_preset_keeps_direct_fallback() {
    let mut config = config();
    config.rules.push(RoutingRule {
        id: "first".into(),
        name: "Selected".into(),
        matcher: RuleMatcher::DomainSuffix,
        target: "example.org".into(),
        port_start: Some(443),
        port_end: None,
        action: RuleAction::Proxy,
        proxy_profile_id: Some("primary".into()),
        enabled: true,
    });
    let matched = evaluate(&config, None, "api.example.org", 443).unwrap();
    assert_eq!(matched.stage, "user_rule");
    assert_eq!(matched.matched_rule_name.as_deref(), Some("Selected"));
    assert_eq!(matched.proxy_name.as_deref(), Some("Primary"));
    let fallback = evaluate(&config, None, "other.invalid", 443).unwrap();
    assert_eq!(fallback.stage, "final");
    assert_eq!(fallback.action, RuleAction::Direct);
    assert_eq!(
        evaluate(&config, None, "bad host", 443).unwrap_err().fields[0].field,
        "target"
    );
    assert_eq!(
        evaluate(&config, None, "example.org", 0)
            .unwrap_err()
            .fields[0]
            .field,
        "port"
    );
}

#[cfg(windows)]
#[test]
fn bundled_rule_sets_explain_domains_and_literal_ipv4_ipv6() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/china-rules");
    let mut config = config();
    config.china_direct_enabled = true;
    for (target, stage, action) in [
        ("baidu.com", "china_domain", RuleAction::Direct),
        ("unknown.invalid", "final", RuleAction::Proxy),
        ("1.0.1.1", "china_ip", RuleAction::Direct),
        ("240e::1", "china_ip", RuleAction::Direct),
        ("127.0.0.1", "private_ip", RuleAction::Direct),
        ("192.0.2.1", "final", RuleAction::Proxy),
    ] {
        let result = evaluate(&config, Some(&root), target, 443).unwrap();
        assert_eq!(result.stage, stage, "{target}");
        assert_eq!(result.action, action, "{target}");
        assert_eq!(result.data_date.as_deref(), Some("2026-09-23"));
    }
    config.rules.push(RoutingRule {
        id: "override".into(),
        name: "Override".into(),
        matcher: RuleMatcher::Domain,
        target: "baidu.com".into(),
        port_start: None,
        port_end: None,
        action: RuleAction::Proxy,
        proxy_profile_id: Some("primary".into()),
        enabled: true,
    });
    assert_eq!(
        evaluate(&config, Some(&root), "baidu.com", 443)
            .unwrap()
            .stage,
        "user_rule"
    );
    let missing = tempfile::tempdir().unwrap();
    assert!(evaluate(&config, Some(missing.path()), "baidu.com", 443).is_err());
}
