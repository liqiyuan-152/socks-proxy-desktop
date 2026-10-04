//! 保持 IPC 兼容的代理档案输入与视图。
use crate::{
    credentials::CredentialUpdate,
    models::{ProxyProfile, ProxyProtocol},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
/// 代理档案编辑输入；凭据通过显式更新操作提供。
pub struct ProfileInput {
    pub id: Option<String>,
    pub name: String,
    pub protocol: ProxyProtocol,
    pub host: String,
    pub port: u16,
    pub authentication_enabled: bool,
    pub enabled: bool,
    pub credential: Option<CredentialUpdate>,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
/// 不含凭据的档案视图，携带已提交配置版本。
pub struct ProfileView {
    pub configuration_revision: u64,
    pub id: String,
    pub name: String,
    pub protocol: ProxyProtocol,
    pub host: String,
    pub port: u16,
    pub authentication_enabled: bool,
    pub enabled: bool,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
/// 显式读取指定档案时返回的认证凭据。
pub struct ProfileCredentialView {
    pub username: String,
    pub password: String,
}

impl From<&ProxyProfile> for ProfileView {
    fn from(value: &ProxyProfile) -> Self {
        Self {
            configuration_revision: 0,
            id: value.id.clone(),
            name: value.name.clone(),
            protocol: value.protocol,
            host: value.host.clone(),
            port: value.port,
            authentication_enabled: value.authentication_enabled,
            enabled: value.enabled,
        }
    }
}
