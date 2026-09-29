//! Learner profile: level, experience, daily streak, unlocked packs. Rules live in
//! `korean_core::progression`; this module only reads the logs.

use korean_core::progression::{self, Level};
use serde::Serialize;
use sqlx::SqlitePool;
use tauri::State;

use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackStatus {
    id: String,
    title: String,
    unlock_level: u32,
    unlocked: bool,
    items: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub level: Level,
    day_streak: u32,
    packs: Vec<PackStatus>,
}

/// The learner's UTC offset right now, for day boundaries.
pub fn utc_offset_seconds() -> i32 {
    chrono::Local::now().offset().local_minus_utc()
}

pub async fn load(pool: &SqlitePool, now_ms: i64) -> Result<Profile, String> {
    let err = |e: sqlx::Error| e.to_string();
    let level = progression::level_for(korean_db::progress::total_xp(pool).await.map_err(err)?);
    let offset = utc_offset_seconds();
    let days = korean_db::progress::practice_days(pool, offset)
        .await
        .map_err(err)?;
    let today = progression::local_day(now_ms, offset);
    let packs = korean_db::content::packs(pool)
        .await
        .map_err(err)?
        .into_iter()
        .map(|p| {
            let unlock_level = p.unlock_level.max(1) as u32;
            PackStatus {
                id: p.id,
                title: p.title,
                unlock_level,
                unlocked: unlock_level <= level.level,
                items: p.items,
            }
        })
        .collect();
    Ok(Profile {
        level,
        day_streak: progression::day_streak(&days, today),
        packs,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelUp {
    level: u32,
    /// Titles of the packs this level unlocked.
    unlocked: Vec<String>,
}

/// What changed between two profiles, if the learner levelled up.
pub fn level_up(before: &Profile, after: &Profile) -> Option<LevelUp> {
    (after.level.level > before.level.level).then(|| LevelUp {
        level: after.level.level,
        unlocked: after
            .packs
            .iter()
            .filter(|p| p.unlock_level > before.level.level && p.unlocked)
            .map(|p| p.title.clone())
            .collect(),
    })
}

#[tauri::command]
pub async fn profile(app: State<'_, AppState>) -> Result<Profile, String> {
    load(app.db()?.pool(), crate::session::now_ms()).await
}
