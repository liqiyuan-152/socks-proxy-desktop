use super::*;

impl RuntimeStateNode {
    /// 原节点不被消费；拒绝无效事件或候选时，调用方仍拥有旧状态。
    pub fn transition(&self, event: RuntimeEvent) -> Result<Self, RuntimeError> {
        self.verify_invariants()?;
        let rejected = RuntimeError::InvalidTransition {
            from: self.phase(),
            to: event.target_phase(),
        };
        let next = match event {
            RuntimeEvent::StartRequested { mode } => {
                if !self.is_stable() || self.active().is_some() {
                    return Err(rejected);
                }
                self.request(mode)
            }
            RuntimeEvent::ModeSwitchRequested { mode } => {
                if !self.is_stable() || self.active().is_none() {
                    return Err(rejected);
                }
                self.request(mode)
            }
            RuntimeEvent::StopRequested => {
                if !self.is_stable() {
                    return Err(rejected);
                }
                self.request(RuntimeMode::Direct)
            }
            RuntimeEvent::ProcessStarted { active } => match self {
                Self::Starting {
                    mode,
                    candidate: None,
                    previous_health,
                    previous_applied_mode,
                } => Self::Starting {
                    mode: *mode,
                    candidate: Some(active),
                    previous_health: *previous_health,
                    previous_applied_mode: *previous_applied_mode,
                },
                Self::Switching {
                    current,
                    mode,
                    candidate: None,
                } => Self::Switching {
                    current: current.clone(),
                    mode: *mode,
                    candidate: Some(active),
                },
                _ => return Err(rejected),
            },
            RuntimeEvent::HealthCheckPassed => match self {
                Self::Starting {
                    candidate: Some(active),
                    ..
                }
                | Self::Switching {
                    candidate: Some(active),
                    ..
                } => Self::Running {
                    active: active.clone(),
                },
                _ => return Err(rejected),
            },
            RuntimeEvent::RecoverySucceeded { explicit_stop } => match self {
                Self::Recovering { .. } => Self::Stopped {
                    applied_mode: (!explicit_stop).then_some(RuntimeMode::Direct),
                    last_error: None,
                },
                _ => return Err(rejected),
            },
            RuntimeEvent::RecoveryFailed { error } => match self {
                Self::Recovering { .. } => self.failed(error),
                _ => return Err(rejected),
            },
            RuntimeEvent::OperationFailed { error } => self.failed(error),
            RuntimeEvent::RecoveryBlocked { error } => {
                if self.active().is_some() {
                    return Err(rejected);
                }
                Self::Failed {
                    error,
                    last_session: None,
                    applied_mode: self.applied_mode(),
                    health: SessionHealth::RecoveryRequired,
                }
            }
            RuntimeEvent::ProcessExited { recovery_error } => {
                if self.active().is_none() || !self.is_stable() {
                    return Err(rejected);
                }
                let health = if recovery_error.is_some() {
                    SessionHealth::RecoveryRequired
                } else {
                    SessionHealth::Exited
                };
                Self::Failed {
                    error: recovery_error
                        .map(|error| format!("内核异常或网络恢复失败：{error}"))
                        .unwrap_or_else(|| "受管内核意外退出，系统代理已恢复".into()),
                    last_session: None,
                    applied_mode: None,
                    health,
                }
            }
            RuntimeEvent::Rollback { previous } => {
                if self.is_stable() && !matches!(self, Self::Failed { .. }) || !previous.is_stable()
                {
                    return Err(rejected);
                }
                *previous
            }
            RuntimeEvent::SnapshotRestored { previous } => {
                if !self.is_stable() || !previous.is_stable() {
                    return Err(rejected);
                }
                *previous
            }
            RuntimeEvent::MetadataCommitted => {
                if !self.is_stable() {
                    return Err(rejected);
                }
                if let Some(active) = self.active() {
                    Self::Running {
                        active: active.clone(),
                    }
                } else if self.health() == SessionHealth::Inactive {
                    Self::Stopped {
                        applied_mode: self.applied_mode(),
                        last_error: None,
                    }
                } else {
                    self.clone()
                }
            }
        };
        next.verify_invariants()?;
        Ok(next)
    }

    fn is_stable(&self) -> bool {
        matches!(
            self,
            Self::Stopped { .. } | Self::Running { .. } | Self::Failed { .. }
        )
    }

    fn request(&self, mode: RuntimeMode) -> Self {
        if mode == RuntimeMode::Direct {
            Self::Recovering {
                failed_session: self.active().cloned(),
                applied_mode: self.applied_mode(),
                health: self.health(),
                attempt: 0,
            }
        } else if let Some(current) = self.active() {
            Self::Switching {
                current: current.clone(),
                mode,
                candidate: None,
            }
        } else {
            Self::Starting {
                mode,
                candidate: None,
                previous_health: self.health(),
                previous_applied_mode: self.applied_mode(),
            }
        }
    }

    fn failed(&self, error: String) -> Self {
        Self::Failed {
            error,
            last_session: self.active().cloned(),
            applied_mode: self.applied_mode(),
            health: self.health(),
        }
    }
}

impl RuntimeEvent {
    pub(super) fn target_phase(&self) -> RuntimePhase {
        match self {
            Self::StartRequested {
                mode: RuntimeMode::Direct,
            }
            | Self::ModeSwitchRequested {
                mode: RuntimeMode::Direct,
            }
            | Self::StopRequested => RuntimePhase::Recovering,
            Self::StartRequested { .. } | Self::ProcessStarted { .. } => RuntimePhase::Starting,
            Self::ModeSwitchRequested { .. } => RuntimePhase::Switching,
            Self::HealthCheckPassed => RuntimePhase::Running,
            Self::RecoverySucceeded { .. } => RuntimePhase::Stopped,
            Self::Rollback { previous } | Self::SnapshotRestored { previous } => previous.phase(),
            Self::MetadataCommitted => RuntimePhase::Running,
            _ => RuntimePhase::Failed,
        }
    }
}
