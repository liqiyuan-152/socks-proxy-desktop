use serde::Serialize;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub fields: Vec<FieldError>,
    /// 领域边界补充的上下文；旧错误仍可只包含原有三个字段。
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub context: Option<Box<crate::error_context::ErrorContext>>,
}

impl AppError {
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "unavailable".into(),
            message: message.into(),
            fields: Vec::new(),
            context: None,
        }
    }

    pub fn validation(fields: Vec<FieldError>) -> Self {
        Self {
            code: "validation_error".into(),
            message: "配置包含无效字段".into(),
            fields,
            context: None,
        }
    }

    pub fn storage(message: impl Into<String>) -> Self {
        Self {
            code: "storage_error".into(),
            message: message.into(),
            fields: Vec::new(),
            context: None,
        }
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for AppError {}
