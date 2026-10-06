use super::{mode_schema, storage_error};
use crate::{error::AppError, models::RuntimeMode};
use rusqlite::{Connection, OptionalExtension};
use serde_json::{json, Value};

pub(super) fn upgrade(connection: &Connection) -> Result<(), AppError> {
    tracing::info!(from = 6, to = 7, "开始迁移代理模式数据库");
    let selected: Option<String> = connection
        .query_row("SELECT mode FROM selected_mode WHERE id = 1", [], |row| {
            row.get(0)
        })
        .optional()
        .map_err(storage_error)?;
    let selected = selected.as_deref().unwrap_or("direct");
    if !matches!(selected, "rules" | "global" | "direct") {
        return Err(AppError::storage("迁移前的代理模式无效"));
    }
    let document: Option<String> = connection
        .query_row(
            "SELECT document_json FROM configuration WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage_error)?;
    let mut enabled = false;
    let mut converted = None;
    if let Some(document) = document {
        let mut value: Value = serde_json::from_str(&document)
            .map_err(|_| AppError::storage("迁移前的配置文档无效"))?;
        enabled = legacy_china(&value)?;
        convert_document(&mut value, selected)?;
        converted = Some(value.to_string());
    }
    mode_schema::create(connection)?;
    connection
        .execute(
            "INSERT INTO selected_mode_v7 VALUES (1, ?1, ?2, ?3)",
            rusqlite::params![
                selected,
                (selected == "rules").then_some(i32::from(enabled)),
                (selected == "rules").then_some("proxy")
            ],
        )
        .map_err(storage_error)?;
    if let Some(document) = converted {
        connection
            .execute(
                "UPDATE configuration SET document_json = ?1 WHERE id = 1",
                [document],
            )
            .map_err(storage_error)?;
    }
    // Recovery evidence must remain readable after upgrading. Never discard an
    // unfinished credential/startup transaction just to make migration succeed.
    let recovery: Option<String> = connection
        .query_row(
            "SELECT intent_json FROM configuration_recovery WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage_error)?;
    if let Some(recovery) = recovery {
        let mut value: Value = serde_json::from_str(&recovery)
            .map_err(|_| AppError::storage("迁移前的恢复记录无效"))?;
        for key in ["previous", "candidate"] {
            convert_document(
                value
                    .get_mut(key)
                    .ok_or_else(|| AppError::storage("恢复配置缺失"))?,
                selected,
            )?;
        }
        connection
            .execute(
                "UPDATE configuration_recovery SET intent_json = ?1 WHERE id = 1",
                [value.to_string()],
            )
            .map_err(storage_error)?;
    }
    connection
        .execute_batch(
            "DROP TABLE selected_mode; ALTER TABLE selected_mode_v7 RENAME TO selected_mode;",
        )
        .map_err(storage_error)?;
    tracing::info!(
        mode = selected,
        use_china_direct = enabled,
        default_action = "proxy",
        "代理模式迁移完成"
    );
    Ok(())
}

fn legacy_china(value: &Value) -> Result<bool, AppError> {
    match value.get("china_direct_enabled") {
        None => Ok(false),
        Some(Value::Bool(enabled)) => Ok(*enabled),
        _ => Err(AppError::storage("旧国内直连设置无效")),
    }
}

fn convert_document(value: &mut Value, selected: &str) -> Result<(), AppError> {
    let enabled = legacy_china(value)?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| AppError::storage("配置文档不是对象"))?;
    match object.get("schema_version").and_then(Value::as_u64) {
        Some(1 | 2) => {
            let mode = if selected == "rules" {
                json!({"rules": {"use_china_direct": enabled, "default_action": "proxy"}})
            } else {
                json!(selected)
            };
            object.insert("runtime_mode".into(), mode);
            // Existing v1 rule binding is handled by the configuration decoder.
            if object["schema_version"] == 2 {
                object.insert("schema_version".into(), json!(3));
            }
        }
        Some(3) => {
            serde_json::from_value::<RuntimeMode>(
                object
                    .get("runtime_mode")
                    .cloned()
                    .unwrap_or(json!("direct")),
            )
            .map_err(|_| AppError::storage("配置模式参数无效"))?;
        }
        _ => return Err(AppError::storage("迁移前的配置版本无效")),
    }
    object.remove("china_direct_enabled");
    let mut configuration: crate::models::PersistedConfiguration =
        serde_json::from_value(value.clone())
            .map_err(|_| AppError::storage("迁移后的配置结构无效"))?;
    configuration.migrate_v1()?;
    *value =
        serde_json::to_value(configuration).map_err(|_| AppError::storage("迁移配置无法序列化"))?;
    Ok(())
}

#[cfg(test)]
#[path = "store_mode_migration_tests.rs"]
mod tests;
