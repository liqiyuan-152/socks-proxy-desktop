#[cfg(any(windows, test))]
use crate::credentials::ProxyCredential;
use crate::{error::AppError, models::PersistedConfiguration};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

const EXECUTION_LIMIT: usize = 3;
const QUEUE_LIMIT: usize = 32;

// No Serialize/Debug: the consistent input owns an explicit secret boundary.
pub(crate) struct LatencyInput {
    pub profile_id: String,
    pub configuration_revision: u64,
    pub configuration: PersistedConfiguration,
    #[cfg(any(windows, test))]
    pub credential: Option<ProxyCredential>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct LatencyResult {
    pub latency_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum LatencyTaskState {
    Queued,
    Running,
    Cancelling,
    Cancelled,
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct LatencyTaskSnapshot {
    pub task_id: String,
    pub profile_id: String,
    pub configuration_revision: u64,
    pub state: LatencyTaskState,
    pub result: Option<LatencyResult>,
    pub error: Option<AppError>,
}
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct LatencySubscription {
    pub subscription_id: String,
    pub task: LatencyTaskSnapshot,
}

pub(crate) trait LatencyExecutor: Send + Sync {
    // Return only after stopping and cleaning resources owned by this task.
    fn run(
        &self,
        input: LatencyInput,
        cancelled: Arc<AtomicBool>,
    ) -> Result<LatencyResult, AppError>;
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct TaskKey {
    profile_id: String,
    revision: u64,
    credential_ref: Option<String>,
}
struct Task {
    key: TaskKey,
    input: Option<LatencyInput>,
    snapshot: LatencyTaskSnapshot,
    subscribers: HashSet<String>,
    cancelled: Arc<AtomicBool>,
}
#[derive(Default)]
struct RegistryState {
    tasks: HashMap<String, Task>,
    keys: HashMap<TaskKey, String>,
    subscriptions: HashMap<String, String>,
    queue: VecDeque<String>,
    running: usize,
}

pub(crate) struct LatencyTaskRegistry {
    state: Mutex<RegistryState>,
    executor: Arc<dyn LatencyExecutor>,
}
impl LatencyTaskRegistry {
    #[cfg(any(windows, test))]
    pub(crate) fn new(executor: Arc<dyn LatencyExecutor>) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(RegistryState::default()),
            executor,
        })
    }
    pub(crate) fn subscribe(
        self: &Arc<Self>,
        input: LatencyInput,
    ) -> Result<LatencySubscription, AppError> {
        let key = TaskKey {
            profile_id: input.profile_id.clone(),
            revision: input.configuration_revision,
            credential_ref: input.configuration.profiles[0].credential_ref.clone(),
        };
        let subscription_id = uuid::Uuid::new_v4().to_string();
        let mut state = self.state.lock().map_err(|_| task_error())?;
        let existing = state.keys.get(&key).cloned();
        let id = if let Some(id) = existing {
            id
        } else {
            if state.running >= EXECUTION_LIMIT && state.queue.len() >= QUEUE_LIMIT {
                return Err(busy_error());
            }
            let id = uuid::Uuid::new_v4().to_string();
            let snapshot = LatencyTaskSnapshot {
                task_id: id.clone(),
                profile_id: input.profile_id.clone(),
                configuration_revision: input.configuration_revision,
                state: LatencyTaskState::Queued,
                result: None,
                error: None,
            };
            state.tasks.insert(
                id.clone(),
                Task {
                    key: key.clone(),
                    input: Some(input),
                    snapshot,
                    subscribers: HashSet::new(),
                    cancelled: Arc::new(AtomicBool::new(false)),
                },
            );
            state.keys.insert(key, id.clone());
            state.queue.push_back(id.clone());
            id
        };
        let task = state.tasks.get_mut(&id).ok_or_else(task_error)?;
        task.subscribers.insert(subscription_id.clone());
        let snapshot = task.snapshot.clone();
        state.subscriptions.insert(subscription_id.clone(), id);
        drop(state);
        self.drain();
        Ok(LatencySubscription {
            subscription_id,
            task: snapshot,
        })
    }
    pub(crate) fn snapshot(&self, subscription: &str) -> Result<LatencyTaskSnapshot, AppError> {
        let state = self.state.lock().map_err(|_| task_error())?;
        let id = state
            .subscriptions
            .get(subscription)
            .ok_or_else(task_error)?;
        Ok(state.tasks.get(id).ok_or_else(task_error)?.snapshot.clone())
    }
    fn drain(self: &Arc<Self>) {
        loop {
            let work = {
                let mut state = self
                    .state
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner());
                if state.running >= EXECUTION_LIMIT {
                    return;
                }
                let Some(id) = state.queue.pop_front() else {
                    return;
                };
                let Some(task) = state.tasks.get_mut(&id) else {
                    continue;
                };
                let Some(input) = task.input.take() else {
                    continue;
                };
                task.snapshot.state = LatencyTaskState::Running;
                let cancelled = task.cancelled.clone();
                state.running += 1;
                (id, input, cancelled)
            };
            let registry = Arc::clone(self);
            let failure_id = work.0.clone();
            if std::thread::Builder::new()
                .name("proxy-latency".into())
                .spawn(move || {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        registry.executor.run(work.1, work.2)
                    }))
                    .unwrap_or_else(|_| Err(task_error()));
                    registry.complete(&work.0, result);
                })
                .is_err()
            {
                self.complete(&failure_id, Err(task_error()));
            }
        }
    }
    fn complete(self: &Arc<Self>, id: &str, result: Result<LatencyResult, AppError>) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        state.running -= 1;
        if let Some(task) = state.tasks.get_mut(id) {
            if task.cancelled.load(Ordering::Acquire) {
                task.snapshot.state = LatencyTaskState::Cancelled;
                task.snapshot.result = None;
                task.snapshot.error = None;
            } else {
                match result {
                    Ok(result) => {
                        task.snapshot.state = LatencyTaskState::Succeeded;
                        task.snapshot.result = Some(result);
                    }
                    Err(error) => {
                        task.snapshot.state = LatencyTaskState::Failed;
                        task.snapshot.error = Some(error);
                    }
                }
            }
            if task.subscribers.is_empty() {
                Self::remove_task(&mut state, id);
            }
        }
        drop(state);
        self.drain();
    }
    fn remove_task(state: &mut RegistryState, id: &str) {
        if let Some(task) = state.tasks.remove(id) {
            if state
                .keys
                .get(&task.key)
                .is_some_and(|existing| existing == id)
            {
                state.keys.remove(&task.key);
            }
        }
    }
}
fn task_error() -> AppError {
    AppError {
        code: "latency_task".into(),
        message: "测速任务不可用或订阅已结束".into(),
        fields: vec![],
    }
}
fn busy_error() -> AppError {
    AppError {
        code: "latency_busy".into(),
        message: "测速队列已满，请稍后重试".into(),
        fields: vec![],
    }
}

#[path = "latency_task_lifecycle.rs"]
mod lifecycle;
#[cfg(test)]
#[path = "latency_task_tests.rs"]
mod tests;
