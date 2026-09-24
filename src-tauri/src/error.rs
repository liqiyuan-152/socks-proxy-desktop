use serde::Serialize;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub fields: Vec<FieldError>,
}

impl AppError {
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "unavailable".into(),
            message: message.into(),
            fields: Vec::new(),
        }
    }

    pub fn validation(fields: Vec<FieldError>) -> Self {
        Self {
            code: "validation_error".into(),
            message: "配置包含无效字段".into(),
            fields,
        }
    }

    pub fn storage(message: impl Into<String>) -> Self {
        Self {
            code: "storage_error".into(),
            message: message.into(),
            fields: Vec::new(),
        }
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for AppError {}
