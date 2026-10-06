use super::*;
use crate::models::{ProxyProtocol, RoutingRule, RuleAction, RuleMatcher};

fn configuration() -> PersistedConfiguration {
    let profiles = ["primary", "secondary"]
        .into_iter()
        .map(|id| ProxyProfile {
            id: id.into(),
            name: id.into(),
            protocol: ProxyProtocol::Socks5,
            host: format!("{id}.example.com"),
            port: 1080,
            authentication_enabled: true,
            credential_ref: Some(id.into()),
            enabled: true,
        })
        .collect();
    let rules = ["a", "b"]
        .into_iter()
        .map(|id| RoutingRule {
            id: id.into(),
            name: id.into(),
            matcher: RuleMatcher::Domain,
            target: format!("{id}.example.com"),
            port_start: None,
            port_end: None,
            action: RuleAction::Proxy,
            proxy_profile_id: Some("secondary".into()),
            enabled: true,
        })
        .collect();
    PersistedConfiguration {
        profiles,
        rules,
        active_profile_id: Some("primary".into()),
        ..Default::default()
    }
}

#[test]
fn ignores_presentation_settings_profile_order_and_disabled_rule_inputs() {
    for mode in [
        crate::models::TEST_RULES_MODE,
        RuntimeMode::Global,
        RuntimeMode::Direct,
    ] {
        let configuration = configuration();
        let versions = candidate_versions(&configuration, &HashMap::new(), &[]);
        let original = RuntimePlan::build(&configuration, mode, &versions).unwrap();
        let mut candidate = configuration.clone();
        candidate.profiles[0].name = "new display".into();
        candidate.rules[0].name = "new rule display".into();
        candidate.settings.launch_at_login = true;
        candidate.settings.latency_test_url = "https://example.com/check".into();
        candidate.settings.diagnostic_retention = crate::models::RetentionPolicy::Permanent;
        candidate.profiles.reverse();
        let mut disabled = candidate.rules[0].clone();
        disabled.id = "disabled".into();
        disabled.name = "disabled".into();
        disabled.enabled = false;
        candidate.rules.push(disabled);
        assert_eq!(
            original,
            RuntimePlan::build(&candidate, mode, &versions).unwrap()
        );
    }
}

#[test]
fn effective_exit_fields_and_opaque_credential_versions_change_plan() {
    let configuration = configuration();
    let versions = candidate_versions(&configuration, &HashMap::new(), &[]);
    let original = RuntimePlan::build(&configuration, RuntimeMode::Global, &versions).unwrap();
    let changes: [fn(&mut ProxyProfile); 5] = [
        |p| p.protocol = ProxyProtocol::Http,
        |p| p.host = "new.example.com".into(),
        |p| p.port = 8080,
        |p| {
            p.authentication_enabled = false;
            p.credential_ref = None;
        },
        |p| p.credential_ref = Some("versioned-ref".into()),
    ];
    for change in changes {
        let mut candidate = configuration.clone();
        change(&mut candidate.profiles[0]);
        assert_ne!(
            original,
            RuntimePlan::build(&candidate, RuntimeMode::Global, &versions).unwrap()
        );
    }
    let updated = candidate_versions(&configuration, &versions, &["primary".into()]);
    assert_ne!(
        original,
        RuntimePlan::build(&configuration, RuntimeMode::Global, &updated).unwrap()
    );
    assert_ne!(versions["primary"], updated["primary"]);
    assert_eq!(versions["secondary"], updated["secondary"]);
    assert!(uuid::Uuid::parse_str(&updated["primary"]).is_ok());
}

#[test]
fn global_isolates_unrelated_exits_rules_and_preset() {
    let configuration = configuration();
    let versions = candidate_versions(&configuration, &HashMap::new(), &[]);
    let original = RuntimePlan::build(&configuration, RuntimeMode::Global, &versions).unwrap();
    let mut candidate = configuration.clone();
    candidate.profiles[1].host = "new-secondary.example.com".into();
    candidate.rules.reverse();
    candidate.runtime_mode = crate::models::RuntimeMode::Rules {
        use_china_direct: true,
        default_action: crate::models::RuleAction::Proxy,
    };
    let updated = candidate_versions(&candidate, &versions, &["secondary".into()]);
    assert_eq!(
        original,
        RuntimePlan::build(&candidate, RuntimeMode::Global, &updated).unwrap()
    );
    assert_eq!(original.exits.len(), 1);
    assert_eq!(original.credential_versions.len(), 1);
    let rules = RuntimePlan::build(&candidate, candidate.runtime_mode, &updated).unwrap();
    assert_eq!(rules.exits.len(), 2);
    assert_eq!(rules.rules[0].id, "b");
    assert!(rules.china_direct_enabled);
    assert_eq!(rules.default_profile_id.as_deref(), Some("primary"));
}

#[test]
fn rule_order_matcher_ports_target_action_exit_enable_and_preset_change_plan() {
    let configuration = configuration();
    let versions = candidate_versions(&configuration, &HashMap::new(), &[]);
    let original =
        RuntimePlan::build(&configuration, crate::models::TEST_RULES_MODE, &versions).unwrap();
    let changes: [fn(&mut PersistedConfiguration); 9] = [
        |c| c.rules.reverse(),
        |c| c.rules[0].matcher = RuleMatcher::DomainSuffix,
        |c| c.rules[0].target = "new.example.com".into(),
        |c| c.rules[0].port_start = Some(443),
        |c| c.rules[0].port_end = Some(443),
        |c| {
            c.rules[0].action = RuleAction::Direct;
            c.rules[0].proxy_profile_id = None;
        },
        |c| c.rules[0].proxy_profile_id = Some("primary".into()),
        |c| c.rules[0].enabled = false,
        |c| {
            c.runtime_mode = crate::models::RuntimeMode::Rules {
                use_china_direct: true,
                default_action: crate::models::RuleAction::Proxy,
            }
        },
    ];
    for change in changes {
        let mut candidate = configuration.clone();
        change(&mut candidate);
        assert_ne!(
            original,
            RuntimePlan::build(
                &candidate,
                if matches!(candidate.runtime_mode, RuntimeMode::Rules { .. }) {
                    candidate.runtime_mode
                } else {
                    crate::models::TEST_RULES_MODE
                },
                &versions
            )
            .unwrap()
        );
    }
    let mut candidate = configuration.clone();
    candidate.profiles[1].enabled = false;
    candidate.rules.clear();
    assert_eq!(
        RuntimePlan::build(&candidate, crate::models::TEST_RULES_MODE, &versions)
            .unwrap()
            .exits
            .len(),
        1
    );
}
