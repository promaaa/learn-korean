//! Lesson progress of the Hangul primer and the keyboard course: one `settings` row per passed
//! lesson, `hangul_passed/<lesson id>` or `keyboard_passed/<lesson id>`.
//!
//! One row per lesson rather than a single list or highest index: sync keeps the latest
//! `updated_at` per key, so two devices passing different lessons would otherwise overwrite each
//! other. A pass is never undone, so per-lesson rows merge to the union of both devices.

use std::collections::HashSet;

use sqlx::SqlitePool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Course {
    /// The Hangul primer (Hangul screen).
    Hangul,
    /// The touch-typing course (Typing Gym).
    Keyboard,
}

impl Course {
    fn prefix(self) -> &'static str {
        match self {
            Course::Hangul => "hangul_passed/",
            Course::Keyboard => "keyboard_passed/",
        }
    }
}

/// Ids of the course's lessons passed on any device.
pub async fn passed_lessons(pool: &SqlitePool, course: Course) -> sqlx::Result<HashSet<String>> {
    let prefix = course.prefix();
    let keys: Vec<String> =
        sqlx::query_scalar("SELECT key FROM settings WHERE substr(key, 1, length(?1)) = ?1")
            .bind(prefix)
            .fetch_all(pool)
            .await?;
    Ok(keys
        .into_iter()
        .filter_map(|key| key.strip_prefix(prefix).map(str::to_owned))
        .collect())
}

/// Records a pass at `now_ms`; passing again keeps the first pass untouched.
pub async fn record_pass(
    pool: &SqlitePool,
    course: Course,
    lesson_id: &str,
    now_ms: i64,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, 'passed', ?) \
         ON CONFLICT (key) DO NOTHING",
    )
    .bind(format!("{}{lesson_id}", course.prefix()))
    .bind(now_ms)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Database, sync};

    const HANGUL: Course = Course::Hangul;

    fn set(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|&id| id.to_owned()).collect()
    }

    #[tokio::test]
    async fn passes_accumulate_per_course_and_ignore_other_settings() {
        let db = Database::in_memory().await.unwrap();
        let pool = db.pool();
        assert_eq!(passed_lessons(pool, HANGUL).await.unwrap(), set(&[]));
        crate::settings::set_focus(pool, korean_core::learning::Focus::All, 1)
            .await
            .unwrap();
        record_pass(pool, HANGUL, "basic-vowels", 2).await.unwrap();
        record_pass(pool, HANGUL, "consonants-1", 3).await.unwrap();
        record_pass(pool, HANGUL, "basic-vowels", 4).await.unwrap();
        record_pass(pool, Course::Keyboard, "fj-1", 5)
            .await
            .unwrap();
        assert_eq!(
            passed_lessons(pool, HANGUL).await.unwrap(),
            set(&["basic-vowels", "consonants-1"])
        );
        assert_eq!(
            passed_lessons(pool, Course::Keyboard).await.unwrap(),
            set(&["fj-1"])
        );
        // Passing again does not touch the row, so it gives sync nothing new to send.
        let at: i64 = sqlx::query_scalar(
            "SELECT updated_at FROM settings WHERE key = 'hangul_passed/basic-vowels'",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(at, 2);
    }

    #[tokio::test]
    async fn lessons_passed_on_two_devices_merge_to_both() {
        let tmp = tempfile::tempdir().unwrap();
        let (laptop, desktop) = (
            Database::open(&tmp.path().join("laptop/korean.db"), "0.7.0")
                .await
                .unwrap(),
            Database::open(&tmp.path().join("desktop/korean.db"), "0.7.0")
                .await
                .unwrap(),
        );
        let schema = laptop.schema_version();
        record_pass(laptop.pool(), HANGUL, "basic-vowels", 1_000)
            .await
            .unwrap();
        record_pass(desktop.pool(), HANGUL, "basic-vowels", 2_000)
            .await
            .unwrap();
        record_pass(desktop.pool(), HANGUL, "consonants-1", 3_000)
            .await
            .unwrap();
        record_pass(laptop.pool(), HANGUL, "consonants-2", 4_000)
            .await
            .unwrap();

        let (laptop_file, desktop_file) =
            (tmp.path().join("laptop.db"), tmp.path().join("desktop.db"));
        sync::export(laptop.pool(), schema, "laptop", &laptop_file, 5_000)
            .await
            .unwrap();
        sync::export(desktop.pool(), schema, "desktop", &desktop_file, 5_000)
            .await
            .unwrap();
        sync::import(laptop.pool(), schema, &desktop_file)
            .await
            .unwrap();
        sync::import(desktop.pool(), schema, &laptop_file)
            .await
            .unwrap();

        let all = set(&["basic-vowels", "consonants-1", "consonants-2"]);
        assert_eq!(passed_lessons(laptop.pool(), HANGUL).await.unwrap(), all);
        assert_eq!(passed_lessons(desktop.pool(), HANGUL).await.unwrap(), all);
    }
}
