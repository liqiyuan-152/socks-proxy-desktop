use super::*;

fn rendered(mode: RuntimeMode) -> Value {
    let config = super::tests::configuration();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/china-rules");
    let sets = ChinaRuleSets::verify(&root).unwrap();
    serde_json::from_str(
        &render_with_rules(
            &config,
            mode,
            SingBoxPorts {
                proxy: 18080,
                control: 19090,
            },
            "test-secret",
            &HashMap::new(),
            Some(&sets),
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn mode_parameters_control_china_rule_sets() {
    for use_china_direct in [false, true] {
        let config = rendered(RuntimeMode::Rules {
            use_china_direct,
            default_action: RuleAction::Proxy,
        });
        assert_eq!(config["route"].get("rule_set").is_some(), use_china_direct);
        assert_eq!(config["route"]["rules"][0]["outbound"], "direct");
        if use_china_direct {
            assert_eq!(
                config["route"]["rules"][2]["rule_set"],
                json!(["cn-domain"])
            );
        }
    }
}

#[test]
fn mode_parameters_control_final_and_domain_fallback_for_all_combinations() {
    for use_china_direct in [false, true] {
        for default_action in [RuleAction::Proxy, RuleAction::Direct] {
            let config = rendered(RuntimeMode::Rules {
                use_china_direct,
                default_action,
            });
            let expected = if default_action == RuleAction::Proxy {
                proxy_tag("proxy-1")
            } else {
                "direct".into()
            };
            assert_eq!(config["route"]["final"], expected);
            assert_eq!(config["route"].get("rule_set").is_some(), use_china_direct);
            if use_china_direct {
                assert_eq!(config["route"]["rules"][3]["outbound"], expected);
            }
        }
    }
}
