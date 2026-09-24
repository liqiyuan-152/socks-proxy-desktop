use crate::{
    credentials::{validate_import_credentials, CredentialUpdate},
    error::AppError,
    models::{
        AppSettings, PersistedConfiguration, ProxyProfile, ProxyProtocol, RetentionPolicy,
        RoutingRule,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PortableProfile {
    id: String,
    name: String,
    protocol: ProxyProtocol,
    host: String,
    port: u16,
    authentication_enabled: bool,
    enabled: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PortableSettings {
    launch_at_login: bool,
    diagnostic_retention: RetentionPolicy,
    #[serde(default = "crate::models::default_latency_test_url")]
    latency_test_url: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PortableConfiguration {
    schema_version: u32,
    profiles: Vec<PortableProfile>,
    rules: Vec<RoutingRule>,
    #[serde(rename = "default_profile_id", alias = "active_profile_id")]
    active_profile_id: Option<String>,
    #[serde(default)]
    china_direct_enabled: bool,
    settings: PortableSettings,
}

pub fn export_configuration_json(
    configuration: &PersistedConfiguration,
) -> Result<String, AppError> {
    configuration.validate_runtime_rules()?;
    let portable = PortableConfiguration {
        schema_version: configuration.schema_version,
        profiles: configuration
            .profiles
            .iter()
            .map(|profile| PortableProfile {
                id: profile.id.clone(),
                name: profile.name.clone(),
                protocol: profile.protocol,
                host: profile.host.clone(),
                port: profile.port,
                authentication_enabled: profile.authentication_enabled,
                enabled: profile.enabled,
            })
            .collect(),
        rules: configuration.rules.clone(),
        active_profile_id: configuration.active_profile_id.clone(),
        china_direct_enabled: configuration.china_direct_enabled,
        settings: PortableSettings {
            launch_at_login: configuration.settings.launch_at_login,
            diagnostic_retention: configuration.settings.diagnostic_retention,
            latency_test_url: configuration.settings.latency_test_url.clone(),
        },
    };
    serde_json::to_string_pretty(&portable).map_err(|_| AppError::storage("配置无法导出"))
}

pub fn parse_import_configuration(
    json: &str,
    updates: &HashMap<String, CredentialUpdate>,
) -> Result<PersistedConfiguration, AppError> {
    let portable: PortableConfiguration =
        serde_json::from_str(json).map_err(|_| AppError::storage("导入配置格式无效"))?;
    let mut configuration = PersistedConfiguration {
        schema_version: portable.schema_version,
        profiles: portable
            .profiles
            .into_iter()
            .map(|profile| ProxyProfile {
                credential_ref: profile.authentication_enabled.then(|| profile.id.clone()),
                id: profile.id,
                name: profile.name,
                protocol: profile.protocol,
                host: profile.host,
                port: profile.port,
                authentication_enabled: profile.authentication_enabled,
                enabled: profile.enabled,
            })
            .collect(),
        rules: portable.rules,
        active_profile_id: portable.active_profile_id,
        china_direct_enabled: portable.china_direct_enabled,
        legacy_unresolved_rule_ids: Vec::new(),
        settings: AppSettings {
            launch_at_login: portable.settings.launch_at_login,
            diagnostic_retention: portable.settings.diagnostic_retention,
            latency_test_url: portable.settings.latency_test_url,
        },
    };
    configuration.migrate_v1()?;
    configuration.validate_runtime_rules()?;
    validate_import_credentials(&configuration, updates)?;
    Ok(configuration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_omits_credential_references_and_import_requires_replacement() {
        let mut configuration = PersistedConfiguration::default();
        configuration.profiles.push(ProxyProfile {
            id: "stable-id".into(),
            name: "Primary".into(),
            protocol: ProxyProtocol::Socks5,
            host: "proxy.example.com".into(),
            port: 1080,
            authentication_enabled: true,
            credential_ref: Some("stable-id".into()),
            enabled: true,
        });
        let exported = export_configuration_json(&configuration).unwrap();
        assert!(!exported.contains("credential_ref"));
        assert!(!exported.contains("password"));
        assert_eq!(
            parse_import_configuration(&exported, &HashMap::new())
                .unwrap_err()
                .fields[0]
                .field,
            "profiles[0].credential"
        );
        let updates = HashMap::from([(
            "stable-id".into(),
            CredentialUpdate::Replace {
                username: "alice".into(),
                password: "secret-value".into(),
            },
        )]);
        assert_eq!(
            parse_import_configuration(&exported, &updates).unwrap(),
            configuration
        );
        assert!(!exported.contains("secret-value"));
        assert!(!exported.contains("alice"));
    }

    #[test]
    fn import_rejects_embedded_password_and_wrong_schema() {
        let json = export_configuration_json(&PersistedConfiguration::default()).unwrap();
        let altered = json.replace(
            "\"profiles\": []",
            "\"profiles\": [], \"password\": \"secret\"",
        );
        assert!(parse_import_configuration(&altered, &HashMap::new()).is_err());
        let altered = json.replace("\"schema_version\": 2", "\"schema_version\": 99");
        assert!(parse_import_configuration(&altered, &HashMap::new()).is_err());
    }

    #[test]
    fn old_export_defaults_test_url_and_new_export_preserves_it() {
        let mut config = PersistedConfiguration::default();
        config.settings.latency_test_url = "https://example.org/check".into();
        let json = export_configuration_json(&config).unwrap();
        assert_eq!(
            parse_import_configuration(&json, &HashMap::new()).unwrap(),
            config
        );
        let old = json.replace(
            ",\n    \"latency_test_url\": \"https://example.org/check\"",
            "",
        );
        let imported = parse_import_configuration(&old, &HashMap::new()).unwrap();
        assert_eq!(
            imported.settings.latency_test_url,
            crate::models::DEFAULT_LATENCY_TEST_URL
        );
    }

    #[test]
    fn v1_import_binds_proxy_rule_and_exports_v2_without_secrets() {
        let mut config = PersistedConfiguration::default();
        config.profiles.push(ProxyProfile {
            id: "one".into(),
            name: "One".into(),
            protocol: ProxyProtocol::Socks5,
            host: "proxy.example.org".into(),
            port: 1080,
            authentication_enabled: false,
            credential_ref: None,
            enabled: true,
        });
        config.active_profile_id = Some("one".into());
        config.rules.push(RoutingRule {
            id: "r".into(),
            name: "R".into(),
            matcher: crate::models::RuleMatcher::Domain,
            target: "example.org".into(),
            port_start: None,
            port_end: None,
            action: crate::models::RuleAction::Proxy,
            proxy_profile_id: Some("one".into()),
            enabled: true,
        });
        let mut old: serde_json::Value =
            serde_json::from_str(&export_configuration_json(&config).unwrap()).unwrap();
        old["schema_version"] = 1.into();
        old["active_profile_id"] = old["default_profile_id"].take();
        old.as_object_mut().unwrap().remove("default_profile_id");
        old["rules"][0]
            .as_object_mut()
            .unwrap()
            .remove("proxy_profile_id");
        let imported = parse_import_configuration(&old.to_string(), &HashMap::new()).unwrap();
        assert_eq!(imported, config);
        assert!(export_configuration_json(&imported)
            .unwrap()
            .contains("\"schema_version\": 2"));
        old["active_profile_id"] = serde_json::Value::Null;
        assert_eq!(
            parse_import_configuration(&old.to_string(), &HashMap::new())
                .unwrap_err()
                .fields[0]
                .field,
            "rules[0].proxy_profile_id"
        );
    }
}
