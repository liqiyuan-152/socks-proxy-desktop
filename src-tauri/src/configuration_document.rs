use crate::{error::AppError, models::RuntimeMode};
use serde_json::{json, Value};

/// Normalize legacy portable/internal JSON before deserializing the v3 model.
/// An explicit selected mode from the database takes precedence for old docs.
pub(crate) fn normalize(json: &str, selected: Option<RuntimeMode>) -> Result<String, AppError> {
    let mut value: Value =
        serde_json::from_str(json).map_err(|_| AppError::storage("配置文档格式无效"))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| AppError::storage("配置文档不是对象"))?;
    match object.get("schema_version").and_then(Value::as_u64) {
        Some(1 | 2) => {
            let china = match object.remove("china_direct_enabled") {
                Some(Value::Bool(enabled)) => enabled,
                None => false,
                _ => return Err(AppError::storage("旧国内直连设置无效")),
            };
            if !object.contains_key("runtime_mode") {
                let mode = selected.unwrap_or(if china {
                    RuntimeMode::Rules {
                        use_china_direct: true,
                        default_action: crate::models::RuleAction::Proxy,
                    }
                } else {
                    RuntimeMode::Direct
                });
                object.insert("runtime_mode".into(), json!(mode));
            }
        }
        Some(3) => {
            if object.contains_key("china_direct_enabled") || !object.contains_key("runtime_mode") {
                return Err(AppError::storage("v3 配置必须使用参数化 runtime_mode"));
            }
        }
        _ => return Err(AppError::storage("不支持的配置文档版本")),
    }
    Ok(value.to_string())
}
