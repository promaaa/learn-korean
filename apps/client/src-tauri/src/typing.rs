//! Typing Gym commands: the keyboard course (`korean_core::typing::keyboard`, passes stored by
//! `korean_db::lessons`) and free rounds of words from the unlocked packs. The drill state machine
//! lives in `korean_core::typing`; key presses are forwarded one by one and answered with a fresh
//! snapshot.

use std::collections::HashSet;
use std::sync::Mutex;

use korean_core::content::Skill;
use korean_core::typing::keyboard::Course;
use korean_core::typing::primer::{PASS_PERCENT, round_passes};
use korean_core::typing::{DrillSnapshot, KeyCap, PressOutcome, Round, Target, pick_targets};
use korean_db::lessons::Course as Progress;
use serde::Serialize;
use tauri::State;

use crate::progress;
use crate::session::now_ms;
use crate::state::AppState;

/// Lines per round of words.
const ROUND: usize = 8;

/// A round in progress: a course lesson (by index), or words when `lesson` is `None`.
struct GymRound {
    lesson: Option<usize>,
    round: Round,
}

pub struct TypingGym {
    course: Course,
    round: Mutex<Option<GymRound>>,
}

impl Default for TypingGym {
    fn default() -> Self {
        TypingGym {
            course: Course::new(),
            round: Mutex::new(None),
        }
    }
}

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

/// The score of a finished lesson round (Typing Gym course, Hangul primer).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonResult {
    /// Index of the lesson in the list.
    lesson: usize,
    accuracy: f64,
    keystrokes: usize,
    errors: usize,
    passed: bool,
    /// Accuracy needed to pass, in percent.
    pass_percent: usize,
}

impl LessonResult {
    pub fn of(lesson: usize, round: &Round) -> LessonResult {
        LessonResult {
            lesson,
            accuracy: round.accuracy(),
            keystrokes: round.keystrokes(),
            errors: round.errors(),
            passed: round_passes(round),
            pass_percent: PASS_PERCENT,
        }
    }

    pub fn passed(&self) -> bool {
        self.passed
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyLessonView {
    id: String,
    title: String,
    /// The keys the lesson's stage teaches.
    keys: Vec<char>,
    unlocked: bool,
    passed: bool,
}

/// Every course lesson, in order, with whether it is open and passed.
#[tauri::command]
pub async fn typing_lessons(
    app: State<'_, AppState>,
    gym: State<'_, TypingGym>,
) -> Result<Vec<KeyLessonView>, String> {
    let passed = korean_db::lessons::passed_lessons(app.db()?.pool(), Progress::Keyboard)
        .await
        .map_err(|e| e.to_string())?;
    let course = &gym.course;
    Ok(course
        .lessons
        .iter()
        .enumerate()
        .map(|(index, lesson)| KeyLessonView {
            id: lesson.id.clone(),
            title: lesson.title.clone(),
            keys: lesson.new.clone(),
            unlocked: course.is_unlocked(index, &passed),
            passed: passed.contains(&lesson.id),
        })
        .collect())
}

#[tauri::command]
pub fn typing_layout() -> &'static [KeyCap] {
    korean_core::typing::layout()
}

/// Starts a round: the course lesson `lesson` (fresh random lines; it must be unlocked) or, for
/// `None`, lines taken from unlocked packs, known items first.
#[tauri::command]
pub async fn typing_start(
    lesson: Option<usize>,
    app: State<'_, AppState>,
    gym: State<'_, TypingGym>,
) -> Result<DrillView, String> {
    let pool = app.db()?.pool();
    let now = now_ms();
    let targets = match lesson {
        Some(index) => {
            let passed = korean_db::lessons::passed_lessons(pool, Progress::Keyboard)
                .await
                .map_err(|e| e.to_string())?;
            if !gym.course.is_unlocked(index, &passed) {
                return Err(format!("lesson {} is locked", index + 1));
            }
            gym.course.lessons[index].targets(now as u64)
        }
        None => {
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
            pick_targets(&items, &seen, ROUND, now as u64)
        }
    };
    let round = new_round(targets)?;
    let started = view(&round, 0);
    *gym.round.lock().map_err(|e| e.to_string())? = Some(GymRound { lesson, round });
    Ok(started)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentRound {
    lesson: Option<usize>,
    view: DrillView,
}

/// The round left when the screen was switched away, so the gym resumes it.
#[tauri::command]
pub fn typing_current(gym: State<'_, TypingGym>) -> Result<Option<CurrentRound>, String> {
    let guard = gym.round.lock().map_err(|e| e.to_string())?;
    Ok(guard.as_ref().map(|current| CurrentRound {
        lesson: current.lesson,
        view: view(&current.round, 0),
    }))
}

/// One physical key press (`KeyboardEvent.code`), timestamped by the UI's monotonic clock.
#[tauri::command]
pub fn typing_press(
    code: String,
    shift: bool,
    at_ms: u64,
    gym: State<'_, TypingGym>,
) -> Result<Pressed, String> {
    let mut guard = gym.round.lock().map_err(|e| e.to_string())?;
    let current = guard.as_mut().ok_or("no typing round")?;
    Ok(press(&mut current.round, &code, shift, at_ms))
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum GymNext {
    Line {
        view: DrillView,
    },
    /// `result` scores a course lesson; a round of words has none.
    Over {
        result: Option<LessonResult>,
    },
}

/// Moves to the next line once the current one is typed. After the last line of a lesson, scores
/// the round and records the pass.
#[tauri::command]
pub async fn typing_next(
    app: State<'_, AppState>,
    gym: State<'_, TypingGym>,
) -> Result<GymNext, String> {
    let finished = {
        let mut guard = gym.round.lock().map_err(|e| e.to_string())?;
        let current = guard.as_mut().ok_or("no typing round")?;
        if current.round.advance() {
            return Ok(GymNext::Line {
                view: view(&current.round, 0),
            });
        }
        if !current.round.is_over() {
            return Err("the line is not typed yet".into());
        }
        guard.take().expect("checked above")
    };
    let Some(lesson) = finished.lesson else {
        return Ok(GymNext::Over { result: None });
    };
    let result = LessonResult::of(lesson, &finished.round);
    if result.passed() {
        let id = &gym.course.lessons[lesson].id;
        korean_db::lessons::record_pass(app.db()?.pool(), Progress::Keyboard, id, now_ms())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(GymNext::Over {
        result: Some(result),
    })
}
