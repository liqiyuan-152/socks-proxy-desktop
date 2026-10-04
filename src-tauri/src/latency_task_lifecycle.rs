use super::*;

impl LatencyTaskRegistry {
    pub(crate) fn release(self: &Arc<Self>, subscription: &str) -> Result<(), AppError> {
        let mut state = self.state.lock().map_err(|_| task_error())?;
        let Some(id) = state.subscriptions.remove(subscription) else {
            return Ok(());
        };
        let task = state.tasks.get_mut(&id).ok_or_else(task_error)?;
        task.subscribers.remove(subscription);
        if task.subscribers.is_empty() {
            Self::cancel_task(&mut state, &id);
            let running = state
                .tasks
                .get(&id)
                .is_some_and(|task| task.snapshot.state == LatencyTaskState::Cancelling);
            if !running {
                Self::remove_task(&mut state, &id);
            }
        }
        drop(state);
        self.drain();
        Ok(())
    }
    pub(crate) fn invalidate(self: &Arc<Self>, revision: u64) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let outdated: Vec<_> = state
            .tasks
            .iter()
            .filter(|(_, task)| task.key.revision != revision)
            .map(|(id, _)| id.clone())
            .collect();
        for id in outdated {
            Self::cancel_task(&mut state, &id);
        }
        drop(state);
        self.drain();
    }
    fn cancel_task(state: &mut RegistryState, id: &str) {
        let Some(task) = state.tasks.get_mut(id) else {
            return;
        };
        task.cancelled.store(true, Ordering::Release);
        if state
            .keys
            .get(&task.key)
            .is_some_and(|existing| existing == id)
        {
            state.keys.remove(&task.key);
        }
        task.snapshot.result = None;
        task.snapshot.error = None;
        if matches!(
            task.snapshot.state,
            LatencyTaskState::Running | LatencyTaskState::Cancelling
        ) {
            task.snapshot.state = LatencyTaskState::Cancelling;
        } else {
            task.snapshot.state = LatencyTaskState::Cancelled;
            task.input.take();
            state.queue.retain(|queued| queued != id);
        }
    }
}
