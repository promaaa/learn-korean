//! Review session commands. All rules live in `korean_core::learning`; this module only loads
//! data, holds the running session and persists outcomes.

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use korean_core::content::bundled_glossary;
use korean_core::learning::{
    Answer, ExerciseView, Feedback, Focus, Limits, PLAYABLE, Progress, Session, plan_session,
};
use serde::Serialize;
use tauri::State;

use crate::progress::{self, LevelUp, Profile};
use crate::state::AppState;

#[derive(Default)]
pub struct ActiveSession(Mutex<Option<Session>>);

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStarted {
    total: usize,
    due: usize,
    new: usize,
    focus: Focus,
}

#[tauri::command]
pub async fn session_start(
    app: State<'_, AppState>,
    active: State<'_, ActiveSession>,
) -> Result<SessionStarted, String> {
    let pool = app.db()?.pool();
    let now = now_ms();
    let level = progress::load(pool, now).await?.level.level;
    let items = korean_db::content::unlocked_items(pool, level)
        .await
        .map_err(|e| e.to_string())?;
    let states = korean_db::reviews::states(pool)
        .await
        .map_err(|e| e.to_string())?;
    let focus = korean_db::settings::focus(pool)
        .await
        .map_err(|e| e.to_string())?;
    let plan = plan_session(&items, &states, PLAYABLE, focus, now, Limits::default());
    let started = SessionStarted {
        total: plan.cards.len(),
        due: plan.due,
        new: plan.new,
        focus,
    };
    let session = Session::new(plan, items, states, bundled_glossary(), now as u64);
    *active.0.lock().map_err(|e| e.to_string())? = Some(session);
    Ok(started)
}

/// Persists the learner's focus; it applies from the next `session_start`.
#[tauri::command]
pub async fn set_focus(focus: Focus, app: State<'_, AppState>) -> Result<(), String> {
    korean_db::settings::set_focus(app.db()?.pool(), focus, now_ms())
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Current {
    exercise: Option<ExerciseView>,
    progress: Progress,
}

/// The exercise waiting for an answer (`None` once the session is finished).
#[tauri::command]
pub fn session_current(active: State<'_, ActiveSession>) -> Result<Current, String> {
    let mut guard = active.0.lock().map_err(|e| e.to_string())?;
    let session = guard.as_mut().ok_or("no session")?;
    Ok(Current {
        exercise: session.current().cloned(),
        progress: session.progress(),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Answered {
    feedback: Feedback,
    progress: Progress,
    profile: Profile,
    level_up: Option<LevelUp>,
}

#[tauri::command]
pub async fn session_answer(
    answer: Answer,
    elapsed_ms: u64,
    app: State<'_, AppState>,
    active: State<'_, ActiveSession>,
) -> Result<Answered, String> {
    let pool = app.db()?.pool();
    let now = now_ms();
    let before = progress::load(pool, now).await?;
    let (outcome, progress) = {
        let mut guard = active.0.lock().map_err(|e| e.to_string())?;
        let session = guard.as_mut().ok_or("no session")?;
        let outcome = session
            .answer(&answer, elapsed_ms, now)
            .map_err(|e| e.to_string())?;
        (outcome, session.progress())
    };
    korean_db::reviews::record(pool, &outcome)
        .await
        .map_err(|e| e.to_string())?;
    let profile = progress::load(pool, now).await?;
    Ok(Answered {
        feedback: outcome.feedback,
        progress,
        level_up: progress::level_up(&before, &profile),
        profile,
    })
}
