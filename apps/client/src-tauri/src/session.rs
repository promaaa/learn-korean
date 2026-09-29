//! Review session commands. All rules live in `korean_core::learning`; this module only loads
//! data, holds the running session and persists outcomes.

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use korean_core::learning::{
    Answer, ExerciseView, Feedback, Limits, PLAYABLE, Progress, Session, plan_session,
};
use serde::Serialize;
use tauri::State;

use crate::state::AppState;

/// Until progression lands every learner is level 1.
const LEVEL: u32 = 1;

#[derive(Default)]
pub struct ActiveSession(Mutex<Option<Session>>);

fn now_ms() -> i64 {
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
}

#[tauri::command]
pub async fn session_start(
    app: State<'_, AppState>,
    active: State<'_, ActiveSession>,
) -> Result<SessionStarted, String> {
    let pool = app.db()?.pool();
    let items = korean_db::content::unlocked_items(pool, LEVEL)
        .await
        .map_err(|e| e.to_string())?;
    let states = korean_db::reviews::states(pool)
        .await
        .map_err(|e| e.to_string())?;
    let now = now_ms();
    let plan = plan_session(&items, &states, PLAYABLE, now, Limits::default());
    let started = SessionStarted {
        total: plan.cards.len(),
        due: plan.due,
        new: plan.new,
    };
    let session = Session::new(plan, items, states, now as u64);
    *active.0.lock().map_err(|e| e.to_string())? = Some(session);
    Ok(started)
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
}

#[tauri::command]
pub async fn session_answer(
    answer: Answer,
    elapsed_ms: u64,
    app: State<'_, AppState>,
    active: State<'_, ActiveSession>,
) -> Result<Answered, String> {
    let (outcome, progress) = {
        let mut guard = active.0.lock().map_err(|e| e.to_string())?;
        let session = guard.as_mut().ok_or("no session")?;
        let outcome = session
            .answer(&answer, elapsed_ms, now_ms())
            .map_err(|e| e.to_string())?;
        (outcome, session.progress())
    };
    korean_db::reviews::record(app.db()?.pool(), &outcome)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Answered {
        feedback: outcome.feedback,
        progress,
    })
}
