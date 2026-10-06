use super::*;
use crate::{
    models::PersistedConfiguration,
    runtime::{BackendSession, ManagedRuntime, RuntimeBackend, RuntimePhase},
    runtime_session::SessionLease,
    services::application_service::tests::{profile_input, Fixture},
    services::{interfaces::ProfileService, profile_service::ProxyProfileService},
    store::ConfigurationStore,
};
use std::sync::atomic::AtomicBool;

#[derive(Default)]
struct Backend {
    reject: AtomicBool,
    recovery_failure: AtomicBool,
}

impl RuntimeBackend for Arc<Backend> {
    fn transition(
        &self,
        _: Option<&BackendSession>,
        _: &PersistedConfiguration,
        mode: RuntimeMode,
        revision: u64,
    ) -> Result<Option<BackendSession>, AppError> {
        if self.reject.swap(false, Ordering::Relaxed) {
            return Err(AppError::unavailable("测试内核拒绝切换"));
        }
        Ok((mode != RuntimeMode::Direct).then(|| BackendSession {
            run_id: "test-run".into(),
            process_id: 1,
            configuration_revision: revision,
            system_proxy_enabled: true,
            tun_enabled: false,
        }))
    }
    fn confirm_transition(&self, _: Option<&BackendSession>) {}
    fn revert_transition(
        &self,
        _: Option<&BackendSession>,
        _: Option<&BackendSession>,
    ) -> Result<(), AppError> {
        Ok(())
    }
    fn reconcile_session(&self, _: &BackendSession) -> Result<bool, AppError> {
        Ok(true)
    }
    fn restore_network(&self) -> Result<(), AppError> {
        if self.recovery_failure.load(Ordering::Relaxed) {
            return Err(AppError::unavailable("测试网络恢复失败"));
        }
        Ok(())
    }
}

#[test]
fn mode_persistence_and_recovery_preserve_coordinator_semantics() -> Result<(), AppError> {
    let fixture = Fixture::new();
    let backend = Arc::new(Backend::default());
    let lease = SessionLease::acquire(&uuid::Uuid::new_v4().to_string())?;
    let coordinator = ManagedRuntime::from_lease(
        fixture.store.load()?,
        Box::new(backend.clone()),
        lease,
        RuntimeMode::Direct,
    )?;
    let previous_context = &fixture.service.context;
    let context = Arc::new(ConfigurationContext::new(
        previous_context.store.clone(),
        previous_context.credentials.clone(),
        previous_context.startup.clone(),
        Arc::new(coordinator),
    ));
    let profiles = ProxyProfileService::new(context.clone());
    let runtime: Arc<dyn RuntimeServiceInterface> = Arc::new(RuntimeService::new(context.clone()));
    let profile = profiles.save_profile(profile_input())?;
    profiles.select_profile(Some(profile.id))?;
    assert_eq!(runtime.snapshot().phase, RuntimePhase::Stopped);
    assert_eq!(
        runtime.active_connections().status,
        crate::observability::ObservationStatus::Degraded
    );
    let running = runtime.set_mode(crate::models::TEST_RULES_MODE)?;
    assert_eq!(running.phase, RuntimePhase::Running);
    assert_eq!(running.applied_mode, Some(crate::models::TEST_RULES_MODE));
    assert_eq!(fixture.store.load_mode()?, crate::models::TEST_RULES_MODE);
    backend.reject.store(true, Ordering::Relaxed);
    assert!(runtime.set_mode(RuntimeMode::Global).is_err());
    assert_eq!(
        runtime.snapshot().applied_mode,
        Some(crate::models::TEST_RULES_MODE)
    );
    // Failed switching restores the previous durable mode and its parameters.
    assert_eq!(fixture.store.load_mode()?, crate::models::TEST_RULES_MODE);
    assert_eq!(
        runtime.set_mode(RuntimeMode::Global)?.applied_mode,
        Some(RuntimeMode::Global)
    );
    *fixture.store.fail_save.lock().expect("test store lock") = true;
    assert!(matches!(
        runtime.stop(),
        Err(RuntimeError::StorageFailed(_))
    ));
    assert_eq!(runtime.snapshot().phase, RuntimePhase::Running);
    *fixture.store.fail_save.lock().expect("test store lock") = false;
    assert_eq!(runtime.stop()?.phase, RuntimePhase::Stopped);
    assert_eq!(fixture.store.load_mode()?, RuntimeMode::Direct);
    runtime.set_mode(crate::models::TEST_RULES_MODE)?;
    backend.recovery_failure.store(true, Ordering::Relaxed);
    assert!(runtime.recover_network().is_err());
    assert!(context.startup_recovery_pending.load(Ordering::Relaxed));
    assert!(matches!(
        runtime.set_mode(RuntimeMode::Global),
        Err(RuntimeError::RecoveryInProgress)
    ));
    assert!(matches!(
        profiles.select_profile(None),
        Err(crate::domain_errors::ProxyError::RecoveryInProgress)
    ));
    backend.recovery_failure.store(false, Ordering::Relaxed);
    assert_eq!(runtime.recover_network()?.phase, RuntimePhase::Stopped);
    assert_eq!(fixture.store.load_mode()?, RuntimeMode::Direct);
    assert!(!context.startup_recovery_pending.load(Ordering::Relaxed));
    profiles.select_profile(None)?;
    Ok(())
}

#[test]
fn rules_parameters_commit_and_failed_hot_change_restores_durable_configuration(
) -> Result<(), AppError> {
    let fixture = Fixture::new();
    let backend = Arc::new(Backend::default());
    let coordinator = ManagedRuntime::from_lease(
        fixture.store.load()?,
        Box::new(backend.clone()),
        SessionLease::acquire(&uuid::Uuid::new_v4().to_string())?,
        RuntimeMode::Direct,
    )?;
    let previous = &fixture.service.context;
    let context = Arc::new(ConfigurationContext::new(
        previous.store.clone(),
        previous.credentials.clone(),
        previous.startup.clone(),
        Arc::new(coordinator),
    ));
    *context.china_rule_root.lock().unwrap() =
        Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/china-rules"));
    let profiles = ProxyProfileService::new(context.clone());
    let profile = profiles.save_profile(profile_input())?;
    profiles.select_profile(Some(profile.id))?;
    let runtime = RuntimeService::new(context.clone());
    for use_china_direct in [false, true] {
        for default_action in [
            crate::models::RuleAction::Proxy,
            crate::models::RuleAction::Direct,
        ] {
            let mode = RuntimeMode::Rules {
                use_china_direct,
                default_action,
            };
            let snapshot = runtime.set_mode(mode)?;
            assert_eq!(snapshot.selected_mode, mode);
            assert_eq!(snapshot.applied_mode, Some(mode));
            assert_eq!(fixture.store.load()?.runtime_mode, mode);
            assert_eq!(fixture.store.load_mode()?, mode);
        }
    }
    let previous = fixture.store.load()?;
    let snapshot = runtime.snapshot();
    let revision = fixture.store.recovery_revision()?;
    backend.reject.store(true, Ordering::Relaxed);
    assert!(runtime.set_mode(crate::models::TEST_RULES_MODE).is_err());
    assert_eq!(fixture.store.load()?, previous);
    assert_eq!(fixture.store.recovery_revision()?, revision);
    assert_eq!(runtime.snapshot().applied_mode, snapshot.applied_mode);
    assert!(fixture.store.recovery_record()?.is_none());
    *fixture.store.fail_save.lock().unwrap() = true;
    assert!(runtime.set_mode(RuntimeMode::Global).is_err());
    assert_eq!(fixture.store.load()?, previous);
    assert_eq!(runtime.snapshot().applied_mode, snapshot.applied_mode);
    Ok(())
}

#[cfg(not(windows))]
#[test]
fn unsupported_platform_saves_parameters_without_claiming_a_running_session() -> Result<(), AppError>
{
    let fixture = Fixture::new();
    let coordinator = ManagedRuntime::from_lease(
        fixture.store.load()?,
        Box::new(crate::runtime_unavailable::UnavailableRuntimeBackend),
        SessionLease::acquire(&uuid::Uuid::new_v4().to_string())?,
        RuntimeMode::Direct,
    )?;
    let previous = &fixture.service.context;
    let context = Arc::new(ConfigurationContext::new(
        previous.store.clone(),
        previous.credentials.clone(),
        previous.startup.clone(),
        Arc::new(coordinator),
    ));
    let runtime = RuntimeService::new(context);
    for use_china_direct in [false, true] {
        for default_action in [
            crate::models::RuleAction::Proxy,
            crate::models::RuleAction::Direct,
        ] {
            let mode = RuntimeMode::Rules {
                use_china_direct,
                default_action,
            };
            let snapshot = runtime.set_mode(mode)?;
            assert_eq!(fixture.store.load()?.runtime_mode, mode);
            assert_eq!(snapshot.selected_mode, mode);
            assert_eq!(snapshot.applied_mode, None);
            assert!(!snapshot.system_proxy_enabled);
            assert_eq!(snapshot.phase, RuntimePhase::Stopped);
        }
    }
    Ok(())
}
