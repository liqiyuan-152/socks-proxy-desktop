use crate::support::TestApp;
use socks_proxy_lib::services::{
    adapters::{
        ConfigurationStore, RecoveryIntent, RuntimePhase, SessionHealth, SqliteConfigurationStore,
    },
    RuntimeMode,
};

#[test]
fn clean_exit_reopens_committed_configuration() -> Result<(), Box<dyn std::error::Error>> {
    let app = TestApp::new()?;
    let profile = app.add_profile("persisted")?;
    app.service.select_profile(Some(profile.id.clone()))?;
    app.service.request_mode(RuntimeMode::Global)?;
    app.service.stop_runtime()?;
    let before = app.persisted()?;
    let app = app.restart()?;
    assert_eq!(app.persisted()?, before);
    assert_eq!(app.service.list_profiles()?[0].id, profile.id);
    assert_eq!(app.service.runtime_snapshot().phase, RuntimePhase::Stopped);
    assert!(!app.service.health_check()?.recovery_pending);
    Ok(())
}

#[test]
#[ignore = "isolated crash child invoked by parent integration test"]
fn abrupt_commit_child() -> Result<(), Box<dyn std::error::Error>> {
    let database = std::env::var("ARCHITECTURE_CRASH_DATABASE")?;
    let store = SqliteConfigurationStore::open(database)?;
    let previous = store.load()?;
    let mut candidate = previous.clone();
    candidate.profiles[0].name = "committed-after-crash".into();
    let revision = store.recovery_revision()?;
    let intent = RecoveryIntent {
        transaction_id: uuid::Uuid::new_v4().to_string(),
        previous_revision: revision,
        next_revision: revision + 1,
        previous,
        candidate,
        staged_refs: vec![],
        retired_refs: vec![],
        startup: None,
    };
    store.begin_recovery(&intent)?;
    if std::env::var("ARCHITECTURE_CRASH_COMMIT")?.as_str() == "yes" {
        store.commit_recovery(&intent.transaction_id)?;
    }
    std::process::exit(86);
}

#[test]
fn abrupt_exit_retains_exact_durable_journal_and_recovers_both_boundaries(
) -> Result<(), Box<dyn std::error::Error>> {
    for committed in [false, true] {
        let app = TestApp::new()?;
        app.add_profile("before-crash")?;
        let before = app.persisted()?;
        let status = std::process::Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "configuration_recovery::abrupt_commit_child",
                "--ignored",
                "--nocapture",
            ])
            .env("ARCHITECTURE_CRASH_DATABASE", app.database())
            .env(
                "ARCHITECTURE_CRASH_COMMIT",
                if committed { "yes" } else { "no" },
            )
            .status()?;
        assert_eq!(status.code(), Some(86));
        let store = SqliteConfigurationStore::open(app.database())?;
        let record = store.recovery_record()?.ok_or("missing durable journal")?;
        assert_eq!(record.committed, committed);
        assert_eq!(record.intent.previous, before);
        assert_eq!(
            store.recovery_revision()?,
            if committed {
                record.intent.next_revision
            } else {
                record.intent.previous_revision
            }
        );
        assert_eq!(
            store.load()?,
            if committed {
                record.intent.candidate.clone()
            } else {
                before.clone()
            }
        );
        let app = app.restart()?;
        assert!(app.service.health_check()?.recovery_pending);
        let blocked = app
            .add_profile("blocked")
            .expect_err("pending journal blocks writes");
        assert_eq!(blocked.code, "configuration_recovery");
        app.service.recover_network_confirmed(true)?;
        assert!(!app.service.health_check()?.recovery_pending);
        assert!(store.recovery_record()?.is_none());
        assert_eq!(
            app.persisted()?.profiles[0].name,
            if committed {
                "committed-after-crash"
            } else {
                "before-crash"
            }
        );
    }
    Ok(())
}

#[test]
fn failed_recovery_retains_evidence_and_gives_actionable_error(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = TestApp::new()?;
    let profile = app.add_profile("proxy")?;
    app.service.select_profile(Some(profile.id))?;
    app.service.request_mode(RuntimeMode::Global)?;
    app.backend
        .0
        .lock()
        .map_err(|_| "backend poisoned")?
        .fail_restore = true;
    let error = app
        .service
        .recover_network_confirmed(true)
        .err()
        .ok_or("expected recovery failure")?;
    assert!(!error.message.is_empty());
    let context = error.context.ok_or("missing error context")?;
    assert!(!context.error_id.is_empty());
    assert!(!context.recovery_suggestion.is_empty());
    assert!(app.service.health_check()?.recovery_pending);
    assert_eq!(
        app.service.runtime_snapshot().session_health,
        SessionHealth::RecoveryRequired
    );
    app.backend
        .0
        .lock()
        .map_err(|_| "backend poisoned")?
        .fail_restore = false;
    app.service.recover_network_confirmed(true)?;
    assert!(!app.service.health_check()?.recovery_pending);
    Ok(())
}
