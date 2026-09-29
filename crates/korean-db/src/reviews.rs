//! Memory states and the review log.

use std::collections::HashMap;

use korean_core::content::Skill;
use korean_core::learning::{Card, Outcome};
use korean_core::scheduler::MemoryState;
use sqlx::SqlitePool;

type StateRow = (String, String, i64, i64, i64, i64, i64);

/// All memory states. Rows for skills this build does not know are ignored, not deleted.
pub async fn states(pool: &SqlitePool) -> sqlx::Result<HashMap<Card, MemoryState>> {
    let rows: Vec<StateRow> = sqlx::query_as(
        "SELECT item_id, skill, step, due_at, reps, lapses, last_review_at FROM review_states",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(item_id, skill, step, due_at, reps, lapses, last)| {
            let card = Card::new(item_id, Skill::parse(&skill)?);
            let state = MemoryState {
                step: step as u32,
                due_at,
                reps: reps as u32,
                lapses: lapses as u32,
                last_review_at: last,
            };
            Some((card, state))
        })
        .collect())
}

/// Persists one answer: the new memory state and a review-log row, atomically.
pub async fn record(pool: &SqlitePool, outcome: &Outcome) -> sqlx::Result<()> {
    let Outcome {
        card,
        state,
        feedback,
        elapsed_ms,
    } = outcome;
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO review_states (item_id, skill, step, due_at, reps, lapses, last_review_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT (item_id, skill) DO UPDATE SET step = excluded.step, \
         due_at = excluded.due_at, reps = excluded.reps, lapses = excluded.lapses, \
         last_review_at = excluded.last_review_at",
    )
    .bind(&card.item_id)
    .bind(card.skill.as_str())
    .bind(state.step)
    .bind(state.due_at)
    .bind(state.reps)
    .bind(state.lapses)
    .bind(state.last_review_at)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO review_log (item_id, skill, correct, rating, elapsed_ms, answered_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&card.item_id)
    .bind(card.skill.as_str())
    .bind(feedback.correct)
    .bind(feedback.rating.value())
    .bind(*elapsed_ms as i64)
    .bind(state.last_review_at)
    .execute(&mut *tx)
    .await?;
    tx.commit().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use korean_core::learning::Feedback;
    use korean_core::scheduler::{Rating, review};

    fn outcome(item: &str, rating: Rating, previous: Option<&MemoryState>, now: i64) -> Outcome {
        Outcome {
            feedback: Feedback {
                correct: rating != Rating::Again,
                correct_index: Some(0),
                translations: vec![],
                rating,
                korean: String::new(),
                english: String::new(),
                note: None,
                streak: 0,
                retry: false,
            },
            card: Card::new(item, Skill::Listening),
            state: review(previous, rating, now),
            elapsed_ms: 1_500,
        }
    }

    #[tokio::test]
    async fn states_round_trip_and_every_answer_is_logged() {
        let db = Database::in_memory().await.unwrap();
        let pool = db.pool();
        let first = outcome("a/1", Rating::Good, None, 1_000);
        record(pool, &first).await.unwrap();
        let second = outcome("a/1", Rating::Again, Some(&first.state), 2_000);
        record(pool, &second).await.unwrap();

        let states = states(pool).await.unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(states[&Card::new("a/1", Skill::Listening)], second.state);

        let log: Vec<(i64, i64, i64)> =
            sqlx::query_as("SELECT correct, rating, answered_at FROM review_log ORDER BY id")
                .fetch_all(pool)
                .await
                .unwrap();
        assert_eq!(log, [(1, 3, 1_000), (0, 1, 2_000)]);
    }

    #[tokio::test]
    async fn unknown_skills_from_newer_builds_are_ignored() {
        let db = Database::in_memory().await.unwrap();
        sqlx::query("INSERT INTO review_states VALUES ('a/1', 'speaking', 0, 0, 1, 0, 0)")
            .execute(db.pool())
            .await
            .unwrap();
        assert!(states(db.pool()).await.unwrap().is_empty());
    }
}
