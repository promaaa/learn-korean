//! Learner preferences.

use korean_core::learning::Focus;
use sqlx::SqlitePool;

const FOCUS: &str = "focus";

/// The chosen focus; the default when never chosen or unknown to this release.
pub async fn focus(pool: &SqlitePool) -> sqlx::Result<Focus> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(FOCUS)
        .fetch_optional(pool)
        .await?;
    Ok(value.as_deref().and_then(Focus::parse).unwrap_or_default())
}

/// `now_ms` stamps the change so the latest choice wins when devices sync.
pub async fn set_focus(pool: &SqlitePool, focus: Focus, now_ms: i64) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?) \
         ON CONFLICT (key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(FOCUS)
    .bind(focus.as_str())
    .bind(now_ms)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    #[tokio::test]
    async fn focus_defaults_to_guided_and_keeps_the_last_choice() {
        let db = Database::in_memory().await.unwrap();
        let pool = db.pool();
        assert_eq!(focus(pool).await.unwrap(), Focus::Guided);
        set_focus(pool, Focus::Words, 1).await.unwrap();
        set_focus(pool, Focus::All, 2).await.unwrap();
        assert_eq!(focus(pool).await.unwrap(), Focus::All);
        // A value written by a newer release falls back to the default.
        sqlx::query("UPDATE settings SET value = 'grammar' WHERE key = 'focus'")
            .execute(pool)
            .await
            .unwrap();
        assert_eq!(focus(pool).await.unwrap(), Focus::Guided);
    }
}
