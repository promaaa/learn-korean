//! Hangul primer commands. Lessons and the pass rule live in `korean_core::typing::primer`;
//! passes are stored by `korean_db::hangul`. The round is separate from the Typing Gym's, so
//! switching screens mid-round leaves both where they were.

use std::sync::Mutex;

use korean_core::typing::Round;
use korean_core::typing::primer::{NewJamo, PASS_PERCENT, Primer, bundled_primer, round_passes};
use serde::Serialize;
use tauri::State;

use crate::session::now_ms;
use crate::state::AppState;
use crate::typing::{DrillView, Pressed, new_round, press, view};

struct LessonRound {
    lesson: usize,
    round: Round,
}

pub struct HangulPrimer {
    primer: Primer,
    round: Mutex<Option<LessonRound>>,
}

impl Default for HangulPrimer {
    fn default() -> Self {
        HangulPrimer {
            primer: bundled_primer(),
            round: Mutex::new(None),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonView {
    id: String,
    title: String,
    jamo: Vec<NewJamo>,
    lines: usize,
    unlocked: bool,
    passed: bool,
}

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

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum HangulNext {
    Line { view: DrillView },
    Over { result: LessonResult },
}

/// Every lesson, in order, with whether it is open and passed.
#[tauri::command]
pub async fn hangul_lessons(
    app: State<'_, AppState>,
    hangul: State<'_, HangulPrimer>,
) -> Result<Vec<LessonView>, String> {
    let passed = korean_db::hangul::passed_lessons(app.db()?.pool())
        .await
        .map_err(|e| e.to_string())?;
    let primer = &hangul.primer;
    Ok(primer
        .lessons
        .iter()
        .enumerate()
        .map(|(index, lesson)| LessonView {
            id: lesson.id.clone(),
            title: lesson.title.clone(),
            jamo: lesson.jamo.clone(),
            lines: lesson.lines.len(),
            unlocked: primer.is_unlocked(index, &passed),
            passed: passed.contains(&lesson.id),
        })
        .collect())
}

/// Starts a round of the lesson's lines, if it is unlocked.
#[tauri::command]
pub async fn hangul_start(
    lesson: usize,
    app: State<'_, AppState>,
    hangul: State<'_, HangulPrimer>,
) -> Result<DrillView, String> {
    let passed = korean_db::hangul::passed_lessons(app.db()?.pool())
        .await
        .map_err(|e| e.to_string())?;
    let primer = &hangul.primer;
    if !primer.is_unlocked(lesson, &passed) {
        return Err(format!("lesson {} is locked", lesson + 1));
    }
    let round = new_round(primer.targets(&primer.lessons[lesson]))?;
    let started = view(&round, 0);
    *hangul.round.lock().map_err(|e| e.to_string())? = Some(LessonRound { lesson, round });
    Ok(started)
}

/// One physical key press (`KeyboardEvent.code`), timestamped by the UI's monotonic clock.
#[tauri::command]
pub fn hangul_press(
    code: String,
    shift: bool,
    at_ms: u64,
    hangul: State<'_, HangulPrimer>,
) -> Result<Pressed, String> {
    let mut guard = hangul.round.lock().map_err(|e| e.to_string())?;
    let current = guard.as_mut().ok_or("no lesson round")?;
    Ok(press(&mut current.round, &code, shift, at_ms))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentLesson {
    lesson: usize,
    view: DrillView,
}

/// The round left when the screen was switched away, so the primer resumes it.
#[tauri::command]
pub fn hangul_current(hangul: State<'_, HangulPrimer>) -> Result<Option<CurrentLesson>, String> {
    let guard = hangul.round.lock().map_err(|e| e.to_string())?;
    Ok(guard.as_ref().map(|current| CurrentLesson {
        lesson: current.lesson,
        view: view(&current.round, 0),
    }))
}

/// Moves to the next line once the current one is typed. After the last line, scores the round
/// and records the pass.
#[tauri::command]
pub async fn hangul_next(
    app: State<'_, AppState>,
    hangul: State<'_, HangulPrimer>,
) -> Result<HangulNext, String> {
    let finished = {
        let mut guard = hangul.round.lock().map_err(|e| e.to_string())?;
        let current = guard.as_mut().ok_or("no lesson round")?;
        if current.round.advance() {
            return Ok(HangulNext::Line {
                view: view(&current.round, 0),
            });
        }
        if !current.round.is_over() {
            return Err("the line is not typed yet".into());
        }
        guard.take().expect("checked above")
    };
    let LessonRound { lesson, round } = finished;
    let passed = round_passes(&round);
    if passed {
        let id = &hangul.primer.lessons[lesson].id;
        korean_db::hangul::record_pass(app.db()?.pool(), id, now_ms())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(HangulNext::Over {
        result: LessonResult {
            lesson,
            accuracy: round.accuracy(),
            keystrokes: round.keystrokes(),
            errors: round.errors(),
            passed,
            pass_percent: PASS_PERCENT,
        },
    })
}
