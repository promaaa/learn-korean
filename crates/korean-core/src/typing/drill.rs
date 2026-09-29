//! Strict typing drill: wrong keys never advance the cursor but count as errors.

use serde::Serialize;

use super::compose::compose;
use super::jamo::{UnsupportedChar, indexed_jamo};
use super::layout::{Finger, Keystroke, keycap, keystroke_for_jamo, shift_finger};

/// Result of feeding one key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PressOutcome {
    pub correct: bool,
    pub finished: bool,
    /// Modifier-only presses and presses after the drill finished: not counted at all.
    pub ignored: bool,
}

/// The key the learner must press next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextKey {
    pub code: &'static str,
    pub shift: bool,
    pub jamo: char,
    pub finger: Finger,
    pub shift_finger: Option<Finger>,
}

/// Everything the UI renders about a drill at a given instant.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrillSnapshot {
    pub target: String,
    /// Live IME preview of the correctly typed prefix.
    pub typed: String,
    /// Target chars whose keystrokes are all typed.
    pub done_chars: usize,
    pub next: Option<NextKey>,
    /// Correct keystrokes.
    pub keystrokes: usize,
    pub errors: usize,
    pub streak: usize,
    pub best_streak: usize,
    pub accuracy: f64,
    pub cpm: f64,
    pub wpm: f64,
    pub finished: bool,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy)]
struct Step {
    char_index: usize,
    jamo: char,
    key: Keystroke,
}

#[derive(Debug, Clone)]
pub struct Drill {
    target: String,
    char_count: usize,
    steps: Vec<Step>,
    /// Char index of the last char of every space-separated word.
    word_last_chars: Vec<usize>,
    cursor: usize,
    errors: usize,
    streak: usize,
    best_streak: usize,
    first_press_ms: Option<u64>,
    finished_ms: Option<u64>,
}

fn is_modifier(code: &str) -> bool {
    matches!(
        code,
        "ShiftLeft"
            | "ShiftRight"
            | "ControlLeft"
            | "ControlRight"
            | "AltLeft"
            | "AltRight"
            | "MetaLeft"
            | "MetaRight"
            | "OSLeft"
            | "OSRight"
            | "CapsLock"
            | "Fn"
            | "FnLock"
            | "Lang1"
            | "Lang2"
            | "HangulMode"
            | "Hanja"
    )
}

impl Drill {
    pub fn new(target: &str) -> Result<Drill, UnsupportedChar> {
        let steps = indexed_jamo(target)?
            .into_iter()
            .map(|(char_index, jamo)| Step {
                char_index,
                jamo,
                key: keystroke_for_jamo(jamo).expect("indexed_jamo yields only typeable jamo"),
            })
            .collect();
        let chars: Vec<char> = target.chars().collect();
        let word_last_chars = chars
            .iter()
            .enumerate()
            .filter(|&(i, &c)| c != ' ' && chars.get(i + 1).is_none_or(|&n| n == ' '))
            .map(|(i, _)| i)
            .collect();
        Ok(Drill {
            target: target.to_owned(),
            char_count: chars.len(),
            steps,
            word_last_chars,
            cursor: 0,
            errors: 0,
            streak: 0,
            best_streak: 0,
            first_press_ms: None,
            finished_ms: None,
        })
    }

    fn is_finished(&self) -> bool {
        self.cursor == self.steps.len()
    }

    /// Feed one physical key press. Wrong keys do not advance (strict mode) but count as errors.
    pub fn press(&mut self, code: &str, shift: bool, at_ms: u64) -> PressOutcome {
        if is_modifier(code) || self.is_finished() {
            return PressOutcome {
                correct: true,
                finished: self.is_finished(),
                ignored: true,
            };
        }
        self.first_press_ms.get_or_insert(at_ms);
        let expected = self.steps[self.cursor].key;
        let correct = expected.code == code && expected.shift == shift;
        if correct {
            self.cursor += 1;
            self.streak += 1;
            self.best_streak = self.best_streak.max(self.streak);
            if self.is_finished() {
                self.finished_ms = Some(at_ms);
            }
        } else {
            self.errors += 1;
            self.streak = 0;
        }
        PressOutcome {
            correct,
            finished: self.is_finished(),
            ignored: false,
        }
    }

    pub fn snapshot(&self, now_ms: u64) -> DrillSnapshot {
        let finished = self.is_finished();
        let typed_jamo: Vec<char> = self.steps[..self.cursor].iter().map(|s| s.jamo).collect();
        let done_chars = self
            .steps
            .get(self.cursor)
            .map_or(self.char_count, |s| s.char_index);
        let done_words = self
            .word_last_chars
            .iter()
            .filter(|&&i| i < done_chars)
            .count();
        let next = self.steps.get(self.cursor).map(|s| {
            let finger = keycap(s.key.code)
                .expect("keystroke codes come from the layout")
                .finger;
            NextKey {
                code: s.key.code,
                shift: s.key.shift,
                jamo: s.jamo,
                finger,
                shift_finger: s.key.shift.then(|| shift_finger(finger)),
            }
        });
        let elapsed_ms = match self.first_press_ms {
            Some(start) => self.finished_ms.unwrap_or(now_ms).saturating_sub(start),
            None => 0,
        };
        let keystrokes = self.cursor;
        let attempts = keystrokes + self.errors;
        let accuracy = if attempts == 0 {
            1.0
        } else {
            keystrokes as f64 / attempts as f64
        };
        let per_minute = |count: usize| {
            if elapsed_ms == 0 {
                0.0
            } else {
                count as f64 * 60_000.0 / elapsed_ms as f64
            }
        };
        DrillSnapshot {
            target: self.target.clone(),
            typed: compose(&typed_jamo),
            done_chars,
            next,
            keystrokes,
            errors: self.errors,
            streak: self.streak,
            best_streak: self.best_streak,
            accuracy,
            cpm: per_minute(keystrokes),
            wpm: per_minute(done_words),
            finished,
            elapsed_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::jamo::keystrokes_for_text;
    use super::*;

    /// Types every expected keystroke of `drill`'s target, one every `step_ms`.
    fn type_all(drill: &mut Drill, target: &str, start_ms: u64, step_ms: u64) -> PressOutcome {
        let mut last = None;
        for (n, (_, k)) in keystrokes_for_text(target).unwrap().into_iter().enumerate() {
            let out = drill.press(k.code, k.shift, start_ms + n as u64 * step_ms);
            assert!(out.correct && !out.ignored);
            last = Some(out);
        }
        last.unwrap()
    }

    #[test]
    fn correct_run_finishes_with_full_accuracy() {
        let target = "괜찮아요?";
        let mut drill = Drill::new(target).unwrap();
        let out = type_all(&mut drill, target, 1_000, 100);
        assert!(out.finished);
        let snap = drill.snapshot(99_999);
        assert!(snap.finished);
        assert_eq!(snap.typed, target);
        assert_eq!(snap.done_chars, 5);
        assert_eq!(snap.next, None);
        assert_eq!(snap.keystrokes, 13);
        assert_eq!(snap.errors, 0);
        assert_eq!(snap.accuracy, 1.0);
        assert_eq!(snap.streak, 13);
        assert_eq!(snap.best_streak, 13);
        // Elapsed freezes at the finishing press.
        assert_eq!(snap.elapsed_ms, 1_200);
        let after = drill.press("KeyA", false, 100_000);
        assert!(after.ignored && after.finished);
    }

    #[test]
    fn wrong_key_counts_error_and_does_not_advance() {
        let mut drill = Drill::new("한").unwrap();
        assert!(drill.press("KeyG", false, 0).correct);
        let out = drill.press("KeyJ", false, 10);
        assert_eq!(
            out,
            PressOutcome {
                correct: false,
                finished: false,
                ignored: false
            }
        );
        let snap = drill.snapshot(10);
        assert_eq!(snap.typed, "ㅎ");
        assert_eq!(snap.keystrokes, 1);
        assert_eq!(snap.errors, 1);
        assert_eq!(snap.accuracy, 0.5);
        assert_eq!(snap.next.unwrap().code, "KeyK");
        assert_eq!(snap.next.unwrap().jamo, 'ㅏ');
    }

    #[test]
    fn shift_must_match_exactly() {
        let mut drill = Drill::new("까가").unwrap();
        let next = drill.snapshot(0).next.unwrap();
        assert_eq!(
            next,
            NextKey {
                code: "KeyR",
                shift: true,
                jamo: 'ㄲ',
                finger: Finger::LeftIndex,
                shift_finger: Some(Finger::RightPinky),
            }
        );
        // Missing shift on ㄲ is an error.
        assert!(!drill.press("KeyR", false, 0).correct);
        assert!(drill.press("KeyR", true, 1).correct);
        assert!(drill.press("KeyK", false, 2).correct);
        // Typing ㄲ when ㄱ is expected is an error.
        assert!(!drill.press("KeyR", true, 3).correct);
        assert!(drill.press("KeyR", false, 4).correct);
        assert_eq!(drill.snapshot(4).next.unwrap().shift_finger, None);
        assert_eq!(drill.snapshot(4).errors, 2);
    }

    #[test]
    fn modifiers_are_ignored() {
        let mut drill = Drill::new("빨").unwrap();
        let out = drill.press("ShiftLeft", true, 0);
        assert_eq!(
            out,
            PressOutcome {
                correct: true,
                finished: false,
                ignored: true
            }
        );
        let snap = drill.snapshot(500);
        assert_eq!((snap.keystrokes, snap.errors, snap.elapsed_ms), (0, 0, 0));
        assert_eq!(snap.accuracy, 1.0);
        assert_eq!(snap.cpm, 0.0);
    }

    #[test]
    fn streak_resets_on_error_and_best_is_kept() {
        let mut drill = Drill::new("하나").unwrap();
        drill.press("KeyG", false, 0);
        drill.press("KeyK", false, 1);
        drill.press("KeyS", false, 2);
        drill.press("KeyA", false, 3);
        let snap = drill.snapshot(3);
        assert_eq!((snap.streak, snap.best_streak), (0, 3));
        drill.press("KeyK", false, 4);
        let snap = drill.snapshot(4);
        assert_eq!((snap.streak, snap.best_streak), (1, 3));
        assert!(snap.finished);
        assert_eq!(snap.accuracy, 0.8);
    }

    #[test]
    fn live_preview_and_done_chars() {
        let target = "앉아";
        let mut drill = Drill::new(target).unwrap();
        let keys = keystrokes_for_text(target).unwrap();
        let expected = [
            ("ㅇ", 0),
            ("아", 0),
            ("안", 0),
            ("앉", 1),
            ("앉ㅇ", 1),
            ("앉아", 2),
        ];
        for ((_, k), (typed, done)) in keys.into_iter().zip(expected) {
            drill.press(k.code, k.shift, 0);
            let snap = drill.snapshot(0);
            assert_eq!((snap.typed.as_str(), snap.done_chars), (typed, done));
        }
    }

    #[test]
    fn cpm_and_wpm_from_timestamps() {
        // "뭐 해요?" = 3 + 1 + 4 + 1 = 9 keystrokes, 2 words.
        let target = "뭐 해요?";
        let mut drill = Drill::new(target).unwrap();
        let keys = keystrokes_for_text(target).unwrap();
        assert_eq!(keys.len(), 9);
        // First 3 keystrokes (뭐) at 0, 1s, 2s.
        for (n, (_, k)) in keys[..3].iter().enumerate() {
            drill.press(k.code, k.shift, n as u64 * 1_000);
        }
        let snap = drill.snapshot(2_000);
        // 뭐 is done but the space has not been typed yet: the word still counts as completed.
        assert_eq!(snap.done_chars, 1);
        assert_eq!(snap.elapsed_ms, 2_000);
        assert_eq!(snap.cpm, 90.0);
        assert_eq!(snap.wpm, 30.0);
        // Snapshot later without pressing: rates drop with time.
        let snap = drill.snapshot(6_000);
        assert_eq!(snap.cpm, 30.0);
        assert_eq!(snap.wpm, 10.0);
        // Finish: remaining 6 keystrokes, last one at 30s.
        for (n, (_, k)) in keys[3..].iter().enumerate() {
            drill.press(k.code, k.shift, 25_000 + n as u64 * 1_000);
        }
        let snap = drill.snapshot(1_000_000);
        assert!(snap.finished);
        assert_eq!(snap.elapsed_ms, 30_000);
        assert_eq!(snap.cpm, 18.0);
        assert_eq!(snap.wpm, 4.0);
    }

    #[test]
    fn unsupported_target_is_rejected_and_empty_is_finished() {
        assert_eq!(
            Drill::new("hi").unwrap_err(),
            UnsupportedChar { index: 0, ch: 'h' }
        );
        let drill = Drill::new("").unwrap();
        assert!(drill.snapshot(0).finished);
    }

    #[test]
    fn snapshot_serializes_camel_case() {
        let mut drill = Drill::new("쓰").unwrap();
        drill.press("KeyT", true, 0);
        let json = serde_json::to_value(drill.snapshot(0)).unwrap();
        assert_eq!(json["doneChars"], 0);
        assert_eq!(json["bestStreak"], 1);
        assert_eq!(json["elapsedMs"], 0);
        assert_eq!(
            json["next"],
            serde_json::json!({
                "code": "KeyM", "shift": false, "jamo": "ㅡ",
                "finger": "rightIndex", "shiftFinger": null
            })
        );
        let out = serde_json::to_value(drill.press("KeyM", false, 1)).unwrap();
        assert_eq!(
            out,
            serde_json::json!({"correct": true, "finished": true, "ignored": false})
        );
    }
}
