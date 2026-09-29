//! Learner preferences and FSRS parameters. Each key syncs between devices; the latest change
//! wins (`updated_at`).

use korean_core::learning::Focus;
use sqlx::SqlitePool;

const FOCUS: &str = "focus";
const FSRS: &str = "fsrs";

/// FSRS parameters optimized from the review log.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Fsrs {
    pub parameters: Vec<f32>,
    /// Size of the review log they were fitted on.
    pub reviews: u64,
    /// When they were fitted (unix milliseconds).
    #[serde(skip)]
    pub optimized_at: i64,
}

async fn get(pool: &SqlitePool, key: &str) -> sqlx::Result<Option<(String, i64)>> {
    sqlx::query_as("SELECT value, updated_at FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
}

/// `now_ms` stamps the change so the latest one wins when devices sync.
async fn set(pool: &SqlitePool, key: &str, value: &str, now_ms: i64) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?) \
         ON CONFLICT (key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(key)
    .bind(value)
    .bind(now_ms)
    .execute(pool)
    .await?;
    Ok(())
}

/// The chosen focus; the default when never chosen or unknown to this release.
pub async fn focus(pool: &SqlitePool) -> sqlx::Result<Focus> {
    let value = get(pool, FOCUS).await?;
    Ok(value
        .and_then(|(v, _)| Focus::parse(&v))
        .unwrap_or_default())
}

pub async fn set_focus(pool: &SqlitePool, focus: Focus, now_ms: i64) -> sqlx::Result<()> {
    set(pool, FOCUS, focus.as_str(), now_ms).await
}

/// Optimized FSRS parameters; `None` until the first optimization, or when unreadable.
pub async fn fsrs(pool: &SqlitePool) -> sqlx::Result<Option<Fsrs>> {
    Ok(get(pool, FSRS).await?.and_then(|(value, updated_at)| {
        let fsrs: Fsrs = serde_json::from_str(&value).ok()?;
        Some(Fsrs {
            optimized_at: updated_at,
            ..fsrs
        })
    }))
}

pub async fn set_fsrs(pool: &SqlitePool, fsrs: &Fsrs) -> sqlx::Result<()> {
    let value = serde_json::to_string(fsrs).expect("FSRS settings serialize");
    set(pool, FSRS, &value, fsrs.optimized_at).await
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

    #[tokio::test]
    async fn fsrs_parameters_round_trip_and_unreadable_values_are_ignored() {
        let db = Database::in_memory().await.unwrap();
        let pool = db.pool();
        assert_eq!(fsrs(pool).await.unwrap(), None);
        let fitted = Fsrs {
            parameters: vec![0.25, 1.5, 2.0],
            reviews: 420,
            optimized_at: 7,
        };
        set_fsrs(pool, &fitted).await.unwrap();
        assert_eq!(fsrs(pool).await.unwrap(), Some(fitted));
        sqlx::query("UPDATE settings SET value = '{' WHERE key = 'fsrs'")
            .execute(pool)
            .await
            .unwrap();
        assert_eq!(fsrs(pool).await.unwrap(), None);
    }
}
