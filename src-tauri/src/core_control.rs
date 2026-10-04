use crate::error::AppError;
use reqwest::{redirect::Policy, Client, StatusCode};
use std::{sync::Arc, time::Duration};

const RESPONSE_LIMIT: usize = 1024 * 1024;
const TOTAL_TIMEOUT: Duration = Duration::from_secs(2);

/// Never derive Debug or serialize this type: it owns the session control key.
#[derive(Clone)]
pub(crate) struct CoreControlClient {
    client: Client,
    executor: Arc<tokio::runtime::Runtime>,
    port: u16,
    secret: String,
}

impl CoreControlClient {
    pub(crate) fn new(port: u16, secret: String) -> Result<Self, AppError> {
        if port == 0 || secret.is_empty() {
            return Err(control_error("control_endpoint", "内核控制端点无效"));
        }
        let client = Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_millis(200))
            .timeout(TOTAL_TIMEOUT)
            .build()
            .map_err(|_| control_error("control_unavailable", "无法建立内核控制客户端"))?;
        let executor = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|_| control_error("control_unavailable", "无法建立内核控制执行器"))?;
        Ok(Self {
            executor: Arc::new(executor),
            client,
            port,
            secret,
        })
    }

    pub(crate) fn connections(&self) -> Result<serde_json::Value, AppError> {
        self.executor.block_on(async {
            tokio::time::timeout(TOTAL_TIMEOUT, self.read_connections())
                .await
                .map_err(|_| control_error("control_timeout", "内核控制请求超时"))?
        })
    }

    async fn read_connections(&self) -> Result<serde_json::Value, AppError> {
        let mut response = self
            .client
            .get(format!("http://127.0.0.1:{}/connections", self.port))
            .bearer_auth(&self.secret)
            .send()
            .await
            .map_err(request_error)?;
        if response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::FORBIDDEN
        {
            return Err(control_error(
                "control_authentication",
                "内核控制接口认证失败",
            ));
        }
        if response.status() != StatusCode::OK {
            return Err(control_error("control_status", "内核控制接口返回异常状态"));
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(request_error)? {
            if chunk.len() > RESPONSE_LIMIT - body.len() {
                return Err(control_error(
                    "control_response_limit",
                    "内核控制响应超过大小限制",
                ));
            }
            body.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&body)
            .map_err(|_| control_error("control_json", "内核控制响应 JSON 无效"))
    }
}

fn request_error(error: reqwest::Error) -> AppError {
    if error.is_timeout() {
        control_error("control_timeout", "内核控制请求超时")
    } else {
        control_error("control_unavailable", "内核控制接口不可访问")
    }
}

fn control_error(code: &str, message: &str) -> AppError {
    AppError {
        code: code.into(),
        message: message.into(),
        fields: Vec::new(),
        context: None,
    }
}

#[cfg(test)]
#[path = "core_control_tests.rs"]
mod tests;
