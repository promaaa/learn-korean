//! Derived progress: total experience and practice days.

use sqlx::SqlitePool;

pub async fn total_xp(pool: &SqlitePool) -> sqlx::Result<u64> {
    let total: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(amount), 0) FROM xp_events")
        .fetch_one(pool)
        .await?;
    Ok(total.max(0) as u64)
}

/// Local day numbers (see `korean_core::progression::local_day`) with at least one correct
/// answer.
pub async fn practice_days(pool: &SqlitePool, utc_offset_seconds: i32) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar(
        "SELECT DISTINCT (answered_at + ? * 1000) / 86400000 FROM review_log \
         WHERE correct = 1 ORDER BY 1",
    )
    .bind(utc_offset_seconds)
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    #[tokio::test]
    async fn totals_and_days_are_derived_from_the_logs() {
        let db = Database::in_memory().await.unwrap();
        let pool = db.pool();
        assert_eq!(total_xp(pool).await.unwrap(), 0);
        let day = 86_400_000_i64;
        for (at, correct, xp) in [
            (3 * day + 10, 1, 15),
            (3 * day + 20, 0, 2),
            (5 * day - 1, 1, 10),
        ] {
            sqlx::query(
                "INSERT INTO review_log (item_id, skill, correct, rating, elapsed_ms, answered_at) \
                 VALUES ('a/1', 'listening', ?, 3, 1000, ?)",
            )
            .bind(correct)
            .bind(at)
            .execute(pool)
            .await
            .unwrap();
            sqlx::query(
                "INSERT INTO xp_events (earned_at, amount, item_id, skill) \
                 VALUES (?, ?, 'a/1', 'listening')",
            )
            .bind(at)
            .bind(xp)
            .execute(pool)
            .await
            .unwrap();
        }
        assert_eq!(total_xp(pool).await.unwrap(), 27);
        assert_eq!(practice_days(pool, 0).await.unwrap(), [3, 4]);
        // One hour ahead, the late answer of day 4 falls on day 5.
        assert_eq!(practice_days(pool, 3600).await.unwrap(), [3, 5]);
    }
}
