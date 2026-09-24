use crate::{
    credentials::CredentialStore,
    error::AppError,
    models::{PersistedConfiguration, RuntimeMode},
    runtime::{BackendSession, RuntimeBackend},
    sing_box_process::SingBoxProcess,
    system_proxy::SystemProxyAdapter,
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

struct CoreSession {
    identity: BackendSession,
    process: SingBoxProcess,
}

#[derive(Default)]
struct CoreSlots {
    active: Option<CoreSession>,
    pending: Option<CoreSession>,
    pending_direct: bool,
}

pub struct SingBoxRuntimeBackend {
    binary: PathBuf,
    expected_sha256: String,
    runtime_root: PathBuf,
    credentials: Arc<dyn CredentialStore>,
    system_proxy: Box<dyn SystemProxyAdapter>,
    slots: Mutex<CoreSlots>,
}

impl SingBoxRuntimeBackend {
    pub fn new(
        binary: PathBuf,
        expected_sha256: String,
        runtime_root: PathBuf,
        credentials: Arc<dyn CredentialStore>,
        system_proxy: Box<dyn SystemProxyAdapter>,
    ) -> Self {
        Self {
            binary,
            expected_sha256,
            runtime_root,
            credentials,
            system_proxy,
            slots: Mutex::new(CoreSlots::default()),
        }
    }

    pub fn active_connections_json(&self) -> Result<serde_json::Value, AppError> {
        let mut slots = self.slots.lock().map_err(|_| backend_error())?;
        let Some(active) = slots.active.as_mut() else {
            return Err(AppError::unavailable("内核未运行，无法读取活跃连接"));
        };
        active.process.connections_json()
    }
}

impl RuntimeBackend for SingBoxRuntimeBackend {
    fn restore_network(&self) -> Result<(), AppError> {
        let slots = self.slots.lock().map_err(|_| backend_error())?;
        if slots.active.is_some() || slots.pending.is_some() || slots.pending_direct {
            return Err(AppError::unavailable(
                "内核运行时仍在切换，暂不能单独恢复网络",
            ));
        }
        self.system_proxy.restore_if_owned()
    }
    fn transition(
        &self,
        previous: Option<&BackendSession>,
        candidate: &PersistedConfiguration,
        mode: RuntimeMode,
        revision: u64,
    ) -> Result<Option<BackendSession>, AppError> {
        let mut slots = self.slots.lock().map_err(|_| backend_error())?;
        if slots.pending.is_some()
            || slots.pending_direct
            || slots.active.as_ref().map(|run| &run.identity) != previous
        {
            return Err(backend_error());
        }
        if mode == RuntimeMode::Direct {
            self.system_proxy.restore_if_owned()?;
            slots.pending_direct = true;
            return Ok(None);
        }
        let profile = candidate
            .profiles
            .iter()
            .find(|profile| Some(&profile.id) == candidate.active_profile_id.as_ref())
            .ok_or_else(backend_error)?;
        let credential = if profile.authentication_enabled {
            self.credentials.get(&profile.id)?
        } else {
            None
        };
        let process = SingBoxProcess::start(
            &self.binary,
            &self.expected_sha256,
            &self.runtime_root,
            candidate,
            mode,
            credential.as_ref(),
        )?;
        // The adapter must leave the old setting intact on failure, including a
        // previous app-owned setting during a hot switch.
        self.system_proxy.enable(process.proxy_port)?;
        let identity = BackendSession {
            run_id: uuid::Uuid::new_v4().to_string(),
            process_id: process.process_id(),
            configuration_revision: revision,
            system_proxy_enabled: true,
            tun_enabled: false,
        };
        slots.pending = Some(CoreSession {
            identity: identity.clone(),
            process,
        });
        Ok(Some(identity))
    }

    fn confirm_transition(&self, previous: Option<&BackendSession>) {
        let mut slots = self
            .slots
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if slots.active.as_ref().map(|run| &run.identity) != previous {
            return;
        }
        if slots.pending_direct {
            slots.pending_direct = false;
            slots.active.take();
        } else if let Some(candidate) = slots.pending.take() {
            slots.active.replace(candidate);
        }
    }

    fn revert_transition(
        &self,
        candidate: Option<&BackendSession>,
        previous: Option<&BackendSession>,
    ) -> Result<(), AppError> {
        let mut slots = self.slots.lock().map_err(|_| backend_error())?;
        if slots.active.as_ref().map(|run| &run.identity) != previous
            || slots.pending.as_ref().map(|run| &run.identity) != candidate
        {
            return Err(backend_error());
        }
        if let Some(active) = slots.active.as_mut() {
            if !active.process.is_running()? {
                return Err(AppError::unavailable("先前内核已退出，无法恢复代理路径"));
            }
            self.system_proxy.enable(active.process.proxy_port)?;
        } else {
            self.system_proxy.restore_if_owned()?;
        }
        slots.pending.take();
        slots.pending_direct = false;
        Ok(())
    }

    fn reconcile_session(&self, session: &BackendSession) -> Result<bool, AppError> {
        let mut slots = self.slots.lock().map_err(|_| backend_error())?;
        let Some(active) = slots.active.as_mut() else {
            return Ok(false);
        };
        if &active.identity != session {
            return Err(backend_error());
        }
        if active.process.is_running()? {
            return Ok(true);
        }
        // Never leave an app-owned system proxy pointing at an exited process.
        let restoration = self.system_proxy.restore_if_owned();
        slots.active.take();
        restoration.map(|()| false)
    }

    fn active_connections_json(&self) -> Result<serde_json::Value, AppError> {
        SingBoxRuntimeBackend::active_connections_json(self)
    }
}

impl Drop for SingBoxRuntimeBackend {
    fn drop(&mut self) {
        let _ = self.system_proxy.restore_if_owned();
        if let Ok(mut slots) = self.slots.lock() {
            slots.pending.take();
            slots.active.take();
        }
    }
}

fn backend_error() -> AppError {
    AppError {
        code: "runtime_error".into(),
        message: "受管内核会话状态不一致".into(),
        fields: Vec::new(),
    }
}

#[cfg(test)]
#[path = "sing_box_backend_tests.rs"]
mod tests;
