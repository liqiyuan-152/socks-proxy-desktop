//! 纯运行时状态机：候选会话与已应用会话分离，转换不执行 I/O。
use crate::{
    domain_errors::RuntimeError,
    models::RuntimeMode,
    runtime::{BackendSession, RuntimePhase, SessionHealth},
};
use std::time::Instant;

/// 已经由后端准备完成的会话；只有提交事件才能使其成为已应用会话。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveSession {
    pub session: BackendSession,
    pub mode: RuntimeMode,
    pub started_at: Instant,
}

/// 一个完整生命周期节点，避免 phase/session/health 三组字段独立变化。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeStateNode {
    Stopped {
        applied_mode: Option<RuntimeMode>,
        last_error: Option<String>,
    },
    Starting {
        mode: RuntimeMode,
        candidate: Option<ActiveSession>,
        previous_health: SessionHealth,
        previous_applied_mode: Option<RuntimeMode>,
    },
    Running {
        active: ActiveSession,
    },
    Switching {
        current: ActiveSession,
        mode: RuntimeMode,
        candidate: Option<ActiveSession>,
    },
    Recovering {
        failed_session: Option<ActiveSession>,
        applied_mode: Option<RuntimeMode>,
        health: SessionHealth,
        attempt: u32,
    },
    Failed {
        error: String,
        last_session: Option<ActiveSession>,
        applied_mode: Option<RuntimeMode>,
        health: SessionHealth,
    },
}

/// 后端或存储完成 I/O 后显式通知状态机；请求事件本身不启动进程。
#[derive(Clone, Debug)]
pub enum RuntimeEvent {
    StartRequested {
        mode: RuntimeMode,
    },
    ModeSwitchRequested {
        mode: RuntimeMode,
    },
    ProcessStarted {
        active: ActiveSession,
    },
    HealthCheckPassed,
    ProcessExited {
        recovery_error: Option<String>,
    },
    RecoverySucceeded {
        explicit_stop: bool,
    },
    RecoveryFailed {
        error: String,
    },
    OperationFailed {
        error: String,
    },
    RecoveryBlocked {
        error: String,
    },
    StopRequested,
    /// 后端已成功撤销候选会话后恢复先前节点。
    Rollback {
        previous: Box<RuntimeStateNode>,
    },
    /// 已重新应用并确认旧配置后，恢复其快照元数据。
    SnapshotRestored {
        previous: Box<RuntimeStateNode>,
    },
    /// 不涉及运行计划的配置提交完成，清除操作错误。
    MetadataCommitted,
}

impl Default for RuntimeStateNode {
    fn default() -> Self {
        Self::Stopped {
            applied_mode: None,
            last_error: None,
        }
    }
}

impl RuntimeStateNode {
    /// 兼容现有 IPC 中的生命周期阶段。
    pub fn phase(&self) -> RuntimePhase {
        match self {
            Self::Stopped { .. } => RuntimePhase::Stopped,
            Self::Starting { .. } => RuntimePhase::Starting,
            Self::Running { .. } => RuntimePhase::Running,
            Self::Switching { .. } => RuntimePhase::Switching,
            Self::Recovering { .. } => RuntimePhase::Recovering,
            Self::Failed { .. } => RuntimePhase::Failed,
        }
    }

    /// 当前生效会话；候选会话不会提前暴露为已应用会话。
    pub fn active(&self) -> Option<&ActiveSession> {
        match self {
            Self::Running { active } => Some(active),
            Self::Switching { current, .. } => Some(current),
            Self::Recovering { failed_session, .. } => failed_session.as_ref(),
            Self::Failed { last_session, .. } => last_session.as_ref(),
            _ => None,
        }
    }

    /// 保留直连已应用与显式停止尚未应用的区别。
    pub fn applied_mode(&self) -> Option<RuntimeMode> {
        self.active().map(|active| active.mode).or(match self {
            Self::Stopped { applied_mode, .. }
            | Self::Recovering { applied_mode, .. }
            | Self::Failed { applied_mode, .. } => *applied_mode,
            Self::Starting {
                previous_applied_mode,
                ..
            } => *previous_applied_mode,
            _ => None,
        })
    }

    /// 失败并不意味着旧会话已退出。
    pub fn health(&self) -> SessionHealth {
        match self {
            Self::Running { .. } | Self::Switching { .. } => SessionHealth::Healthy,
            Self::Starting {
                previous_health, ..
            } => *previous_health,
            Self::Recovering { health, .. } | Self::Failed { health, .. } => *health,
            Self::Stopped { .. } => SessionHealth::Inactive,
        }
    }

    /// 可展示的生命周期错误，不包含内部会话数据。
    pub fn last_error(&self) -> Option<&str> {
        match self {
            Self::Stopped { last_error, .. } => last_error.as_deref(),
            Self::Failed { error, .. } => Some(error),
            _ => None,
        }
    }

    /// 校验会话身份、模式及健康状态之间的关系。
    pub fn verify_invariants(&self) -> Result<(), RuntimeError> {
        let fail = |message: &str| RuntimeError::InvariantViolated(message.into());
        let valid_session = |active: &ActiveSession| {
            if active.session.process_id == 0
                || active.session.run_id.is_empty()
                || active.mode == RuntimeMode::Direct
            {
                Err(fail("运行会话必须具有有效身份及代理模式"))
            } else {
                Ok(())
            }
        };
        if let Some(active) = self.active() {
            valid_session(active)?;
        }
        if (self.health() == SessionHealth::Healthy) != self.active().is_some() {
            return Err(fail("健康状态必须与已应用会话一致"));
        }
        if self.active().is_none()
            && self
                .applied_mode()
                .is_some_and(|mode| mode != RuntimeMode::Direct)
        {
            return Err(fail("没有已应用会话时不能声明代理模式已生效"));
        }
        match self {
            Self::Starting {
                mode, candidate, ..
            }
            | Self::Switching {
                mode, candidate, ..
            } => {
                if *mode == RuntimeMode::Direct {
                    return Err(fail("直连请求必须进入恢复状态"));
                }
                if let Some(candidate) = candidate {
                    valid_session(candidate)?;
                    if candidate.mode != *mode {
                        return Err(fail("候选会话模式必须匹配请求"));
                    }
                }
            }
            Self::Recovering {
                applied_mode,
                failed_session,
                ..
            }
            | Self::Failed {
                applied_mode,
                last_session: failed_session,
                ..
            } => {
                if let Some(active) = failed_session {
                    if *applied_mode != Some(active.mode) {
                        return Err(fail("保留会话与已应用模式不一致"));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "runtime_state_machine_tests.rs"]
mod tests;
#[path = "runtime_state_transitions.rs"]
mod transitions;

#[cfg(test)]
#[path = "runtime_state_event_matrix_tests.rs"]
mod event_matrix_tests;

#[cfg(test)]
#[path = "runtime_state_property_tests.rs"]
mod property_tests;
