use super::*;
use crate::{
    services::application_service::tests::{profile_input, Fixture},
    store::ConfigurationStore,
};

#[test]
fn routing_errors_identify_invalid_targets_rules_and_missing_assets() -> Result<(), AppError> {
    let fixture = Fixture::new();
    let routing = RoutingService::new(fixture.service.context.clone());
    assert!(
        matches!(routing.test_route("", 443), Err(RoutingError::InvalidTarget(value)) if value.fields[0].field == "target")
    );
    assert!(
        matches!(routing.test_route("example.com", 0), Err(RoutingError::InvalidTarget(value)) if value.fields[0].field == "port")
    );
    assert!(matches!(
        routing.reorder_rules(&["missing".into()]),
        Err(RoutingError::RuleValidationFailed(_))
    ));
    let profile = fixture.service.save_profile(profile_input())?;
    fixture.service.select_profile(Some(profile.id))?;
    assert!(matches!(
        routing.set_china_direct_enabled(true),
        Err(RoutingError::ChinaRulesUnavailable)
    ));
    assert!(!fixture.store.load()?.china_direct_enabled());
    fixture
        .service
        .context
        .startup_recovery_pending
        .store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        routing.replace_rules(vec![]),
        Err(RoutingError::RecoveryInProgress)
    ));
    Ok(())
}

#[test]
fn rules_validate_references_and_reorder_without_partial_commits() -> Result<(), AppError> {
    use crate::models::{RuleAction, RuleMatcher};
    let fixture = Fixture::new();
    let routing: Arc<dyn RoutingServiceInterface> =
        Arc::new(RoutingService::new(fixture.service.context.clone()));
    let profile = fixture.service.save_profile(profile_input())?;
    let make_rule = |id: &str, target: &str| RoutingRule {
        id: id.into(),
        name: id.into(),
        matcher: RuleMatcher::Domain,
        target: target.into(),
        port_start: None,
        port_end: None,
        action: RuleAction::Proxy,
        proxy_profile_id: Some(profile.id.clone()),
        enabled: true,
    };
    let first = make_rule("first", "example.com");
    let second = make_rule("second", "example.org");
    routing.replace_rules(vec![first.clone(), second.clone()])?;
    let mut configuration = fixture.store.load()?;
    configuration.runtime_mode = crate::models::TEST_RULES_MODE;
    fixture.store.save(&configuration)?;
    let prediction = routing.test_route("EXAMPLE.COM.", 443)?;
    assert_eq!(prediction.proxy_profile_id, Some(profile.id.clone()));
    assert_eq!(prediction.matched_rule_id.as_deref(), Some("first"));
    assert_eq!(
        prediction.configuration_revision,
        fixture.store.recovery_revision()?
    );
    routing.reorder_rules(&["second".into(), "first".into()])?;
    assert_eq!(routing.list_rules()?[0].id, "second");
    let revision = fixture.store.recovery_revision()?;
    assert!(routing
        .reorder_rules(&["first".into(), "first".into()])
        .is_err());
    assert!(routing.reorder_rules(&["first".into()]).is_err());
    let mut invalid = first.clone();
    invalid.proxy_profile_id = Some("missing".into());
    assert!(routing.replace_rules(vec![invalid]).is_err());
    assert!(fixture.service.delete_profile(&profile.id).is_err());
    assert_eq!(fixture.store.recovery_revision()?, revision);
    *fixture
        .runtime
        .reject_next
        .lock()
        .expect("test runtime lock") = true;
    assert!(routing.replace_rules(vec![]).is_err());
    assert_eq!(routing.list_rules()?.len(), 2);
    assert_eq!(routing.list_rules()?[0].id, "second");
    assert_eq!(fixture.store.recovery_revision()?, revision);
    assert!(fixture.store.recovery_record()?.is_none());
    routing.replace_rules(vec![])?;
    fixture.service.delete_profile(&profile.id)?;
    Ok(())
}
#[test]
fn china_preset_requires_default_and_valid_bundled_rules_before_commit() {
    let mut fixture = Fixture::new();
    let routing: Arc<dyn RoutingServiceInterface> =
        Arc::new(RoutingService::new(fixture.service.context.clone()));
    assert!(!routing.get_china_status().unwrap().available);
    assert!(routing.set_china_direct_enabled(true).is_err());
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/china-rules");
    fixture.service = fixture.service.with_china_rule_root(root);
    assert!(routing.get_china_status().unwrap().available);
    assert_eq!(
        AppError::from(routing.set_china_direct_enabled(true).unwrap_err()).fields[0].field,
        "default_profile_id"
    );
    let profile = fixture.service.save_profile(profile_input()).unwrap();
    fixture.service.select_profile(Some(profile.id)).unwrap();
    let status = routing.set_china_direct_enabled(true).unwrap();
    assert!(status.enabled && status.available && status.data_date.is_some());
    assert!(fixture.store.load().unwrap().china_direct_enabled());
}

#[test]
fn invalid_china_rules_reject_revision_without_changing_stored_configuration() {
    let mut fixture = Fixture::new();
    let routing: Arc<dyn RoutingServiceInterface> =
        Arc::new(RoutingService::new(fixture.service.context.clone()));
    let profile = fixture.service.save_profile(profile_input()).unwrap();
    fixture.service.select_profile(Some(profile.id)).unwrap();
    let missing = tempfile::tempdir().unwrap();
    fixture.service = fixture.service.with_china_rule_root(missing.path().into());
    let error = routing.set_china_direct_enabled(true).unwrap_err();
    assert!(error.to_string().contains("规则集"));
    assert!(!fixture.store.load().unwrap().china_direct_enabled());
}
