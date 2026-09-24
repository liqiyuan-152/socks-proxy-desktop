use super::*;

#[test]
fn coordinator_acquires_exclusive_session_before_a_second_instance_can_start() {
    let name = uuid::Uuid::new_v4().to_string();
    let first = ManagedRuntime::from_lease(
        configuration(),
        Box::new(Arc::new(FakeBackend::default())),
        SessionLease::acquire(&name).unwrap(),
        RuntimeMode::Direct,
    )
    .unwrap();
    assert_eq!(
        SessionLease::acquire(&name).err().unwrap().code,
        "runtime_already_owned"
    );
    assert_eq!(first.snapshot().phase, RuntimePhase::Stopped);
    drop(first);
    assert!(SessionLease::acquire(&name).is_ok());
}

#[test]
fn explicit_network_recovery_stops_owned_runtime_before_restoring() {
    let backend = Arc::new(FakeBackend::default());
    let runtime = manager(backend.clone(), configuration());
    runtime.request_mode(RuntimeMode::Global).unwrap();
    let recovered = runtime.recover_network().unwrap();
    assert_eq!(recovered.phase, RuntimePhase::Stopped);
    assert_eq!(recovered.applied_mode, None);
    assert_eq!(recovered.runtime_uptime_ms, None);
    assert_eq!(backend.restorations.load(Ordering::SeqCst), 1);
    assert_eq!(
        backend.modes.lock().unwrap().last(),
        Some(&RuntimeMode::Direct)
    );
}

#[test]
fn failed_exit_recovery_clears_stale_applied_session() {
    let backend = Arc::new(FakeBackend::default());
    let runtime = manager(backend.clone(), configuration());
    runtime.request_mode(RuntimeMode::Global).unwrap();
    backend.fail_reconcile.store(true, Ordering::SeqCst);
    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.phase, RuntimePhase::Failed);
    assert_eq!(snapshot.applied_mode, None);
    assert_eq!(snapshot.runtime_uptime_ms, None);
    assert!(!snapshot.system_proxy_enabled);
    assert!(snapshot.last_error.unwrap().contains("系统代理恢复失败"));
}

#[test]
fn direct_without_session_must_retry_unresolved_startup_recovery() {
    let backend = Arc::new(FakeBackend::default());
    backend.fail_direct.store(true, Ordering::SeqCst);
    let runtime = manager(backend.clone(), configuration());
    runtime.report_startup_recovery_issue("启动时无法恢复系统代理".into());
    assert!(runtime.request_mode(RuntimeMode::Direct).is_err());
    let failed = runtime.snapshot();
    assert_eq!(failed.phase, RuntimePhase::Failed);
    assert_eq!(failed.applied_mode, None);
    assert!(failed.last_error.unwrap().contains("遗留系统代理恢复失败"));
    assert_eq!(
        backend.modes.lock().unwrap().as_slice(),
        &[RuntimeMode::Direct]
    );

    backend.fail_direct.store(false, Ordering::SeqCst);
    let recovered = runtime.request_mode(RuntimeMode::Direct).unwrap();
    assert_eq!(recovered.phase, RuntimePhase::Stopped);
    assert_eq!(recovered.applied_mode, Some(RuntimeMode::Direct));
}

#[test]
fn transition_events_include_short_lived_stages_and_rollback() {
    let backend = Arc::new(FakeBackend::default());
    let runtime = manager(backend.clone(), configuration());
    let mut events = runtime.subscribe();
    runtime.request_mode(RuntimeMode::Global).unwrap();
    assert_eq!(events.try_recv().unwrap().phase, RuntimePhase::Starting);
    assert_eq!(events.try_recv().unwrap().phase, RuntimePhase::Running);
    backend.fail_next.store(true, Ordering::SeqCst);
    assert!(runtime.request_mode(RuntimeMode::Rules).is_err());
    let switching = events.try_recv().unwrap();
    assert_eq!(switching.phase, RuntimePhase::Switching);
    assert_eq!(switching.applied_mode, Some(RuntimeMode::Global));
    let failed = events.try_recv().unwrap();
    assert_eq!(failed.phase, RuntimePhase::Failed);
    assert_eq!(failed.applied_mode, Some(RuntimeMode::Global));
    assert!(events.try_recv().is_err());
}
