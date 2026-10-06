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
    config.runtime_mode = crate::models::TEST_RULES_MODE;
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
    assert_eq!(matched.stage, RouteStage::UserRule);
    assert_eq!(matched.matched_rule_name.as_deref(), Some("Selected"));
    assert_eq!(matched.proxy_name.as_deref(), Some("Primary"));
    let fallback = evaluate(&config, None, "other.invalid", 443).unwrap();
    assert_eq!(fallback.stage, RouteStage::Final);
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
    config.runtime_mode = crate::models::RuntimeMode::Rules {
        use_china_direct: true,
        default_action: crate::models::RuleAction::Proxy,
    };
    for (target, stage, action) in [
        ("baidu.com", RouteStage::ChinaDomain, RuleAction::Direct),
        ("unknown.invalid", RouteStage::Final, RuleAction::Proxy),
        ("1.0.1.1", RouteStage::ChinaIp, RuleAction::Direct),
        ("240e::1", RouteStage::ChinaIp, RuleAction::Direct),
        ("127.0.0.1", RouteStage::PrivateIp, RuleAction::Direct),
        ("192.0.2.1", RouteStage::Final, RuleAction::Proxy),
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
        RouteStage::UserRule
    );
    let missing = tempfile::tempdir().unwrap();
    assert!(evaluate(&config, Some(missing.path()), "baidu.com", 443).is_err());
}

#[test]
fn parameter_combinations_preserve_user_china_and_default_priority() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/china-rules");
    for enabled in [false, true] {
        for action in [RuleAction::Proxy, RuleAction::Direct] {
            let mut config = config();
            config.runtime_mode = crate::models::RuntimeMode::Rules {
                use_china_direct: enabled,
                default_action: action,
            };
            config.rules.push(RoutingRule {
                id: "override".into(),
                name: "Override".into(),
                matcher: RuleMatcher::Domain,
                target: "override.cn".into(),
                port_start: None,
                port_end: None,
                action: RuleAction::Proxy,
                proxy_profile_id: Some("primary".into()),
                enabled: true,
            });
            let eval = |host| {
                evaluate_with_matcher(&config, Some(&root), host, 443, |_, _, host| {
                    Ok(host.ends_with(".cn"))
                })
                .unwrap()
            };
            assert_eq!(eval("override.cn").stage, RouteStage::UserRule);
            assert_eq!(eval("override.cn").action, RuleAction::Proxy);
            let china = eval("other.cn");
            assert_eq!(
                china.stage,
                if enabled {
                    RouteStage::ChinaDomain
                } else {
                    RouteStage::Final
                }
            );
            assert_eq!(
                china.action,
                if enabled { RuleAction::Direct } else { action }
            );
            let fallback = eval("outside.invalid");
            assert_eq!(fallback.stage, RouteStage::Final);
            assert_eq!(fallback.action, action);
            assert_eq!(
                fallback.proxy_profile_id.is_some(),
                action == RuleAction::Proxy
            );
        }
    }
}

#[test]
fn macos_can_predict_user_rules_but_never_fakes_unavailable_china_matches() {
    let mut config = config();
    config.runtime_mode = crate::models::RuntimeMode::Rules {
        use_china_direct: true,
        default_action: RuleAction::Proxy,
    };
    config.rules.push(RoutingRule {
        id: "override".into(),
        name: "Override".into(),
        matcher: RuleMatcher::Domain,
        target: "override.cn".into(),
        port_start: None,
        port_end: None,
        action: RuleAction::Direct,
        proxy_profile_id: None,
        enabled: true,
    });
    assert_eq!(
        evaluate(&config, None, "override.cn", 443).unwrap().stage,
        RouteStage::UserRule
    );
    assert!(evaluate(&config, None, "outside.invalid", 443).is_err());
}
