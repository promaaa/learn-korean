//! Memory states and the review log.

use std::collections::HashMap;

use korean_core::content::Skill;
use korean_core::learning::{Card, Outcome};
use korean_core::scheduler::{Answered, MemoryState, Phase, Rating};
use sqlx::SqlitePool;

#[derive(sqlx::FromRow)]
struct StateRow {
    item_id: String,
    skill: String,
    phase: i64,
    stability: f64,
    difficulty: f64,
    due_at: i64,
    last_review_at: i64,
    scheduled_days: i64,
    reps: i64,
    lapses: i64,
    step: i64,
}

/// All memory states. Rows for skills this build does not know are ignored, not deleted.
pub async fn states(pool: &SqlitePool) -> sqlx::Result<HashMap<Card, MemoryState>> {
    let rows: Vec<StateRow> = sqlx::query_as(
        "SELECT item_id, skill, phase, stability, difficulty, due_at, last_review_at, \
         scheduled_days, reps, lapses, step FROM review_states",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let card = Card::new(row.item_id, Skill::parse(&row.skill)?);
            let state = MemoryState {
                phase: Phase::from_code(row.phase),
                stability: row.stability,
                difficulty: row.difficulty,
                due_at: row.due_at,
                last_review_at: row.last_review_at,
                scheduled_days: row.scheduled_days,
                reps: row.reps as u32,
                lapses: row.lapses as u32,
                // Rows from the interval ladder (schema 3) hold its rung, clamped as a step.
                step: row.step.max(0) as u32,
            };
            Some((card, state))
        })
        .collect())
}

/// Persists one answer atomically: the new memory state, a review-log row and the XP earned.
pub async fn record(pool: &SqlitePool, outcome: &Outcome) -> sqlx::Result<()> {
    let Outcome {
        card,
        state,
        feedback,
        elapsed_ms,
    } = outcome;
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO review_states (item_id, skill, step, phase, stability, difficulty, due_at, \
         last_review_at, scheduled_days, reps, lapses) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT (item_id, skill) DO UPDATE SET step = excluded.step, phase = excluded.phase, \
         stability = excluded.stability, difficulty = excluded.difficulty, \
         due_at = excluded.due_at, last_review_at = excluded.last_review_at, \
         scheduled_days = excluded.scheduled_days, reps = excluded.reps, lapses = excluded.lapses",
    )
    .bind(&card.item_id)
    .bind(card.skill.as_str())
    .bind(state.step)
    .bind(state.phase.code())
    .bind(state.stability)
    .bind(state.difficulty)
    .bind(state.due_at)
    .bind(state.last_review_at)
    .bind(state.scheduled_days)
    .bind(state.reps)
    .bind(state.lapses)
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
    sqlx::query("INSERT INTO xp_events (earned_at, amount, item_id, skill) VALUES (?, ?, ?, ?)")
        .bind(state.last_review_at)
        .bind(feedback.xp)
        .bind(&card.item_id)
        .bind(card.skill.as_str())
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

/// Every card's answers, oldest first: what FSRS replays and learns from.
pub async fn histories(pool: &SqlitePool) -> sqlx::Result<HashMap<Card, Vec<Answered>>> {
    let rows: Vec<(String, String, i64, i64)> = sqlx::query_as(
        "SELECT item_id, skill, rating, answered_at FROM review_log ORDER BY answered_at, id",
    )
    .fetch_all(pool)
    .await?;
    let mut histories: HashMap<Card, Vec<Answered>> = HashMap::new();
    for (item_id, skill, rating, answered_at) in rows {
        let (Some(skill), Some(rating)) = (Skill::parse(&skill), Rating::from_value(rating)) else {
            continue;
        };
        histories
            .entry(Card::new(item_id, skill))
            .or_default()
            .push((rating, answered_at));
    }
    Ok(histories)
}

/// Number of answers in the review log.
pub async fn count(pool: &SqlitePool) -> sqlx::Result<u64> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM review_log")
        .fetch_one(pool)
        .await?;
    Ok(n as u64)
}

/// Cards answered for the first time at or after `since_ms`: the new cards introduced since.
pub async fn introduced_since(pool: &SqlitePool, since_ms: i64) -> sqlx::Result<usize> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM (SELECT MIN(answered_at) AS first FROM review_log \
         GROUP BY item_id, skill) WHERE first >= ?",
    )
    .bind(since_ms)
    .fetch_one(pool)
    .await?;
    Ok(n as usize)
}

/// Replaces the stability and difficulty of these cards, e.g. once FSRS parameters changed.
/// Due dates are kept, as in Anki.
pub async fn set_memory(pool: &SqlitePool, updates: &[(Card, f64, f64)]) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    for (card, stability, difficulty) in updates {
        sqlx::query(
            "UPDATE review_states SET stability = ?, difficulty = ? WHERE item_id = ? AND skill = ?",
        )
        .bind(stability)
        .bind(difficulty)
        .bind(&card.item_id)
        .bind(card.skill.as_str())
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use korean_core::learning::Feedback;
    use korean_core::scheduler::Scheduler;

    fn outcome(item: &str, rating: Rating, previous: Option<&MemoryState>, now: i64) -> Outcome {
        Outcome {
            feedback: Feedback {
                correct: rating != Rating::Again,
                correct_index: Some(0),
                translations: vec![],
                glosses: Default::default(),
                rating,
                korean: String::new(),
                english: String::new(),
                note: None,
                streak: 0,
                retry: false,
                due_in_ms: 0,
                xp: 10,
            },
            card: Card::new(item, Skill::Listening),
            state: Scheduler::with_defaults(0).review(previous, rating, now, 0.5),
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
        assert_eq!(crate::progress::total_xp(pool).await.unwrap(), 20);
    }

    #[tokio::test]
    async fn histories_replay_answers_in_order_and_count_cards_introduced() {
        let db = Database::in_memory().await.unwrap();
        let pool = db.pool();
        let a1 = outcome("a/1", Rating::Good, None, 1_000);
        record(pool, &a1).await.unwrap();
        record(pool, &outcome("a/2", Rating::Easy, None, 5_000))
            .await
            .unwrap();
        record(pool, &outcome("a/1", Rating::Again, Some(&a1.state), 9_000))
            .await
            .unwrap();
        let histories = histories(pool).await.unwrap();
        assert_eq!(
            histories[&Card::new("a/1", Skill::Listening)],
            [(Rating::Good, 1_000), (Rating::Again, 9_000)]
        );
        assert_eq!(count(pool).await.unwrap(), 3);
        // a/1 was introduced before 2 000 and answered again after: only a/2 is new since.
        assert_eq!(introduced_since(pool, 2_000).await.unwrap(), 1);
        assert_eq!(introduced_since(pool, 0).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn unknown_skills_from_newer_builds_are_ignored() {
        let db = Database::in_memory().await.unwrap();
        sqlx::query("INSERT INTO review_states (item_id, skill, step, due_at, reps, lapses, last_review_at) VALUES ('a/1', 'speaking', 0, 0, 1, 0, 0)")
            .execute(db.pool())
            .await
            .unwrap();
        assert!(states(db.pool()).await.unwrap().is_empty());
    }
}
