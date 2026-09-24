use super::*;
use crate::models::{ProxyProfile, RoutingRule};
use std::collections::HashMap;

fn configuration() -> PersistedConfiguration {
    let mut config = PersistedConfiguration::default();
    config.profiles.push(ProxyProfile {
        id: "proxy-1".into(),
        name: "Primary".into(),
        protocol: ProxyProtocol::Socks5,
        host: "proxy.example.com".into(),
        port: 1080,
        authentication_enabled: false,
        credential_ref: None,
        enabled: true,
    });
    config.active_profile_id = Some("proxy-1".into());
    config.rules = vec![
        RoutingRule {
            id: "first".into(),
            name: "First".into(),
            matcher: RuleMatcher::DomainSuffix,
            target: "example.com".into(),
            port_start: Some(8000),
            port_end: Some(9000),
            action: RuleAction::Direct,
            proxy_profile_id: None,
            enabled: true,
        },
        RoutingRule {
            id: "second".into(),
            name: "Second".into(),
            matcher: RuleMatcher::Domain,
            target: "api.example.com".into(),
            port_start: None,
            port_end: None,
            action: RuleAction::Proxy,
            proxy_profile_id: Some("proxy-1".into()),
            enabled: true,
        },
    ];
    config
}

fn rendered(config: &PersistedConfiguration, mode: RuntimeMode) -> Value {
    serde_json::from_str(
        &render(
            config,
            mode,
            SingBoxPorts {
                proxy: 18080,
                control: 19090,
            },
            "test-only-secret",
            &HashMap::new(),
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn rules_preserve_first_match_and_default_to_direct() {
    let config = rendered(&configuration(), RuntimeMode::Rules);
    assert_eq!(config["route"]["final"], "direct");
    assert_eq!(
        config["route"]["rules"][0]["domain_suffix"][0],
        "example.com"
    );
    assert_eq!(config["route"]["rules"][0]["port_range"][0], "8000:9000");
    assert_eq!(config["route"]["rules"][0]["outbound"], "direct");
    assert_eq!(
        config["route"]["rules"][1]["outbound"],
        proxy_tag("proxy-1")
    );
    assert_eq!(config["inbounds"][0]["listen"], "127.0.0.1");
    assert_eq!(
        config["experimental"]["clash_api"]["external_controller"],
        "127.0.0.1:19090"
    );
}

#[test]
fn enabled_exits_keep_stable_distinct_tags_and_per_profile_secrets() {
    let mut config = configuration();
    config.profiles.push(ProxyProfile {
        id: "selected-proxy".into(),
        name: "Second".into(),
        protocol: ProxyProtocol::Http,
        host: "second.example.com".into(),
        port: 8080,
        authentication_enabled: true,
        credential_ref: Some("selected-proxy".into()),
        enabled: true,
    });
    config.profiles.push(ProxyProfile {
        id: "disabled".into(),
        name: "Disabled".into(),
        protocol: ProxyProtocol::Http,
        host: "disabled.example.com".into(),
        port: 8080,
        authentication_enabled: true,
        credential_ref: Some("disabled".into()),
        enabled: false,
    });
    config.rules.push(RoutingRule {
        id: "third".into(),
        name: "Third".into(),
        matcher: RuleMatcher::Domain,
        target: "other.example.com".into(),
        port_start: None,
        port_end: None,
        action: RuleAction::Proxy,
        proxy_profile_id: Some("selected-proxy".into()),
        enabled: true,
    });
    let credentials = HashMap::from([(
        "selected-proxy".into(),
        ProxyCredential {
            username: "bob".into(),
            password: "private-second".into(),
        },
    )]);
    let raw = render(
        &config,
        RuntimeMode::Rules,
        SingBoxPorts {
            proxy: 18080,
            control: 19090,
        },
        "secret",
        &credentials,
    )
    .unwrap();
    let value: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(value["outbounds"].as_array().unwrap().len(), 3);
    assert_eq!(value["outbounds"][0]["tag"], proxy_tag("proxy-1"));
    assert_eq!(value["outbounds"][1]["tag"], proxy_tag("selected-proxy"));
    assert_eq!(value["outbounds"][1]["password"], "private-second");
    assert!(value["outbounds"][0].get("password").is_none());
    assert_eq!(value["route"]["rules"][1]["outbound"], proxy_tag("proxy-1"));
    assert_eq!(
        value["route"]["rules"][2]["outbound"],
        proxy_tag("selected-proxy")
    );
    assert_eq!(value["route"]["final"], "direct");
    assert_eq!(
        rendered(&config_with_no_default(), RuntimeMode::Rules)["route"]["final"],
        "direct"
    );
    assert_eq!(
        render(
            &config_with_no_default(),
            RuntimeMode::Global,
            SingBoxPorts {
                proxy: 18080,
                control: 19090
            },
            "secret",
            &HashMap::new()
        )
        .unwrap_err()
        .fields[0]
            .field,
        "default_profile_id"
    );
    assert!(!serde_json::to_string(&config)
        .unwrap()
        .contains("private-second"));
}

fn config_with_no_default() -> PersistedConfiguration {
    let mut config = configuration();
    config.active_profile_id = None;
    config.rules.clear();
    config
}

#[test]
fn missing_credential_for_any_enabled_exit_rejects_config() {
    let mut config = configuration();
    config.profiles.push(ProxyProfile {
        id: "second".into(),
        name: "Second".into(),
        protocol: ProxyProtocol::Http,
        host: "second.example.com".into(),
        port: 8080,
        authentication_enabled: true,
        credential_ref: Some("second".into()),
        enabled: true,
    });
    let error = render(
        &config,
        RuntimeMode::Global,
        SingBoxPorts {
            proxy: 18080,
            control: 19090,
        },
        "secret",
        &HashMap::new(),
    )
    .unwrap_err();
    assert_eq!(error.fields[0].field, "profiles[1].credential");
}

#[test]
fn global_ignores_rules_and_authentication_is_only_in_kernel_config() {
    let mut config = configuration();
    config.profiles[0].authentication_enabled = true;
    config.profiles[0].credential_ref = Some("proxy-1".into());
    let secret = ProxyCredential {
        username: "alice".into(),
        password: "private-secret".into(),
    };
    let raw = render(
        &config,
        RuntimeMode::Global,
        SingBoxPorts {
            proxy: 18080,
            control: 19090,
        },
        "test-only-secret",
        &HashMap::from([("proxy-1".into(), secret)]),
    )
    .unwrap();
    let value: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(value["route"]["rules"], json!([]));
    assert_eq!(value["route"]["final"], proxy_tag("proxy-1"));
    assert_eq!(value["outbounds"][0]["password"], "private-secret");
    assert!(!serde_json::to_string(&config)
        .unwrap()
        .contains("private-secret"));
}

#[test]
fn missing_profile_or_credentials_never_produces_config() {
    let mut config = configuration();
    config.profiles[0].enabled = false;
    assert_eq!(
        render(
            &config,
            RuntimeMode::Rules,
            SingBoxPorts {
                proxy: 18080,
                control: 19090
            },
            "test-only-secret",
            &HashMap::new()
        )
        .unwrap_err()
        .fields[0]
            .field,
        "rules[1].proxy_profile_id"
    );
    config.profiles[0].enabled = true;
    config.profiles[0].authentication_enabled = true;
    config.profiles[0].credential_ref = Some("proxy-1".into());
    assert_eq!(
        render(
            &config,
            RuntimeMode::Rules,
            SingBoxPorts {
                proxy: 18080,
                control: 19090
            },
            "test-only-secret",
            &HashMap::new()
        )
        .unwrap_err()
        .fields[0]
            .field,
        "profiles[0].credential"
    );
}

#[test]
fn fixed_sing_box_accepts_rendered_rule_and_global_configs_when_available() {
    let Ok(binary) = std::env::var("SING_BOX_TEST_BIN") else {
        return;
    };
    let mut config = configuration();
    config.profiles.push(ProxyProfile {
        id: "proxy-2".into(),
        name: "Second".into(),
        protocol: ProxyProtocol::Http,
        host: "second.example.com".into(),
        port: 8080,
        authentication_enabled: false,
        credential_ref: None,
        enabled: true,
    });
    config.rules.push(RoutingRule {
        id: "third".into(),
        name: "Third".into(),
        matcher: RuleMatcher::Domain,
        target: "second.example.com".into(),
        port_start: None,
        port_end: None,
        action: RuleAction::Proxy,
        proxy_profile_id: Some("proxy-2".into()),
        enabled: true,
    });
    for mode in [RuntimeMode::Rules, RuntimeMode::Global] {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        let contents = render(
            &config,
            mode,
            SingBoxPorts {
                proxy: 18080,
                control: 19090,
            },
            "test-only-secret",
            &HashMap::new(),
        )
        .unwrap();
        std::io::Write::write_all(&mut file, contents.as_bytes()).unwrap();
        let output = std::process::Command::new(&binary)
            .arg("check")
            .arg("-c")
            .arg(file.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn china_preset_keeps_domains_before_literal_ip_rules() {
    let mut configuration = configuration();
    configuration.china_direct_enabled = true;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/china-rules");
    let sets = ChinaRuleSets::verify(&root).unwrap();
    let raw = render_with_rules(
        &configuration,
        RuntimeMode::Rules,
        SingBoxPorts {
            proxy: 18080,
            control: 19090,
        },
        "test-secret",
        &HashMap::new(),
        Some(&sets),
    )
    .unwrap();
    let value: Value = serde_json::from_str(&raw).unwrap();
    let rules = value["route"]["rules"].as_array().unwrap();
    assert_eq!(rules[2]["rule_set"], json!(["cn-domain"]));
    assert_eq!(rules[3]["domain_regex"], json!([".+"]));
    assert_eq!(rules[3]["outbound"], proxy_tag("proxy-1"));
    assert_eq!(rules[4]["ip_cidr"][0], "10.0.0.0/8");
    assert_eq!(rules[4]["ip_cidr"][7], "::1/128");
    assert_eq!(rules[5]["rule_set"], json!(["cn-v4", "cn-v6"]));
    assert_eq!(value["route"]["rule_set"].as_array().unwrap().len(), 3);
    assert_eq!(value["route"]["final"], proxy_tag("proxy-1"));
    assert!(render(
        &configuration,
        RuntimeMode::Rules,
        SingBoxPorts {
            proxy: 18080,
            control: 19090
        },
        "test-secret",
        &HashMap::new()
    )
    .is_err());
    if let Ok(binary) = std::env::var("SING_BOX_TEST_BIN") {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut file, raw.as_bytes()).unwrap();
        let output = std::process::Command::new(binary)
            .arg("check")
            .arg("-c")
            .arg(file.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
