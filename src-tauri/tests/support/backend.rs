use socks_proxy_lib::services::{
    adapters::{BackendSession, PersistedConfiguration, RuntimeBackend},
    AppError, RuntimeMode,
};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct BackendControl {
    pub sessions_started: u32,
    pub exited: bool,
    pub fail_start: bool,
    pub fail_restore: bool,
    pub confirmations: usize,
    pub reversions: usize,
    pub restorations: usize,
}
#[derive(Clone, Default)]
pub struct MockBackend(pub Arc<Mutex<BackendControl>>);
impl RuntimeBackend for MockBackend {
    fn transition(
        &self,
        _: Option<&BackendSession>,
        _: &PersistedConfiguration,
        mode: RuntimeMode,
        revision: u64,
    ) -> Result<Option<BackendSession>, AppError> {
        let mut control = self
            .0
            .lock()
            .map_err(|_| AppError::unavailable("测试后端锁不可用"))?;
        if control.fail_start {
            return Err(AppError::unavailable("模拟启动失败"));
        }
        if mode == RuntimeMode::Direct {
            if control.fail_restore {
                return Err(AppError::unavailable("模拟系统代理恢复失败"));
            }
            control.restorations += 1;
            return Ok(None);
        }
        control.sessions_started += 1;
        control.exited = false;
        Ok(Some(BackendSession {
            run_id: format!("mock-{}", control.sessions_started),
            process_id: control.sessions_started,
            configuration_revision: revision,
            system_proxy_enabled: true,
            tun_enabled: false,
        }))
    }
    fn confirm_transition(&self, _: Option<&BackendSession>) {
        if let Ok(mut control) = self.0.lock() {
            control.confirmations += 1;
        }
    }
    fn revert_transition(
        &self,
        _: Option<&BackendSession>,
        _: Option<&BackendSession>,
    ) -> Result<(), AppError> {
        self.0
            .lock()
            .map_err(|_| AppError::unavailable("测试后端锁不可用"))?
            .reversions += 1;
        Ok(())
    }
    fn reconcile_session(&self, _: &BackendSession) -> Result<bool, AppError> {
        let mut control = self
            .0
            .lock()
            .map_err(|_| AppError::unavailable("测试后端锁不可用"))?;
        if control.fail_restore {
            return Err(AppError::unavailable("模拟系统代理恢复失败"));
        }
        if control.exited {
            control.restorations += 1;
            return Ok(false);
        }
        Ok(true)
    }
    fn restore_network(&self) -> Result<(), AppError> {
        let mut control = self
            .0
            .lock()
            .map_err(|_| AppError::unavailable("测试后端锁不可用"))?;
        if control.fail_restore {
            return Err(AppError::unavailable("模拟系统代理恢复失败"));
        }
        control.restorations += 1;
        Ok(())
    }
}
