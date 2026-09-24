use crate::models::RoutingRule;
#[cfg(test)]
use crate::models::RuleAction;
use crate::models::RuleMatcher;
use crate::{error::AppError, models::PersistedConfiguration};
use std::net::IpAddr;

#[derive(Debug)]
pub struct CompiledRules {
    rules: Vec<RoutingRule>,
}

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleDecision {
    pub action: RuleAction,
    pub matched_rule_id: Option<String>,
}

impl CompiledRules {
    pub fn compile(configuration: &PersistedConfiguration) -> Result<Self, AppError> {
        configuration.validate()?;
        Ok(Self {
            rules: configuration
                .rules
                .iter()
                .filter(|rule| rule.enabled)
                .cloned()
                .collect(),
        })
    }

    #[cfg(test)]
    pub fn decide(&self, host: &str, port: u16) -> RuleDecision {
        self.matching_rule(host, port).map_or(
            RuleDecision {
                action: RuleAction::Direct,
                matched_rule_id: None,
            },
            |rule| RuleDecision {
                action: rule.action,
                matched_rule_id: Some(rule.id.clone()),
            },
        )
    }

    pub fn matching_rule(&self, host: &str, port: u16) -> Option<&RoutingRule> {
        self.rules
            .iter()
            .find(|rule| matches_port(rule, port) && matches_target(rule, host))
    }

    pub fn ordered_rules(&self) -> &[RoutingRule] {
        &self.rules
    }
}

fn matches_port(rule: &RoutingRule, port: u16) -> bool {
    match (rule.port_start, rule.port_end) {
        (None, None) => true,
        (Some(start), None) => port == start,
        (None, Some(end)) => port == end,
        (Some(start), Some(end)) => (start..=end).contains(&port),
    }
}

fn matches_target(rule: &RoutingRule, host: &str) -> bool {
    match rule.matcher {
        RuleMatcher::Domain => host
            .trim_end_matches('.')
            .eq_ignore_ascii_case(&rule.target),
        RuleMatcher::DomainSuffix => {
            let normalized = host.trim_end_matches('.').to_ascii_lowercase();
            let suffix = rule.target.trim_end_matches('.').to_ascii_lowercase();
            normalized == suffix || normalized.ends_with(&format!(".{suffix}"))
        }
        RuleMatcher::IpCidr => matches_cidr(&rule.target, host),
    }
}

fn matches_cidr(cidr: &str, host: &str) -> bool {
    let Some((network, prefix)) = cidr.split_once('/') else {
        return false;
    };
    let (Ok(network), Ok(destination), Ok(prefix)) = (
        network.parse::<IpAddr>(),
        host.parse::<IpAddr>(),
        prefix.parse::<u8>(),
    ) else {
        return false;
    };
    match (network, destination) {
        (IpAddr::V4(network), IpAddr::V4(destination)) if prefix <= 32 => {
            let mask = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - prefix)
            };
            (u32::from(network) & mask) == (u32::from(destination) & mask)
        }
        (IpAddr::V6(network), IpAddr::V6(destination)) if prefix <= 128 => {
            let mask = if prefix == 0 {
                0
            } else {
                u128::MAX << (128 - prefix)
            };
            (u128::from(network) & mask) == (u128::from(destination) & mask)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ProxyProfile, ProxyProtocol};

    fn with_profile(mut configuration: PersistedConfiguration) -> PersistedConfiguration {
        configuration.profiles.push(ProxyProfile {
            id: "proxy-1".into(),
            name: "Primary".into(),
            protocol: ProxyProtocol::Socks5,
            host: "proxy.example.com".into(),
            port: 1080,
            authentication_enabled: false,
            credential_ref: None,
            enabled: true,
        });
        configuration
    }

    fn rule(id: &str, matcher: RuleMatcher, target: &str, action: RuleAction) -> RoutingRule {
        RoutingRule {
            id: id.into(),
            name: id.into(),
            matcher,
            target: target.into(),
            port_start: None,
            port_end: None,
            action,
            proxy_profile_id: (action == RuleAction::Proxy).then(|| "proxy-1".into()),
            enabled: true,
        }
    }

    #[test]
    fn respects_display_order_and_falls_back_to_direct() {
        let configuration = PersistedConfiguration {
            rules: vec![
                rule(
                    "first",
                    RuleMatcher::DomainSuffix,
                    "example.com",
                    RuleAction::Direct,
                ),
                rule(
                    "second",
                    RuleMatcher::Domain,
                    "api.example.com",
                    RuleAction::Proxy,
                ),
            ],
            ..PersistedConfiguration::default()
        };
        let compiled = CompiledRules::compile(&with_profile(configuration)).unwrap();
        assert_eq!(compiled.ordered_rules()[0].id, "first");
        assert_eq!(
            compiled.decide("api.example.com", 443),
            RuleDecision {
                action: RuleAction::Direct,
                matched_rule_id: Some("first".into()),
            }
        );
        assert_eq!(
            compiled.decide("elsewhere.net", 443),
            RuleDecision {
                action: RuleAction::Direct,
                matched_rule_id: None,
            }
        );
    }

    #[test]
    fn matches_domain_boundaries_ports_and_cidr() {
        let mut configuration = PersistedConfiguration::default();
        let mut domain = rule(
            "domain",
            RuleMatcher::DomainSuffix,
            "example.com",
            RuleAction::Proxy,
        );
        domain.port_start = Some(443);
        let cidr = rule("cidr", RuleMatcher::IpCidr, "10.0.0.0/8", RuleAction::Proxy);
        configuration.rules = vec![domain, cidr];
        let compiled = CompiledRules::compile(&with_profile(configuration)).unwrap();
        assert_eq!(
            compiled
                .decide("api.example.com", 443)
                .matched_rule_id
                .as_deref(),
            Some("domain")
        );
        assert_eq!(compiled.decide("notexample.com", 443).matched_rule_id, None);
        assert_eq!(compiled.decide("api.example.com", 80).matched_rule_id, None);
        assert_eq!(
            compiled.decide("10.2.3.4", 80).matched_rule_id.as_deref(),
            Some("cidr")
        );
    }

    #[test]
    fn rejects_invalid_rule_before_compiling() {
        let mut configuration = PersistedConfiguration::default();
        configuration.rules.push(rule(
            "cidr",
            RuleMatcher::IpCidr,
            "10.0.0.0/44",
            RuleAction::Proxy,
        ));
        assert_eq!(
            CompiledRules::compile(&configuration).unwrap_err().fields[0].field,
            "rules[0].target"
        );
    }
}
