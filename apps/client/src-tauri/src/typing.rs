//! Typing Gym commands: the drill state machine lives in `korean_core::typing`; key presses are
//! forwarded one by one and answered with a fresh snapshot.

use std::collections::HashSet;
use std::sync::Mutex;

use korean_core::content::Skill;
use korean_core::typing::{DrillSnapshot, KeyCap, PressOutcome, Round, Target, pick_targets};
use serde::Serialize;
use tauri::State;

use crate::progress;
use crate::session::now_ms;
use crate::state::AppState;

/// Lines per gym round.
const ROUND: usize = 8;

#[derive(Default)]
pub struct TypingGym(Mutex<Option<Round>>);

/// A round's current line, shared by the Typing Gym and the Hangul primer.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrillView {
    target: Target,
    snapshot: DrillSnapshot,
    /// 1-based index of this line in the round.
    position: usize,
    total: usize,
}

pub fn view(round: &Round, now: u64) -> DrillView {
    DrillView {
        target: round.current().clone(),
        snapshot: round.snapshot(now),
        position: round.position(),
        total: round.total(),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pressed {
    outcome: PressOutcome,
    view: DrillView,
}

/// Feeds one key press to `round` and answers with the outcome and the new view.
pub fn press(round: &mut Round, code: &str, shift: bool, at_ms: u64) -> Pressed {
    let outcome = round.press(code, shift, at_ms);
    Pressed {
        outcome,
        view: view(round, at_ms),
    }
}

/// A round of `targets`; an error when there is nothing to type.
pub fn new_round(targets: Vec<Target>) -> Result<Round, String> {
    Round::new(targets)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "nothing to type yet".to_owned())
}

#[tauri::command]
pub fn typing_layout() -> &'static [KeyCap] {
    korean_core::typing::layout()
}

/// Starts a round of lines taken from unlocked packs, known items first.
#[tauri::command]
pub async fn typing_start(
    app: State<'_, AppState>,
    gym: State<'_, TypingGym>,
) -> Result<DrillView, String> {
    let pool = app.db()?.pool();
    let now = now_ms();
    let level = progress::load(pool, now).await?.level.level;
    let items = korean_db::content::unlocked_items(pool, level)
        .await
        .map_err(|e| e.to_string())?;
    let seen: HashSet<String> = korean_db::reviews::states(pool)
        .await
        .map_err(|e| e.to_string())?
        .into_keys()
        .filter(|card| card.skill == Skill::Listening)
        .map(|card| card.item_id)
        .collect();
    let round = new_round(pick_targets(&items, &seen, ROUND, now as u64))?;
    let started = view(&round, 0);
    *gym.0.lock().map_err(|e| e.to_string())? = Some(round);
    Ok(started)
}

/// One physical key press (`KeyboardEvent.code`), timestamped by the UI's monotonic clock.
#[tauri::command]
pub fn typing_press(
    code: String,
    shift: bool,
    at_ms: u64,
    gym: State<'_, TypingGym>,
) -> Result<Pressed, String> {
    let mut guard = gym.0.lock().map_err(|e| e.to_string())?;
    let round = guard.as_mut().ok_or("no typing round")?;
    Ok(press(round, &code, shift, at_ms))
}

/// Moves to the next line once the current one is typed; `None` when the round is over.
#[tauri::command]
pub fn typing_next(gym: State<'_, TypingGym>) -> Result<Option<DrillView>, String> {
    let mut guard = gym.0.lock().map_err(|e| e.to_string())?;
    let round = guard.as_mut().ok_or("no typing round")?;
    if round.advance() {
        Ok(Some(view(round, 0)))
    } else if round.is_over() {
        Ok(None)
    } else {
        Err("the line is not typed yet".into())
    }
}
