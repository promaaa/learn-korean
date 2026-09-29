//! FSRS parameters and memory states, maintained as Anki maintains them (ADR-003).
//!
//! - Parameters are fitted to the review log in the background at start, at most once a day and
//!   only when the log grew; they sync to the other devices like any setting.
//! - Every card's stability and difficulty are replayed from its answers when the parameters or
//!   the merged log may have changed: at start and after merging another device's progress. Due
//!   dates are kept.

use std::collections::HashMap;

use korean_core::learning::Card;
use korean_core::scheduler::{self, Answered, DAY_MS, Scheduler};
use korean_db::settings::Fsrs;
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

use crate::progress::utc_offset_seconds;
use crate::session::now_ms;
use crate::state::AppState;

/// Scheduler with the optimized parameters, or the defaults before the first optimization.
pub async fn scheduler(pool: &SqlitePool) -> Result<Scheduler, String> {
    let fsrs = korean_db::settings::fsrs(pool)
        .await
        .map_err(|e| e.to_string())?;
    let offset = utc_offset_seconds();
    Ok(match fsrs {
        Some(fsrs) => Scheduler::new(&fsrs.parameters, offset).unwrap_or_else(|err| {
            log::warn!("fsrs: {err}; using the default parameters");
            Scheduler::with_defaults(offset)
        }),
        None => Scheduler::with_defaults(offset),
    })
}

/// Rewrites the memory of every card whose replayed state differs from the stored one.
pub async fn replay(pool: &SqlitePool) -> Result<usize, String> {
    let err = |e: sqlx::Error| e.to_string();
    let scheduler = scheduler(pool).await?;
    let histories = korean_db::reviews::histories(pool).await.map_err(err)?;
    let states = korean_db::reviews::states(pool).await.map_err(err)?;
    let updates: Vec<(Card, f64, f64)> = states
        .into_iter()
        .filter_map(|(card, state)| {
            let (stability, difficulty) = scheduler.memory(histories.get(&card)?)?;
            let changed = (stability - state.stability).abs() > 1e-4
                || (difficulty - state.difficulty).abs() > 1e-4;
            changed.then_some((card, stability, difficulty))
        })
        .collect();
    korean_db::reviews::set_memory(pool, &updates)
        .await
        .map_err(err)?;
    Ok(updates.len())
}

/// Replays memory states, logging the outcome; for callers that cannot report errors.
pub async fn replay_logged(pool: &SqlitePool) {
    match replay(pool).await {
        Ok(0) => {}
        Ok(n) => log::info!("fsrs: replayed the memory of {n} cards"),
        Err(err) => log::warn!("fsrs: replaying memory states failed: {err}"),
    }
}

/// Fits the parameters to the review log if a day passed since the last fit and the log grew.
/// True when the parameters changed.
async fn optimize_if_due(pool: &SqlitePool, now_ms: i64) -> Result<bool, String> {
    let err = |e: sqlx::Error| e.to_string();
    let last = korean_db::settings::fsrs(pool).await.map_err(err)?;
    let reviews = korean_db::reviews::count(pool).await.map_err(err)?;
    if let Some(last) = &last
        && (now_ms - last.optimized_at < DAY_MS || reviews <= last.reviews)
    {
        return Ok(false);
    }
    let current = last.map(|l| l.parameters).unwrap_or_default();
    let histories: HashMap<Card, Vec<Answered>> =
        korean_db::reviews::histories(pool).await.map_err(err)?;
    let reference = scheduler(pool).await?;
    // Training is CPU-bound: keep it off the async workers.
    let fitted = tauri::async_runtime::spawn_blocking(move || {
        scheduler::optimize(&reference, histories.values().map(Vec::as_slice))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    let changed = fitted.is_some();
    // Recorded even when kept, so the next attempt waits for a day and new reviews.
    korean_db::settings::set_fsrs(
        pool,
        &Fsrs {
            parameters: fitted.unwrap_or(current),
            reviews,
            optimized_at: now_ms,
        },
    )
    .await
    .map_err(err)?;
    log::info!(
        "fsrs: optimized on {reviews} reviews, parameters {}",
        if changed { "updated" } else { "kept" }
    );
    Ok(changed)
}

/// Optimizes in the background, then replays memory states if the parameters changed.
pub fn optimize_later(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Some(state) = app.try_state::<AppState>() else {
            return;
        };
        let Ok(db) = state.db() else {
            return;
        };
        match optimize_if_due(db.pool(), now_ms()).await {
            Ok(true) => replay_logged(db.pool()).await,
            Ok(false) => {}
            Err(err) => log::warn!("fsrs: optimization failed: {err}"),
        }
    });
}
