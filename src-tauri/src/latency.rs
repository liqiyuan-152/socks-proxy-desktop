pub use crate::latency_tasks::LatencyResult;
use crate::latency_tasks::{LatencyExecutor, LatencyInput};
#[cfg(test)]
use crate::models::{PersistedConfiguration, ProxyProfile};
#[cfg(test)]
use crate::{credentials::CredentialStore, store::ConfigurationStore};
use crate::{error::AppError, models::RuntimeMode, sing_box_process::SingBoxProcess};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

pub struct LatencyTester {
    #[cfg(test)]
    store: Option<Arc<dyn ConfigurationStore>>,
    #[cfg(test)]
    credentials: Option<Arc<dyn CredentialStore>>,
    binary: PathBuf,
    expected_sha256: String,
    runtime_root: PathBuf,
}

impl LatencyTester {
    #[cfg(test)]
    pub fn new(
        store: Arc<dyn ConfigurationStore>,
        credentials: Arc<dyn CredentialStore>,
        binary: PathBuf,
        expected_sha256: String,
        runtime_root: PathBuf,
    ) -> Self {
        Self {
            store: Some(store),
            credentials: Some(credentials),
            binary,
            expected_sha256,
            runtime_root,
        }
    }

    #[cfg(windows)]
    pub fn for_runtime(binary: PathBuf, expected_sha256: String, runtime_root: PathBuf) -> Self {
        Self {
            #[cfg(test)]
            store: None,
            #[cfg(test)]
            credentials: None,
            binary,
            expected_sha256,
            runtime_root,
        }
    }

    #[cfg(test)]
    pub fn test(&self, id: &str) -> Result<LatencyResult, AppError> {
        let configuration = self.store.as_ref().unwrap().load()?;
        let profile = configuration
            .profiles
            .iter()
            .find(|profile| profile.id == id)
            .ok_or_else(|| AppError::unavailable("代理档案不存在"))?;
        if !profile.enabled {
            return Err(AppError::unavailable("已停用的代理不能测试"));
        }
        let credential = if profile.authentication_enabled {
            Some(
                crate::credentials::read_profile_credential(
                    self.credentials.as_ref().unwrap().as_ref(),
                    profile,
                )?
                .ok_or_else(|| AppError::unavailable("代理认证凭据缺失"))?,
            )
        } else {
            None
        };
        let configuration = probe_configuration(&configuration, profile);
        let credentials = credential
            .map(|credential| HashMap::from([(id.to_owned(), credential)]))
            .unwrap_or_default();
        self.run(
            LatencyInput {
                profile_id: id.to_owned(),
                configuration_revision: 0,
                configuration,
                credential: credentials.into_values().next(),
            },
            Arc::new(std::sync::atomic::AtomicBool::new(false)),
        )
    }
}

impl LatencyExecutor for LatencyTester {
    fn run(
        &self,
        input: LatencyInput,
        cancelled: Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<LatencyResult, AppError> {
        use std::sync::atomic::Ordering;
        if cancelled.load(Ordering::Acquire) {
            return Err(AppError::unavailable("测速已取消"));
        }
        let credentials = input
            .credential
            .map(|secret| HashMap::from([(input.profile_id, secret)]))
            .unwrap_or_default();
        let mut process = SingBoxProcess::start_cancellable(
            &self.binary,
            &self.expected_sha256,
            &self.runtime_root,
            &input.configuration,
            RuntimeMode::Global,
            &credentials,
            &cancelled,
        )?;
        let result = probe_cancel(
            &input.configuration.settings.latency_test_url,
            process.proxy_port,
            &cancelled,
        );
        process.stop();
        drop(process);
        result
    }
}

#[cfg(test)]
fn probe_configuration(
    source: &PersistedConfiguration,
    profile: &ProxyProfile,
) -> PersistedConfiguration {
    // Probe only this exit. Runtime rules and presets belong to the managed session.
    PersistedConfiguration {
        profiles: vec![profile.clone()],
        active_profile_id: Some(profile.id.clone()),
        settings: source.settings.clone(),
        ..PersistedConfiguration::default()
    }
}

#[cfg(test)]
fn probe(url: &str, port: u16) -> Result<LatencyResult, AppError> {
    probe_with_timeout(url, port, Duration::from_secs(5))
}

#[cfg(test)]
fn probe_with_timeout(url: &str, port: u16, timeout: Duration) -> Result<LatencyResult, AppError> {
    let proxy = reqwest::Proxy::all(format!("http://127.0.0.1:{port}"))
        .map_err(|_| AppError::unavailable("无法配置测试代理"))?;
    let client = reqwest::blocking::Client::builder()
        .proxy(proxy)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .build()
        .map_err(|_| AppError::unavailable("无法创建测试请求"))?;
    let start = Instant::now();
    let response = client.get(url).send().map_err(|error| {
        if error.is_timeout() {
            AppError::unavailable("代理测试超时（5 秒）")
        } else {
            AppError::unavailable("代理连接或测试请求失败")
        }
    })?;
    validate_status(response.status().as_u16())?;
    Ok(LatencyResult {
        latency_ms: start.elapsed().as_millis() as u64,
    })
}

fn validate_status(status: u16) -> Result<(), AppError> {
    if (200..300).contains(&status) {
        Ok(())
    } else {
        Err(AppError::unavailable(format!("测试地址返回 HTTP {status}")))
    }
}

#[cfg(test)]
#[path = "latency_tests.rs"]
mod tests;

fn probe_cancel(
    url: &str,
    port: u16,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<LatencyResult, AppError> {
    use std::sync::atomic::Ordering;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| AppError::unavailable("无法创建测试请求"))?;
    let proxy = reqwest::Proxy::all(format!("http://127.0.0.1:{port}"))
        .map_err(|_| AppError::unavailable("无法配置测试代理"))?;
    let client = reqwest::Client::builder()
        .proxy(proxy)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|_| AppError::unavailable("无法创建测试请求"))?;
    let start = Instant::now();
    runtime.block_on(async {
        let request = client.get(url).send();
        tokio::pin!(request);
        loop {
            tokio::select! {
                response = &mut request => {
                    let response = response.map_err(|error| AppError::unavailable(if error.is_timeout() { "代理测试超时（5 秒）" } else { "代理连接或测试请求失败" }))?;
                    validate_status(response.status().as_u16())?;
                    return Ok(LatencyResult { latency_ms: start.elapsed().as_millis() as u64 });
                }
                _ = tokio::time::sleep(Duration::from_millis(20)) => {
                    if cancelled.load(Ordering::Acquire) { return Err(AppError::unavailable("测速已取消")); }
                }
            }
        }
    })
}
