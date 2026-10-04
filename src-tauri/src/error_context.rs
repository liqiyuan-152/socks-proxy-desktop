//! 可序列化的错误身份与恢复建议，以及仅供本地诊断的脱敏堆栈。
use crate::error::AppError;
use serde::Serialize;
use std::{
    backtrace::Backtrace,
    time::{SystemTime, UNIX_EPOCH},
};

/// 前端可用的领域分类，不依赖用户消息中的文字。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum ErrorDomain {
    Proxy,
    Routing,
    Runtime,
    Storage,
    Credential,
    Validation,
    Application,
}

/// 每次失败的结构化身份；同一错误经过多层转换时保留身份。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ErrorContext {
    pub error_id: String,
    /// 本地错误因果链身份，跨领域转换保留，不包含业务输入。
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub trace_id: Option<String>,
    /// subscriber 分配的本进程 span 身份；未启用追踪时可以缺省。
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub span_id: Option<String>,
    /// UTC Unix 毫秒；时钟不可用时为 0，错误处理不得再次失败。
    pub timestamp_ms: u64,
    pub domain: ErrorDomain,
    /// 稳定的领域变体名称，例如 `not_found`。
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub operation: Option<String>,
    pub recovery_suggestion: String,
    /// 仅包含 Rust 符号名称的堆栈，不向 IPC 发送本机代码位置。
    #[serde(skip)]
    #[cfg_attr(test, ts(skip))]
    pub(crate) stack: Vec<String>,
}

impl AppError {
    /// 补充错误上下文，嵌套转换保留原有错误 ID、时间和根因分类。
    pub fn with_context(mut self, domain: ErrorDomain, kind: &'static str) -> Self {
        if self.context.is_none() {
            self.context = Some(Box::new(ErrorContext {
                error_id: uuid::Uuid::new_v4().to_string(),
                trace_id: Some(uuid::Uuid::new_v4().to_string()),
                span_id: tracing::Span::current()
                    .id()
                    .map(|id| format!("{:016x}", id.into_u64())),
                timestamp_ms: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|elapsed| elapsed.as_millis().min(u64::MAX as u128) as u64)
                    .unwrap_or(0),
                domain,
                kind: kind.into(),
                operation: None,
                recovery_suggestion: recovery_suggestion(&self.code).into(),
                stack: stack_symbols(&Backtrace::force_capture().to_string()),
            }));
        }
        self
    }
}

fn recovery_suggestion(code: &str) -> &'static str {
    match code {
        "proxy_not_found" | "not_found" => "请刷新代理列表并重新选择代理。",
        "proxy_in_use" => "请先取消默认代理选择，并修改引用此代理的规则。",
        "credential_error" => "请检查代理认证设置并重新填写凭据。",
        "validation_error" => "请检查标记的字段，修正后重试。",
        "china_rules_unavailable" => "请检查安装包中的国内直连规则集，修复安装后重试。",
        "configuration_recovery"
        | "rollback_failed"
        | "system_proxy_failed"
        | "runtime_invariant_violated" => {
            "请停止代理，检查运行时状态，并执行网络恢复；保留原有配置。"
        }
        "process_start_failed" | "core_process_error" | "runtime_error" => {
            "请检查代理配置和内核状态，确认本地代理端口未被占用后重试。"
        }
        "storage_error" => "请确认本地存储可写并有足够空间；保留数据并查看诊断信息。",
        "runtime_already_owned" => "请关闭另一运行中的应用实例后重试。",
        "invalid_transition" => "请等待当前操作结束，刷新状态后重试。",
        "unavailable" => "请检查当前平台能力和运行时状态。",
        _ => "请刷新状态后重试；若仍失败，请查看诊断信息。",
    }
}

/// Backtrace 的地址、文件路径和源码位置不会进入诊断记录。
fn stack_symbols(trace: &str) -> Vec<String> {
    trace
        .lines()
        .filter_map(|line| {
            let (index, symbol) = line.trim().split_once(':')?;
            if !index.chars().all(|c| c.is_ascii_digit()) || index.is_empty() {
                return None;
            }
            let symbol = symbol.trim();
            if symbol.contains(['/', '\\', '@']) || symbol.starts_with("0x") || symbol.len() > 256 {
                return None;
            }
            Some(symbol.to_owned())
        })
        .take(16)
        .collect()
}

#[cfg(test)]
#[path = "error_context_tests.rs"]
mod tests;
