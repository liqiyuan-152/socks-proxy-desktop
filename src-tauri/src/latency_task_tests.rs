use super::*;
use crate::models::{ProxyProfile, ProxyProtocol};
use std::{
    sync::Condvar,
    time::{Duration, Instant},
};

#[derive(Default)]
struct Executor {
    counts: Mutex<(usize, usize, usize)>,
    gate: Condvar,
    released: AtomicBool,
}
impl LatencyExecutor for Executor {
    fn run(&self, _: LatencyInput, cancelled: Arc<AtomicBool>) -> Result<LatencyResult, AppError> {
        let mut counts = self.counts.lock().unwrap();
        counts.0 += 1;
        counts.1 += 1;
        counts.2 = counts.2.max(counts.0);
        self.gate.notify_all();
        while !self.released.load(Ordering::Acquire) && !cancelled.load(Ordering::Acquire) {
            counts = self
                .gate
                .wait_timeout(counts, Duration::from_millis(10))
                .unwrap()
                .0;
        }
        counts.0 -= 1;
        Ok(LatencyResult { latency_ms: 42 })
    }
}
fn input(id: usize, revision: u64) -> LatencyInput {
    let id = id.to_string();
    LatencyInput {
        profile_id: id.clone(),
        configuration_revision: revision,
        credential: None,
        configuration: PersistedConfiguration {
            profiles: vec![ProxyProfile {
                id: id.clone(),
                name: id.clone(),
                protocol: ProxyProtocol::Socks5,
                host: "proxy.example.com".into(),
                port: 1080,
                authentication_enabled: false,
                credential_ref: None,
                enabled: true,
            }],
            active_profile_id: Some(id),
            ..Default::default()
        },
    }
}
fn wait(condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !condition() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn application_limit_queue_capacity_and_shared_subscribers() {
    let executor = Arc::new(Executor::default());
    let registry = LatencyTaskRegistry::new(executor.clone());
    let subscriptions: Vec<_> = (0..35)
        .map(|id| registry.subscribe(input(id, 1)).unwrap())
        .collect();
    wait(|| executor.counts.lock().unwrap().0 == 3);
    assert_eq!(registry.state.lock().unwrap().queue.len(), 32);
    assert_eq!(
        registry.subscribe(input(35, 1)).err().unwrap().code,
        "latency_busy"
    );
    let shared = registry.subscribe(input(0, 1)).unwrap();
    assert_eq!(shared.task.task_id, subscriptions[0].task.task_id);
    registry.release(&shared.subscription_id).unwrap();
    assert!(!registry.state.lock().unwrap().tasks[&shared.task.task_id]
        .cancelled
        .load(Ordering::Acquire));
    executor.released.store(true, Ordering::Release);
    executor.gate.notify_all();
    wait(|| registry.state.lock().unwrap().running == 0);
    assert_eq!(executor.counts.lock().unwrap().1, 35);
    assert_eq!(executor.counts.lock().unwrap().2, 3);
    for subscription in subscriptions {
        assert_eq!(
            registry
                .snapshot(&subscription.subscription_id)
                .unwrap()
                .state,
            LatencyTaskState::Succeeded
        );
        registry.release(&subscription.subscription_id).unwrap();
    }
    assert!(registry.state.lock().unwrap().tasks.is_empty());
}
#[test]
fn last_subscriber_and_revision_invalidation_cancel_without_early_slot_release() {
    let executor = Arc::new(Executor::default());
    let registry = LatencyTaskRegistry::new(executor.clone());
    let subscriptions: Vec<_> = (0..4)
        .map(|id| registry.subscribe(input(id, 1)).unwrap())
        .collect();
    wait(|| executor.counts.lock().unwrap().0 == 3);
    registry.release(&subscriptions[3].subscription_id).unwrap();
    assert!(registry.state.lock().unwrap().queue.is_empty());
    registry.invalidate(2);
    wait(|| registry.state.lock().unwrap().running == 0);
    for subscription in &subscriptions[..3] {
        assert_eq!(
            registry
                .snapshot(&subscription.subscription_id)
                .unwrap()
                .state,
            LatencyTaskState::Cancelled
        );
        registry.release(&subscription.subscription_id).unwrap();
    }
    assert_eq!(executor.counts.lock().unwrap().1, 3);
    assert!(registry.state.lock().unwrap().tasks.is_empty());
}
