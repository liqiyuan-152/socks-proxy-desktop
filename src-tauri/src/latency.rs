use crate::{
    credentials::CredentialStore, error::AppError, models::RuntimeMode,
    sing_box_process::SingBoxProcess, store::ConfigurationStore,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Debug, Serialize)]
pub struct LatencyResult {
    pub latency_ms: u64,
}

pub struct LatencyTester {
    store: Arc<dyn ConfigurationStore>,
    credentials: Arc<dyn CredentialStore>,
    binary: PathBuf,
    expected_sha256: String,
    runtime_root: PathBuf,
}

impl LatencyTester {
    pub fn new(
        store: Arc<dyn ConfigurationStore>,
        credentials: Arc<dyn CredentialStore>,
        binary: PathBuf,
        expected_sha256: String,
        runtime_root: PathBuf,
    ) -> Self {
        Self {
            store,
            credentials,
            binary,
            expected_sha256,
            runtime_root,
        }
    }

    pub fn test(&self, id: &str) -> Result<LatencyResult, AppError> {
        let mut configuration = self.store.load()?;
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
                self.credentials
                    .get(id)?
                    .ok_or_else(|| AppError::unavailable("代理认证凭据缺失"))?,
            )
        } else {
            None
        };
        configuration.active_profile_id = Some(id.to_owned());
        // A latency probe needs only the selected profile, not unrelated enabled exits.
        configuration.profiles.retain(|profile| profile.id == id);
        let credentials = credential
            .map(|credential| HashMap::from([(id.to_owned(), credential)]))
            .unwrap_or_default();
        let mut process = SingBoxProcess::start(
            &self.binary,
            &self.expected_sha256,
            &self.runtime_root,
            &configuration,
            RuntimeMode::Global,
            &credentials,
        )?;
        let result = probe(&configuration.settings.latency_test_url, process.proxy_port);
        process.stop();
        result
    }
}

fn probe(url: &str, port: u16) -> Result<LatencyResult, AppError> {
    probe_with_timeout(url, port, Duration::from_secs(5))
}

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
