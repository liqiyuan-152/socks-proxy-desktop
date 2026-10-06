use super::{storage_error, AppError, Connection, OptionalExtension};
use crate::models::{RuleAction, RuntimeMode};

pub(super) fn read(connection: &Connection) -> Result<RuntimeMode, AppError> {
    let row: Option<(String, Option<i64>, Option<String>)> = connection.query_row(
        "SELECT mode, rules_use_china_direct, rules_default_action FROM selected_mode WHERE id = 1", [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).optional().map_err(storage_error)?;
    match row {
        None => Ok(RuntimeMode::Direct),
        Some((mode, None, None)) if mode == "global" => Ok(RuntimeMode::Global),
        Some((mode, None, None)) if mode == "direct" => Ok(RuntimeMode::Direct),
        Some((mode, Some(china @ (0 | 1)), Some(action))) if mode == "rules" => {
            let default_action = match action.as_str() {
                "proxy" => RuleAction::Proxy,
                "direct" => RuleAction::Direct,
                _ => return Err(AppError::storage("已保存的默认动作无效")),
            };
            Ok(RuntimeMode::Rules {
                use_china_direct: china == 1,
                default_action,
            })
        }
        _ => Err(AppError::storage("已保存的代理模式参数无效")),
    }
}

pub(super) fn write(connection: &Connection, mode: RuntimeMode) -> Result<(), AppError> {
    let (name, china, action) = match mode {
        RuntimeMode::Rules {
            use_china_direct,
            default_action,
        } => (
            "rules",
            Some(use_china_direct),
            Some(match default_action {
                RuleAction::Proxy => "proxy",
                RuleAction::Direct => "direct",
            }),
        ),
        RuntimeMode::Global => ("global", None, None),
        RuntimeMode::Direct => ("direct", None, None),
    };
    connection.execute(
        "INSERT INTO selected_mode (id, mode, rules_use_china_direct, rules_default_action) VALUES (1, ?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE SET mode = excluded.mode, rules_use_china_direct = excluded.rules_use_china_direct,
         rules_default_action = excluded.rules_default_action",
        rusqlite::params![name, china, action],
    ).map_err(storage_error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{PersistedConfiguration, ProxyProfile, ProxyProtocol},
        store::{ConfigurationStore, SqliteConfigurationStore},
    };

    #[test]
    fn parameterized_modes_round_trip_and_clear_non_rules_columns() {
        let store = SqliteConfigurationStore::open_in_memory().unwrap();
        let mut config = PersistedConfiguration::default();
        config.profiles.push(ProxyProfile {
            id: "default".into(),
            name: "Default".into(),
            protocol: ProxyProtocol::Socks5,
            host: "proxy.example.com".into(),
            port: 1080,
            authentication_enabled: false,
            credential_ref: None,
            enabled: true,
        });
        config.active_profile_id = Some("default".into());
        for china in [false, true] {
            for action in [RuleAction::Proxy, RuleAction::Direct] {
                config.runtime_mode = RuntimeMode::Rules {
                    use_china_direct: china,
                    default_action: action,
                };
                store.save(&config).unwrap();
                assert_eq!(store.load().unwrap(), config);
                assert_eq!(store.load_mode().unwrap(), config.runtime_mode);
            }
        }
        for mode in [RuntimeMode::Global, RuntimeMode::Direct] {
            store.save_mode(mode).unwrap();
            assert_eq!(store.load_mode().unwrap(), mode);
            let connection = store.connection.lock().unwrap();
            let columns: (Option<i32>, Option<String>) = connection
                .query_row(
                    "SELECT rules_use_china_direct, rules_default_action FROM selected_mode",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            assert_eq!(columns, (None, None));
            let document: String = connection
                .query_row("SELECT document_json FROM configuration", [], |r| r.get(0))
                .unwrap();
            assert!(!document.contains("china_direct_enabled"));
        }
    }
}

#[cfg(test)]
mod corruption_tests {
    use super::*;

    #[test]
    fn decoder_rejects_invalid_or_non_rules_parameters_even_if_checks_were_bypassed() {
        let connection = Connection::open_in_memory().unwrap();
        super::super::migrations::migrate(&connection).unwrap();
        connection
            .execute_batch("PRAGMA ignore_check_constraints = ON;")
            .unwrap();
        for (mode, china, action) in [
            ("rules", None, Some("proxy")),
            ("rules", Some(1), None),
            ("rules", Some(2), Some("proxy")),
            ("rules", Some(0), Some("reject")),
            ("global", Some(0), Some("proxy")),
            ("direct", None, Some("direct")),
        ] {
            connection.execute("UPDATE selected_mode SET mode = ?1, rules_use_china_direct = ?2, rules_default_action = ?3",
                rusqlite::params![mode, china, action]).unwrap();
            assert!(read(&connection).is_err(), "{mode} {china:?} {action:?}");
        }
    }
}
