use crate::support::TestApp;
use socks_proxy_lib::services::{
    adapters::{CredentialUpdate, RuntimePhase, SessionHealth},
    RuntimeMode,
};
use std::sync::Arc;

#[test]
fn rules_global_direct_cycle_preserves_committed_modes() -> Result<(), Box<dyn std::error::Error>> {
    let app = TestApp::new()?;
    let profile = app.add_profile("proxy")?;
    app.service.select_profile(Some(profile.id))?;
    for mode in [
        RuntimeMode::Rules {
            use_china_direct: false,
            default_action: socks_proxy_lib::services::RuleAction::Proxy,
        },
        RuntimeMode::Global,
        RuntimeMode::Direct,
    ] {
        let snapshot = app.service.request_mode(mode)?;
        assert_eq!(snapshot.applied_mode, Some(mode));
        assert_eq!(snapshot.selected_mode, mode);
        assert_eq!(
            snapshot.phase,
            if mode == RuntimeMode::Direct {
                RuntimePhase::Stopped
            } else {
                RuntimePhase::Running
            }
        );
        assert_eq!(snapshot.system_proxy_enabled, mode != RuntimeMode::Direct);
    }
    Ok(())
}

#[test]
fn crash_restores_network_and_explicit_retry_starts_new_session(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = TestApp::new()?;
    let profile = app.add_profile("proxy")?;
    app.service.select_profile(Some(profile.id))?;
    let before = app.service.request_mode(RuntimeMode::Rules {
        use_china_direct: false,
        default_action: socks_proxy_lib::services::RuleAction::Proxy,
    })?;
    app.backend.0.lock().map_err(|_| "backend poisoned")?.exited = true;
    let failed = app.service.runtime_snapshot();
    assert_eq!(failed.session_health, SessionHealth::Exited);
    assert_eq!(failed.applied_mode, None);
    assert!(!failed.system_proxy_enabled);
    assert_eq!(failed.last_operation, before.last_operation);
    app.service.runtime_snapshot();
    assert_eq!(
        app.backend
            .0
            .lock()
            .map_err(|_| "backend poisoned")?
            .restorations,
        1
    );
    let restored = app.service.request_mode(RuntimeMode::Rules {
        use_china_direct: false,
        default_action: socks_proxy_lib::services::RuleAction::Proxy,
    })?;
    assert_eq!(restored.session_health, SessionHealth::Healthy);
    assert!(restored.revision > before.revision);
    Ok(())
}

#[test]
fn plan_change_restarts_but_metadata_change_preserves_session(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = TestApp::new()?;
    let profile = app.add_profile("proxy")?;
    app.service.select_profile(Some(profile.id.clone()))?;
    let before = app.service.request_mode(RuntimeMode::Rules {
        use_china_direct: false,
        default_action: socks_proxy_lib::services::RuleAction::Proxy,
    })?;
    app.service.save_profile(TestApp::profile_input(
        Some(profile.id.clone()),
        "renamed",
        CredentialUpdate::Delete,
    ))?;
    assert_eq!(
        app.backend
            .0
            .lock()
            .map_err(|_| "backend poisoned")?
            .sessions_started,
        1
    );
    let mut input = TestApp::profile_input(Some(profile.id), "renamed", CredentialUpdate::Delete);
    input.port = 1081;
    app.service.save_profile(input)?;
    assert_eq!(
        app.backend
            .0
            .lock()
            .map_err(|_| "backend poisoned")?
            .sessions_started,
        2
    );
    let after = app.service.runtime_snapshot();
    assert_eq!(after.phase, RuntimePhase::Running);
    assert!(after.runtime_plan_revision > before.runtime_plan_revision);
    assert_eq!(app.persisted()?.profiles[0].port, 1081);
    Ok(())
}

#[test]
fn concurrent_writes_keep_configuration_and_runtime_consistent(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = Arc::new(TestApp::new()?);
    let profile = app.add_profile("proxy")?;
    app.service.select_profile(Some(profile.id))?;
    let mut workers = Vec::new();
    for index in 0..8 {
        let app = app.clone();
        workers.push(std::thread::spawn(move || {
            if index % 2 == 0 {
                app.service.request_mode(RuntimeMode::Global).map(|_| ())
            } else {
                app.add_profile(&format!("additional-{index}")).map(|_| ())
            }
        }));
    }
    for worker in workers {
        worker.join().map_err(|_| "worker panicked")??;
    }
    let persisted = app.persisted()?;
    assert_eq!(persisted.profiles.len(), 5);
    assert_eq!(app.service.list_profiles()?.len(), 5);
    let snapshot = app.service.runtime_snapshot();
    assert_eq!(snapshot.phase, RuntimePhase::Running);
    assert_eq!(snapshot.applied_mode, Some(RuntimeMode::Global));
    assert_eq!(snapshot.session_health, SessionHealth::Healthy);
    assert_eq!(snapshot.active_profile_id, persisted.active_profile_id);
    Ok(())
}
