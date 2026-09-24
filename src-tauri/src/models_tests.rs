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
        proxy_profile_id: None,
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
    config.rules[0].proxy_profile_id = Some("stable-id".into());
    assert!(config.validate().is_ok());
}

#[test]
fn validates_latency_test_url_and_reads_old_settings() {
    let settings: AppSettings =
        serde_json::from_str(r#"{"launch_at_login":false,"diagnostic_retention":"days30"}"#)
            .unwrap();
    assert_eq!(settings.latency_test_url, DEFAULT_LATENCY_TEST_URL);
    for url in [
        "http://example.org",
        "https://127.0.0.1/",
        "https://localhost/",
        "https://foo.local/",
        "https://user:pass@example.org/",
        "https://example.org/#x",
    ] {
        let mut config = PersistedConfiguration::default();
        config.settings.latency_test_url = url.into();
        assert!(config.validate().is_err(), "accepted {url}");
    }
    let mut config = PersistedConfiguration::default();
    config.settings.latency_test_url = "https://example.org:8443/check".into();
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
