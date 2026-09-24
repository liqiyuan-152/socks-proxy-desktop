use crate::error::AppError;
use serde::Serialize;
use serde_json::Value;
use std::net::IpAddr;

const RECENT_LIMIT: usize = 20;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ActiveConnection {
    pub id: String,
    pub started_at: String,
    pub target_host: String,
    pub target_port: u16,
    pub matched_rule: Option<String>,
    pub outbound_chain: Vec<String>,
}

impl ActiveConnection {
    pub fn copy_detail(&self) -> String {
        format!(
            "连接 ID: {}\n开始时间: {}\n目标: {}:{}\n命中规则: {}\n出口链: {}",
            self.id,
            self.started_at,
            self.target_host,
            self.target_port,
            self.matched_rule.as_deref().unwrap_or("未命中"),
            self.outbound_chain.join(" → "),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationStatus {
    Available,
    Degraded,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ActiveConnectionsSnapshot {
    pub status: ObservationStatus,
    pub active_count: Option<usize>,
    pub recent: Vec<ActiveConnection>,
    pub diagnostic: Option<String>,
    pub history_available: bool,
}

impl ActiveConnectionsSnapshot {
    pub fn degraded() -> Self {
        Self {
            status: ObservationStatus::Degraded,
            active_count: None,
            recent: Vec::new(),
            diagnostic: Some("内核活跃连接接口不可用，连接观测已降级".into()),
            history_available: false,
        }
    }

    pub fn copy_detail(&self, id: &str) -> Result<String, AppError> {
        self.recent
            .iter()
            .find(|item| item.id == id)
            .map(ActiveConnection::copy_detail)
            .ok_or_else(|| AppError::unavailable("活跃连接已结束或不在当前快照中"))
    }
}

#[cfg(test)]
pub trait ActiveConnectionSource: Send + Sync {
    fn connections_json(&self) -> Result<Value, AppError>;
}

#[cfg(test)]
pub fn read_connections(source: &dyn ActiveConnectionSource) -> ActiveConnectionsSnapshot {
    source
        .connections_json()
        .and_then(|value| parse_connections(&value))
        .unwrap_or_else(|_| ActiveConnectionsSnapshot::degraded())
}

pub fn parse_connections(value: &Value) -> Result<ActiveConnectionsSnapshot, AppError> {
    let items = value
        .get("connections")
        .and_then(Value::as_array)
        .ok_or_else(unverified_fields)?;
    // A missing or unsafe field degrades the whole snapshot: never show a
    // partial count as if it represented all active connections.
    let connections: Vec<ActiveConnection> = items
        .iter()
        .map(parse_connection)
        .collect::<Result<_, _>>()?;
    let count = connections.len();
    Ok(ActiveConnectionsSnapshot {
        status: ObservationStatus::Available,
        active_count: Some(count),
        recent: connections.into_iter().take(RECENT_LIMIT).collect(),
        diagnostic: None,
        history_available: false,
    })
}

fn parse_connection(value: &Value) -> Result<ActiveConnection, AppError> {
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(unverified_fields)?;
    if uuid::Uuid::parse_str(id).is_err() {
        return Err(unverified_fields());
    }
    let started_at = value
        .get("start")
        .and_then(Value::as_str)
        .ok_or_else(unverified_fields)?;
    if started_at.len() > 40
        || started_at.is_empty()
        || !started_at
            .bytes()
            .all(|c| c.is_ascii_digit() || matches!(c, b'T' | b'Z' | b':' | b'.' | b'+' | b'-'))
    {
        return Err(unverified_fields());
    }
    let metadata = value.get("metadata").ok_or_else(unverified_fields)?;
    let host = metadata
        .get("host")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .or_else(|| metadata.get("destinationIP").and_then(Value::as_str))
        .ok_or_else(unverified_fields)?;
    if !safe_host(host) {
        return Err(unverified_fields());
    }
    let port = metadata
        .get("destinationPort")
        .ok_or_else(unverified_fields)?;
    let port: u16 = match port {
        Value::String(text) => text.parse().map_err(|_| unverified_fields())?,
        Value::Number(number) => number
            .as_u64()
            .and_then(|n| u16::try_from(n).ok())
            .ok_or_else(unverified_fields)?,
        _ => return Err(unverified_fields()),
    };
    if port == 0 {
        return Err(unverified_fields());
    }
    let rule = value
        .get("rule")
        .and_then(Value::as_str)
        .ok_or_else(unverified_fields)?;
    if !safe_summary(rule) {
        return Err(unverified_fields());
    }
    let chain = value
        .get("chains")
        .and_then(Value::as_array)
        .ok_or_else(unverified_fields)?;
    let outbound_chain: Vec<String> = chain
        .iter()
        .map(|tag| match tag.as_str() {
            Some("selected-proxy" | "direct") => Ok(tag.as_str().unwrap().to_owned()),
            _ => Err(unverified_fields()),
        })
        .collect::<Result<_, _>>()?;
    if outbound_chain.is_empty() {
        return Err(unverified_fields());
    }
    Ok(ActiveConnection {
        id: id.into(),
        started_at: started_at.into(),
        target_host: host.into(),
        target_port: port,
        matched_rule: (!rule.is_empty()).then(|| rule.into()),
        outbound_chain,
    })
}

fn safe_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && (host.parse::<IpAddr>().is_ok()
            || host
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-'))
}

fn safe_summary(summary: &str) -> bool {
    summary.len() <= 256
        && !summary.contains("://")
        && summary.bytes().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(
                    c,
                    b' ' | b'-' | b'_' | b'.' | b':' | b'=' | b'>' | b'(' | b')' | b'/'
                )
        })
        && ![
            "password",
            "secret",
            "token",
            "credential",
            "api_key",
            "authorization",
        ]
        .iter()
        .any(|word| summary.to_ascii_lowercase().contains(word))
}

fn unverified_fields() -> AppError {
    AppError::unavailable("内核活跃连接字段不可验证")
}

#[cfg(test)]
#[path = "observability_tests.rs"]
mod tests;
