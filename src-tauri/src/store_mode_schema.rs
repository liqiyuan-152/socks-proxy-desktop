use crate::error::AppError;
use rusqlite::Connection;

pub(super) const SCHEMA: &str = "CREATE TABLE selected_mode_v7 (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    mode TEXT NOT NULL CHECK (mode IN ('rules', 'global', 'direct')),
    rules_use_china_direct INTEGER,
    rules_default_action TEXT,
    CHECK ((mode = 'rules' AND rules_use_china_direct IS NOT NULL
        AND rules_use_china_direct IN (0, 1) AND rules_default_action IS NOT NULL
        AND rules_default_action IN ('proxy', 'direct'))
        OR (mode != 'rules' AND rules_use_china_direct IS NULL AND rules_default_action IS NULL))
);";

pub(super) fn create(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute_batch(SCHEMA)
        .map_err(super::storage_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_enforces_mode_parameter_combinations() {
        let connection = Connection::open_in_memory().unwrap();
        create(&connection).unwrap();
        for mode in ["rules", "global", "direct"] {
            for china in [None, Some(0), Some(1), Some(2)] {
                for action in [None, Some("proxy"), Some("direct"), Some("reject")] {
                    let valid = if mode == "rules" {
                        matches!(china, Some(0 | 1)) && matches!(action, Some("proxy" | "direct"))
                    } else {
                        china.is_none() && action.is_none()
                    };
                    assert_eq!(
                        connection
                            .execute(
                                "INSERT OR REPLACE INTO selected_mode_v7 VALUES (1, ?1, ?2, ?3)",
                                rusqlite::params![mode, china, action],
                            )
                            .is_ok(),
                        valid,
                        "{mode} {china:?} {action:?}"
                    );
                }
            }
        }
    }
}
