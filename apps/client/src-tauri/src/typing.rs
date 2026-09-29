//! Typing Gym commands: the drill state machine lives in `korean_core::typing`; key presses are
//! forwarded one by one and answered with a fresh snapshot.

use std::collections::{HashSet, VecDeque};
use std::sync::Mutex;

use korean_core::content::Skill;
use korean_core::typing::{Drill, DrillSnapshot, KeyCap, PressOutcome, Target, pick_targets};
use serde::Serialize;
use tauri::State;

use crate::progress;
use crate::session::now_ms;
use crate::state::AppState;

/// Lines per gym round.
const ROUND: usize = 8;

struct Round {
    queue: VecDeque<Target>,
    current: Target,
    drill: Drill,
    position: usize,
    total: usize,
}

#[derive(Default)]
pub struct TypingGym(Mutex<Option<Round>>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrillView {
    target: Target,
    snapshot: DrillSnapshot,
    /// 1-based index of this line in the round.
    position: usize,
    total: usize,
}

fn view(round: &Round, now: u64) -> DrillView {
    DrillView {
        target: round.current.clone(),
        snapshot: round.drill.snapshot(now),
        position: round.position,
        total: round.total,
    }
}

fn start_drill(target: Target) -> Result<(Target, Drill), String> {
    let drill = Drill::new(&target.korean).map_err(|e| e.to_string())?;
    Ok((target, drill))
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
    let mut queue: VecDeque<Target> = pick_targets(&items, &seen, ROUND, now as u64).into();
    let first = queue.pop_front().ok_or("nothing to type yet")?;
    let (current, drill) = start_drill(first)?;
    let round = Round {
        total: queue.len() + 1,
        queue,
        current,
        drill,
        position: 1,
    };
    let started = view(&round, 0);
    *gym.0.lock().map_err(|e| e.to_string())? = Some(round);
    Ok(started)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pressed {
    outcome: PressOutcome,
    view: DrillView,
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
    let outcome = round.drill.press(&code, shift, at_ms);
    Ok(Pressed {
        outcome,
        view: view(round, at_ms),
    })
}

/// Moves to the next line; `None` when the round is over.
#[tauri::command]
pub fn typing_next(gym: State<'_, TypingGym>) -> Result<Option<DrillView>, String> {
    let mut guard = gym.0.lock().map_err(|e| e.to_string())?;
    let round = guard.as_mut().ok_or("no typing round")?;
    let Some(next) = round.queue.pop_front() else {
        return Ok(None);
    };
    let (current, drill) = start_drill(next)?;
    round.current = current;
    round.drill = drill;
    round.position += 1;
    Ok(Some(view(round, 0)))
}
