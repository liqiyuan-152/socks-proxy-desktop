use super::*;
use crate::models::{ProxyProfile, RoutingRule};

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
            None,
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
    assert_eq!(config["route"]["rules"][1]["outbound"], "selected-proxy");
    assert_eq!(config["inbounds"][0]["listen"], "127.0.0.1");
    assert_eq!(
        config["experimental"]["clash_api"]["external_controller"],
        "127.0.0.1:19090"
    );
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
        Some(&secret),
    )
    .unwrap();
    let value: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(value["route"]["rules"], json!([]));
    assert_eq!(value["route"]["final"], "selected-proxy");
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
            None
        )
        .unwrap_err()
        .fields[0]
            .field,
        "active_profile_id"
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
            None
        )
        .unwrap_err()
        .fields[0]
            .field,
        "credential"
    );
}

#[test]
fn fixed_sing_box_accepts_rendered_rule_and_global_configs_when_available() {
    let Ok(binary) = std::env::var("SING_BOX_TEST_BIN") else {
        return;
    };
    for mode in [RuntimeMode::Rules, RuntimeMode::Global] {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        let contents = render(
            &configuration(),
            mode,
            SingBoxPorts {
                proxy: 18080,
                control: 19090,
            },
            "test-only-secret",
            None,
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
