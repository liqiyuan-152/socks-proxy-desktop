use super::*;

#[test]
fn rule_set_validation_failure_restores_replaced_credentials() {
    let mut fixture = Fixture::new();
    let mut input = profile_input();
    input.authentication_enabled = true;
    input.credential = Some(CredentialUpdate::Replace {
        username: "alice".into(),
        password: "old-secret".into(),
    });
    let profile = fixture.service.save_profile(input).unwrap();
    fixture
        .service
        .select_profile(Some(profile.id.clone()))
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    let bundled = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/china-rules");
    for name in ["china-domains.srs", "china-ipv4.srs", "china-ipv6.srs"] {
        std::fs::copy(bundled.join(name), root.path().join(name)).unwrap();
    }
    fixture.service = fixture.service.with_china_rule_root(root.path().into());
    fixture.service.set_china_direct_enabled(true).unwrap();
    let before = fixture.store.load().unwrap();
    std::fs::write(root.path().join("china-domains.srs"), b"corrupt").unwrap();
    let mut update = profile_input();
    update.id = Some(profile.id.clone());
    update.authentication_enabled = true;
    update.credential = Some(CredentialUpdate::Replace {
        username: "alice".into(),
        password: "new-secret".into(),
    });
    assert!(fixture.service.save_profile(update).is_err());
    assert_eq!(fixture.store.load().unwrap(), before);
    assert_eq!(
        fixture
            .credentials
            .get(&profile.id)
            .unwrap()
            .unwrap()
            .password,
        "old-secret"
    );
}

#[test]
fn invalid_settings_restore_startup_before_runtime_is_touched() {
    let fixture = Fixture::new();
    let before = fixture.store.load().unwrap();
    let mut settings = before.settings.clone();
    settings.launch_at_login = true;
    settings.latency_test_url = "http://invalid.example".into();
    assert!(fixture.service.update_settings(settings).is_err());
    assert!(!fixture.startup.is_enabled().unwrap());
    assert_eq!(fixture.store.load().unwrap(), before);
    assert!(fixture
        .runtime
        .committed_active_ids
        .lock()
        .unwrap()
        .is_empty());
}

#[test]
fn imported_preset_validation_failure_restores_credentials_and_startup() {
    let fixture = Fixture::new();
    let mut input = profile_input();
    input.authentication_enabled = true;
    input.credential = Some(CredentialUpdate::Replace {
        username: "alice".into(),
        password: "old-secret".into(),
    });
    let profile = fixture.service.save_profile(input).unwrap();
    fixture
        .service
        .select_profile(Some(profile.id.clone()))
        .unwrap();
    let before = fixture.store.load().unwrap();
    let mut portable: serde_json::Value =
        serde_json::from_str(&fixture.service.export().unwrap()).unwrap();
    portable["china_direct_enabled"] = true.into();
    portable["settings"]["launch_at_login"] = true.into();
    let updates = HashMap::from([(
        profile.id.clone(),
        CredentialUpdate::Replace {
            username: "alice".into(),
            password: "new-secret".into(),
        },
    )]);
    assert!(fixture
        .service
        .import(&portable.to_string(), updates)
        .is_err());
    assert_eq!(fixture.store.load().unwrap(), before);
    assert!(!fixture.startup.is_enabled().unwrap());
    assert_eq!(
        fixture
            .credentials
            .get(&profile.id)
            .unwrap()
            .unwrap()
            .password,
        "old-secret"
    );
}

struct FailingStartupRestore(Arc<MemoryStartup>);

impl StartupAdapter for FailingStartupRestore {
    fn is_enabled(&self) -> Result<bool, AppError> {
        self.0.is_enabled()
    }
    fn set_enabled(&self, enabled: bool) -> Result<(), AppError> {
        if !enabled {
            return Err(AppError::unavailable("restore failed"));
        }
        self.0.set_enabled(enabled)
    }
}

#[test]
fn failed_startup_rollback_does_not_skip_runtime_rollback() {
    let mut fixture = Fixture::new();
    fixture.service = ConfigurationService::new(
        Box::new(fixture.store.clone()),
        Box::new(fixture.credentials.clone()),
        Box::new(FailingStartupRestore(fixture.startup.clone())),
        Box::new(fixture.runtime.clone()),
    );
    let before = fixture.store.load().unwrap();
    let mut settings = before.settings.clone();
    settings.launch_at_login = true;
    *fixture.store.fail_save.lock().unwrap() = true;
    assert_eq!(
        fixture.service.update_settings(settings).unwrap_err().code,
        "rollback_failed"
    );
    assert_eq!(fixture.store.load().unwrap(), before);
    assert_eq!(
        fixture.runtime.committed_active_ids.lock().unwrap().len(),
        2
    );
}

#[test]
fn referenced_rule_blocks_disable_and_delete_without_changing_configuration() {
    let fixture = Fixture::new();
    let created = fixture.service.save_profile(profile_input()).unwrap();
    let mut proxy_rule = rule("site", "example.com");
    proxy_rule.proxy_profile_id = Some(created.id.clone());
    fixture.service.replace_rules(vec![proxy_rule]).unwrap();
    let original = fixture.store.load().unwrap();

    let mut disabled = profile_input();
    disabled.id = Some(created.id.clone());
    disabled.enabled = false;
    assert_eq!(
        fixture.service.save_profile(disabled).unwrap_err().fields[0].field,
        "rules[0].proxy_profile_id"
    );
    let error = fixture.service.delete_profile(&created.id).unwrap_err();
    assert_eq!(error.fields[0].field, "rules[0].proxy_profile_id");
    assert!(error.fields[0].message.contains("site"));
    assert_eq!(fixture.store.load().unwrap(), original);

    let mut invalid = rule("new", "other.net");
    invalid.proxy_profile_id = None;
    assert_eq!(
        fixture
            .service
            .replace_rules(vec![invalid])
            .unwrap_err()
            .fields[0]
            .field,
        "rules[0].proxy_profile_id"
    );
    assert_eq!(fixture.store.load().unwrap(), original);
}

#[test]
fn storage_failure_restores_old_credentials_and_runtime_revision() {
    let fixture = Fixture::new();
    let mut input = profile_input();
    input.authentication_enabled = true;
    input.credential = Some(CredentialUpdate::Replace {
        username: "alice".into(),
        password: "old-secret".into(),
    });
    let created = fixture.service.save_profile(input).unwrap();
    *fixture.store.fail_save.lock().unwrap() = true;
    let mut update = profile_input();
    update.id = Some(created.id.clone());
    update.authentication_enabled = true;
    update.credential = Some(CredentialUpdate::Replace {
        username: "alice".into(),
        password: "new-secret".into(),
    });
    assert!(fixture.service.save_profile(update).is_err());
    assert_eq!(
        fixture
            .credentials
            .get(&created.id)
            .unwrap()
            .unwrap()
            .password,
        "old-secret"
    );
    assert_eq!(
        fixture.store.load().unwrap().profiles[0].host,
        "proxy.example.com"
    );
    assert_eq!(
        fixture.runtime.committed_active_ids.lock().unwrap().len(),
        3
    );
}

#[test]
fn rule_reorder_and_settings_reflect_applied_state() {
    let fixture = Fixture::new();
    let profile = fixture.service.save_profile(profile_input()).unwrap();
    let mut first = rule("first", "example.com");
    first.proxy_profile_id = Some(profile.id.clone());
    let mut second = rule("second", "other.net");
    second.proxy_profile_id = Some(profile.id);
    fixture.service.replace_rules(vec![first, second]).unwrap();
    fixture
        .service
        .reorder_rules(&["second".into(), "first".into()])
        .unwrap();
    assert_eq!(fixture.service.list_rules().unwrap()[0].id, "second");
    let error = fixture
        .service
        .reorder_rules(&["second".into(), "second".into()])
        .unwrap_err();
    assert_eq!(error.fields[0].field, "rule_ids");
    let invalid = fixture
        .service
        .replace_rules(vec![rule("bad", "https://example.com")])
        .unwrap_err();
    assert_eq!(invalid.fields[0].field, "rules[0].target");
    let settings = fixture
        .service
        .update_settings(AppSettings {
            launch_at_login: true,
            diagnostic_retention: RetentionPolicy::Days7,
            latency_test_url: crate::models::default_latency_test_url(),
        })
        .unwrap();
    assert!(settings.launch_at_login);
    assert!(*fixture.startup.0.lock().unwrap());
    assert_eq!(
        fixture.service.settings().unwrap().diagnostic_retention,
        RetentionPolicy::Days7
    );
}

#[test]
fn import_requires_fresh_credentials_and_rolls_back_rejected_revision() {
    let fixture = Fixture::new();
    let mut input = profile_input();
    input.authentication_enabled = true;
    input.credential = Some(CredentialUpdate::Replace {
        username: "alice".into(),
        password: "old-secret".into(),
    });
    let created = fixture.service.save_profile(input).unwrap();
    let exported = fixture.service.export().unwrap();
    let mut portable: serde_json::Value = serde_json::from_str(&exported).unwrap();
    portable["profiles"][0]["host"] = "changed.example.com".into();
    portable["settings"]["launch_at_login"] = true.into();
    let json = serde_json::to_string(&portable).unwrap();

    assert_eq!(
        fixture
            .service
            .import(&json, HashMap::new())
            .unwrap_err()
            .fields[0]
            .field,
        "profiles[0].credential"
    );
    assert_eq!(
        fixture.store.load().unwrap().profiles[0].host,
        "proxy.example.com"
    );
    assert_eq!(
        fixture
            .credentials
            .get(&created.id)
            .unwrap()
            .unwrap()
            .password,
        "old-secret"
    );

    *fixture.runtime.reject_next.lock().unwrap() = true;
    let updates = HashMap::from([(
        created.id.clone(),
        CredentialUpdate::Replace {
            username: "alice".into(),
            password: "new-secret".into(),
        },
    )]);
    assert!(fixture.service.import(&json, updates).is_err());
    assert_eq!(
        fixture.store.load().unwrap().profiles[0].host,
        "proxy.example.com"
    );
    assert_eq!(
        fixture
            .credentials
            .get(&created.id)
            .unwrap()
            .unwrap()
            .password,
        "old-secret"
    );
    assert!(!*fixture.startup.0.lock().unwrap());

    let updates = HashMap::from([(
        created.id.clone(),
        CredentialUpdate::Replace {
            username: "alice".into(),
            password: "new-secret".into(),
        },
    )]);
    fixture.service.import(&json, updates).unwrap();
    assert_eq!(
        fixture.store.load().unwrap().profiles[0].host,
        "changed.example.com"
    );
    assert_eq!(
        fixture
            .credentials
            .get(&created.id)
            .unwrap()
            .unwrap()
            .password,
        "new-secret"
    );
    assert!(*fixture.startup.0.lock().unwrap());
}

#[test]
fn public_configuration_revision_tracks_credentials_and_only_successful_commits() {
    let fixture = Fixture::new();
    let mut input = profile_input();
    input.authentication_enabled = true;
    input.credential = Some(CredentialUpdate::Replace {
        username: "alice".into(),
        password: "first-secret".into(),
    });
    let profile = fixture.service.save_profile(input).unwrap();
    let before = fixture.service.list_profiles().unwrap()[0].configuration_revision;
    assert_eq!(profile.configuration_revision, before);
    let mut update = profile_input();
    update.id = Some(profile.id);
    update.authentication_enabled = true;
    update.credential = Some(CredentialUpdate::Replace {
        username: "alice".into(),
        password: "second-secret".into(),
    });
    fixture.service.save_profile(update).unwrap();
    let after = fixture.service.list_profiles().unwrap()[0].configuration_revision;
    assert!(after > before);
    *fixture.store.fail_save.lock().unwrap() = true;
    assert!(fixture
        .service
        .update_settings(AppSettings::default())
        .is_err());
    let views = fixture.service.list_profiles().unwrap();
    assert_eq!(views[0].configuration_revision, after);
    assert!(!serde_json::to_string(&views).unwrap().contains("secret"));
    assert!(!fixture
        .service
        .export()
        .unwrap()
        .contains("configuration_revision"));
}
