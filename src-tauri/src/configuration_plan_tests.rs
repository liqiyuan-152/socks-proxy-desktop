use super::*;

fn run_id(fixture: &RuntimeFixture) -> String {
    fixture
        .backend
        .runs
        .lock()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone()
}

#[test]
fn rename_and_unrelated_settings_commit_without_replacing_actual_process() {
    let fixture = RuntimeFixture::new();
    let identity = run_id(&fixture);
    let before = fixture.service.runtime_snapshot();
    let mut input = profile_input();
    input.id = Some(fixture.id.clone());
    input.name = "Renamed".into();
    input.authentication_enabled = true;
    fixture.service.save_profile(input).unwrap();
    let renamed = fixture.service.runtime_snapshot();
    assert_eq!(run_id(&fixture), identity);
    assert_eq!(
        renamed.configuration_revision,
        before.configuration_revision + 1
    );
    assert_eq!(renamed.runtime_plan_revision, before.runtime_plan_revision);
    assert_eq!(fixture.service.list_profiles().unwrap()[0].name, "Renamed");
    let mut settings = fixture.service.settings().unwrap();
    settings.latency_test_url = "https://example.com/test".into();
    fixture.service.update_settings(settings).unwrap();
    assert_eq!(run_id(&fixture), identity);
    assert_eq!(
        fixture.service.runtime_snapshot().runtime_plan_revision,
        before.runtime_plan_revision
    );
    // A subsequent credential replacement still changes the effective plan.
    let mut input = profile_input();
    input.id = Some(fixture.id.clone());
    input.name = "Renamed".into();
    input.authentication_enabled = true;
    input.credential = Some(RuntimeFixture::update("alice", "new-password"));
    fixture.service.save_profile(input).unwrap();
    assert_ne!(run_id(&fixture), identity);
    let after = fixture.service.runtime_snapshot();
    assert_eq!(
        after.runtime_plan_revision,
        before.runtime_plan_revision + 1
    );
    fixture.assert_live("alice", "new-password", after.runtime_plan_revision);
}

#[test]
fn failed_metadata_commit_preserves_display_process_and_revisions() {
    let fixture = RuntimeFixture::new();
    let identity = run_id(&fixture);
    let before = fixture.service.runtime_snapshot();
    *fixture.store.fail_save.lock().unwrap() = true;
    let mut input = profile_input();
    input.id = Some(fixture.id.clone());
    input.name = "Rejected name".into();
    input.authentication_enabled = true;
    assert!(fixture.service.save_profile(input).is_err());
    assert_eq!(fixture.service.list_profiles().unwrap()[0].name, "Primary");
    let after = fixture.service.runtime_snapshot();
    assert_eq!(after.configuration_revision, before.configuration_revision);
    assert_eq!(after.runtime_plan_revision, before.runtime_plan_revision);
    assert_eq!(run_id(&fixture), identity);
}

#[test]
fn rejected_credential_plan_restores_version_and_retry_advances_only_once() {
    let fixture = RuntimeFixture::new();
    let identity = run_id(&fixture);
    let before = fixture.service.runtime_snapshot();
    *fixture.store.fail_save.lock().unwrap() = true;
    assert!(fixture.save("bob", "new-password").is_err());
    let rejected = fixture.service.runtime_snapshot();
    assert_eq!(rejected.runtime_plan_revision, before.runtime_plan_revision);
    assert_eq!(
        rejected.configuration_revision,
        before.configuration_revision
    );
    assert_eq!(run_id(&fixture), identity);
    *fixture.store.fail_save.lock().unwrap() = false;
    fixture.save("bob", "new-password").unwrap();
    let retried = fixture.service.runtime_snapshot();
    assert_eq!(
        retried.runtime_plan_revision,
        before.runtime_plan_revision + 1
    );
    assert_eq!(
        retried.configuration_revision,
        before.configuration_revision + 1
    );
    assert_ne!(run_id(&fixture), identity);
    fixture.assert_live("bob", "new-password", retried.runtime_plan_revision);
}

#[test]
fn configuration_recovery_failure_is_visible_without_abandoning_healthy_session() {
    let fixture = RuntimeFixture::new();
    let identity = run_id(&fixture);
    fixture
        .service
        .report_configuration_recovery_issue(&AppError::unavailable("配置恢复未完成"));
    let snapshot = fixture.service.runtime_snapshot();
    assert_eq!(
        snapshot.session_health,
        crate::runtime::SessionHealth::Healthy
    );
    assert_eq!(snapshot.applied_mode, Some(RuntimeMode::Global));
    assert_eq!(
        snapshot.last_operation.outcome,
        crate::runtime::OperationOutcome::Failed
    );
    assert_eq!(
        snapshot.last_operation.error.as_deref(),
        Some("配置恢复未完成")
    );
    fixture.backend.runs.lock().unwrap().remove(&identity);
    assert_eq!(
        fixture.service.runtime_snapshot().session_health,
        crate::runtime::SessionHealth::Exited
    );
}
