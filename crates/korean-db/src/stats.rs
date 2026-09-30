//! Review-log rows for `korean_core::stats`.

use korean_core::content::Skill;
use korean_core::stats::LoggedAnswer;
use sqlx::SqlitePool;

/// Every logged answer, oldest first. Rows for skills this build does not know are skipped, as in
/// `reviews::states`.
pub async fn answers(pool: &SqlitePool) -> sqlx::Result<Vec<LoggedAnswer>> {
    let rows: Vec<(String, String, bool, i64, i64)> = sqlx::query_as(
        "SELECT item_id, skill, correct, elapsed_ms, answered_at FROM review_log \
         ORDER BY answered_at, id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(item_id, skill, correct, elapsed_ms, answered_at)| {
            Some(LoggedAnswer {
                item_id,
                skill: Skill::parse(&skill)?,
                correct,
                elapsed_ms,
                answered_at,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    #[tokio::test]
    async fn answers_come_back_in_answer_order_without_unknown_skills() {
        let db = Database::in_memory().await.unwrap();
        let pool = db.pool();
        // Synced rows arrive out of order; ties keep insertion order.
        for (item, skill, correct, at) in [
            ("a/2", "response", 0, 3_000),
            ("a/1", "listening", 1, 1_000),
            ("a/1", "speaking", 1, 2_000),
            ("a/3", "build", 1, 3_000),
        ] {
            sqlx::query(
                "INSERT INTO review_log (item_id, skill, correct, rating, elapsed_ms, answered_at) \
                 VALUES (?, ?, ?, 3, 1500, ?)",
            )
            .bind(item)
            .bind(skill)
            .bind(correct)
            .bind(at)
            .execute(pool)
            .await
            .unwrap();
        }
        let rows = answers(pool).await.unwrap();
        let summary: Vec<(&str, Skill, bool, i64)> = rows
            .iter()
            .map(|a| (a.item_id.as_str(), a.skill, a.correct, a.answered_at))
            .collect();
        assert_eq!(
            summary,
            [
                ("a/1", Skill::Listening, true, 1_000),
                ("a/2", Skill::Response, false, 3_000),
                ("a/3", Skill::Build, true, 3_000),
            ]
        );
        assert!(rows.iter().all(|a| a.elapsed_ms == 1_500));
    }
}
