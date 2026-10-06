use super::*;
use crate::runtime_plan::RuntimePlan;

#[test]
fn startup_uses_durable_configuration_revision_and_next_commit_advances_it() {
    let config = configuration();
    let runtime =
        manager(Arc::new(FakeBackend::default()), config.clone()).with_configuration_revision(42);
    assert_eq!(runtime.snapshot().configuration_revision, 42);
    runtime.apply_configuration(&config, &config).unwrap();
    assert_eq!(runtime.snapshot().configuration_revision, 42);
    assert_eq!(runtime.confirm_configuration().configuration_revision, 43);
}

#[test]
fn credential_versions_and_plan_commit_only_after_persistence_confirmation() {
    let mut config = configuration();
    config.profiles[0].authentication_enabled = true;
    config.profiles[0].credential_ref = Some(config.profiles[0].id.clone());
    let backend = Arc::new(FakeBackend::default());
    let runtime = manager(backend.clone(), config.clone());
    let before = runtime.request_mode(RuntimeMode::Global).unwrap();
    let old_versions = runtime.state.lock().unwrap().credential_versions.clone();
    let old_plan = RuntimePlan::build(&config, RuntimeMode::Global, &old_versions).unwrap();
    runtime
        .apply_configuration_with_credentials(&config, &config, &[config.profiles[0].id.clone()])
        .unwrap();
    let pending_versions = {
        let state = runtime.state.lock().unwrap();
        assert_eq!(state.credential_versions, old_versions);
        assert_eq!(state.runtime_plan_revision, before.runtime_plan_revision);
        state.pending.as_ref().unwrap().credential_versions.clone()
    };
    assert_ne!(old_versions, pending_versions);
    runtime.restore_configuration(&config, &before).unwrap();
    let restored = runtime.state.lock().unwrap();
    assert_eq!(restored.credential_versions, old_versions);
    assert_eq!(
        old_plan,
        RuntimePlan::build(
            &restored.configuration,
            RuntimeMode::Global,
            &restored.credential_versions
        )
        .unwrap()
    );
    drop(restored);
    runtime
        .apply_configuration_with_credentials(&config, &config, &[config.profiles[0].id.clone()])
        .unwrap();
    let committed = runtime.confirm_configuration();
    assert_eq!(
        committed.configuration_revision,
        before.configuration_revision + 1
    );
    assert_eq!(
        committed.runtime_plan_revision,
        before.runtime_plan_revision + 1
    );
    assert_ne!(
        runtime.state.lock().unwrap().credential_versions,
        old_versions
    );
}

#[test]
fn global_unrelated_secret_commit_is_metadata_but_next_rules_plan_uses_new_version() {
    let mut config = configuration();
    let mut unrelated = config.profiles[0].clone();
    unrelated.id = "unrelated".into();
    unrelated.name = "Unrelated".into();
    unrelated.authentication_enabled = true;
    unrelated.credential_ref = Some(unrelated.id.clone());
    config.profiles.push(unrelated);
    let backend = Arc::new(FakeBackend::default());
    let runtime = manager(backend.clone(), config.clone());
    let running = runtime.request_mode(RuntimeMode::Global).unwrap();
    let old_versions = runtime.state.lock().unwrap().credential_versions.clone();
    runtime
        .apply_configuration_with_credentials(&config, &config, &["unrelated".into()])
        .unwrap();
    let committed = runtime.confirm_configuration();
    assert_eq!(
        committed.runtime_plan_revision,
        running.runtime_plan_revision
    );
    assert_eq!(backend.modes.lock().unwrap().len(), 1);
    let new_versions = runtime.state.lock().unwrap().credential_versions.clone();
    assert_ne!(new_versions["unrelated"], old_versions["unrelated"]);
    runtime
        .request_mode(crate::models::TEST_RULES_MODE)
        .unwrap();
    let plan = RuntimePlan::build(&config, crate::models::TEST_RULES_MODE, &new_versions).unwrap();
    assert_eq!(
        plan.credential_versions["unrelated"],
        new_versions["unrelated"]
    );
    assert_eq!(backend.modes.lock().unwrap().len(), 2);
}
