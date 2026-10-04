//! 只持久化连接数量，不保存目标地址、规则、凭据或连接 ID。
use super::{storage_error, SqliteConfigurationStore};
use crate::{error::AppError, observability::ConnectionTrend};
use rusqlite::params;

const RETENTION_MINUTES: i64 = 31 * 24 * 60;

pub(super) fn record(
    store: &SqliteConfigurationStore,
    count: usize,
    now_ms: i64,
) -> Result<Option<ConnectionTrend>, AppError> {
    if now_ms < 0 {
        return Err(AppError::storage("连接采样时间无效"));
    }
    let count = i64::try_from(count).map_err(|_| AppError::storage("连接采样数量超出范围"))?;
    let minute = now_ms / 60_000;
    let mut connection = store
        .connection
        .lock()
        .map_err(|_| AppError::storage("连接采样存储锁不可用"))?;
    let transaction = connection.transaction().map_err(storage_error)?;
    // 重复轮询或多窗口不会增加该分钟的权重。系统时钟回退也不会覆盖旧样本。
    transaction
        .execute(
            "INSERT OR IGNORE INTO connection_count_samples (minute, active_count) VALUES (?1, ?2)",
            params![minute, count],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "DELETE FROM connection_count_samples WHERE minute < ?1",
            [minute.saturating_sub(RETENTION_MINUTES)],
        )
        .map_err(storage_error)?;
    // 先转本地时间再减一天，跨夏令时仍以本地自然日为边界。
    let yesterday: String = transaction
        .query_row(
            "SELECT date(?1, 'unixepoch', 'localtime', '-1 day')",
            [now_ms / 1000],
            |row| row.get(0),
        )
        .map_err(storage_error)?;
    let (sum, samples): (u64, u32) = transaction
        .query_row(
            "SELECT COALESCE(SUM(active_count), 0), COUNT(*)
             FROM connection_count_samples
             WHERE minute BETWEEN ?1 AND ?2
               AND date(minute * 60, 'unixepoch', 'localtime') = ?3",
            params![minute.saturating_sub(3 * 24 * 60), minute, yesterday],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;
    Ok((samples > 0).then_some(ConnectionTrend {
        yesterday_date: yesterday,
        yesterday_count_sum: sum,
        yesterday_samples: samples,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::ConfigurationStore;

    fn yesterday_noon(store: &SqliteConfigurationStore, now: i64) -> i64 {
        store.connection.lock().unwrap().query_row(
            "SELECT CAST(strftime('%s', date(?1, 'unixepoch', 'localtime', '-1 day') || ' 12:00:00', 'utc') AS INTEGER) * 1000",
            [now / 1000], |row| row.get(0),
        ).unwrap()
    }

    #[test]
    fn persists_real_yesterday_samples_and_deduplicates_minutes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("samples.sqlite3");
        let store = SqliteConfigurationStore::open(&path).unwrap();
        let now = 1_791_072_000_000;
        let yesterday = yesterday_noon(&store, now);
        assert!(store.connection_trend(3, yesterday).unwrap().is_none());
        store.connection_trend(99, yesterday + 1_000).unwrap();
        store.connection_trend(5, yesterday + 60_000).unwrap();
        drop(store);
        let reopened = SqliteConfigurationStore::open(&path).unwrap();
        let trend = reopened.connection_trend(8, now).unwrap().unwrap();
        assert_eq!(trend.yesterday_count_sum, 8);
        assert_eq!(trend.yesterday_samples, 2);
    }

    #[test]
    fn zero_is_a_sample_but_missing_days_are_not_zero_and_old_data_is_pruned() {
        let store = SqliteConfigurationStore::open_in_memory().unwrap();
        let now = 1_791_072_000_000;
        let yesterday = yesterday_noon(&store, now);
        store.connection_trend(0, yesterday).unwrap();
        let trend = store.connection_trend(1, now).unwrap().unwrap();
        assert_eq!(trend.yesterday_count_sum, 0);
        assert_eq!(trend.yesterday_samples, 1);
        assert!(store
            .connection_trend(1, now + 3 * 86_400_000)
            .unwrap()
            .is_none());
        store.connection_trend(1, now + 40 * 86_400_000).unwrap();
        let count: u32 = store
            .connection
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM connection_count_samples", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
        assert!(store.connection_trend(1, -1).is_err());
    }
}
