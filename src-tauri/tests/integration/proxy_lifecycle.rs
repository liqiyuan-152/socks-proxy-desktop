use crate::support::TestApp;
use socks_proxy_lib::services::{
    adapters::{CredentialUpdate, RuntimePhase, SessionHealth},
    RuntimeMode,
};

#[test]
fn add_start_stop_and_persist_profile() -> Result<(), Box<dyn std::error::Error>> {
    let app = TestApp::new()?;
    let profile = app.add_profile("first")?;
    app.service.select_profile(Some(profile.id.clone()))?;
    let running = app.service.request_mode(RuntimeMode::Rules {
        use_china_direct: false,
        default_action: socks_proxy_lib::services::RuleAction::Proxy,
    })?;
    assert_eq!(running.phase, RuntimePhase::Running);
    assert_eq!(running.session_health, SessionHealth::Healthy);
    assert_eq!(running.active_profile_id, Some(profile.id.clone()));
    let stopped = app.service.stop_runtime()?;
    assert_eq!(stopped.phase, RuntimePhase::Stopped);
    assert_eq!(stopped.applied_mode, None);
    assert_eq!(app.persisted()?.profiles[0].id, profile.id);
    assert_eq!(
        app.backend
            .0
            .lock()
            .map_err(|_| "backend poisoned")?
            .confirmations,
        3
    );
    Ok(())
}

#[test]
fn multiple_profiles_switch_and_in_use_deletion_is_rejected(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = TestApp::new()?;
    let first = app.add_profile("first")?;
    let second = app.add_profile("second")?;
    app.service.select_profile(Some(first.id.clone()))?;
    app.service.request_mode(RuntimeMode::Global)?;
    app.service.select_profile(Some(second.id.clone()))?;
    assert_eq!(
        app.service.runtime_snapshot().active_profile_id,
        Some(second.id.clone())
    );
    let error = app
        .service
        .delete_profile(&second.id)
        .expect_err("active profile in use");
    assert_eq!(error.code, "proxy_in_use");
    app.service.delete_profile(&first.id)?;
    assert_eq!(app.service.list_profiles()?.len(), 1);
    assert_eq!(
        app.backend
            .0
            .lock()
            .map_err(|_| "backend poisoned")?
            .sessions_started,
        2
    );
    Ok(())
}

#[test]
fn credentials_update_and_validation_preserve_previous_version(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = TestApp::new()?;
    let profile = app.service.save_profile(TestApp::profile_input(
        None,
        "authenticated",
        CredentialUpdate::Replace {
            username: "first".into(),
            password: "secret-one".into(),
        },
    ))?;
    app.service.save_profile(TestApp::profile_input(
        Some(profile.id.clone()),
        "authenticated",
        CredentialUpdate::Replace {
            username: "second".into(),
            password: "secret-two".into(),
        },
    ))?;
    let credential = app.service.profile_credential(&profile.id)?;
    assert_eq!(credential.username, "second");
    assert_eq!(credential.password, "secret-two");
    assert!(!serde_json::to_string(&app.persisted()?)?.contains("secret-two"));
    let error = app
        .service
        .save_profile(TestApp::profile_input(
            Some(profile.id.clone()),
            "authenticated",
            CredentialUpdate::Replace {
                username: "".into(),
                password: "".into(),
            },
        ))
        .expect_err("blank credential rejected");
    assert_eq!(error.code, "validation_error");
    assert_eq!(
        app.service.profile_credential(&profile.id)?.password,
        "secret-two"
    );
    Ok(())
}
