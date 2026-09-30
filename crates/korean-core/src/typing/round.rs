//! A round: lines typed one after the other (Typing Gym, Hangul primer), scored together.

use std::collections::VecDeque;

use super::drill::{Drill, DrillSnapshot, PressOutcome};
use super::jamo::UnsupportedChar;
use super::targets::Target;

#[derive(Debug, Clone)]
pub struct Round {
    upcoming: VecDeque<(Target, Drill)>,
    current: Target,
    drill: Drill,
    position: usize,
    total: usize,
    /// Correct keystrokes and errors of the lines already left behind.
    past_keystrokes: usize,
    past_errors: usize,
}

impl Round {
    /// `None` when there is nothing to type.
    pub fn new(targets: Vec<Target>) -> Result<Option<Round>, UnsupportedChar> {
        let mut upcoming = targets
            .into_iter()
            .map(|target| Drill::new(&target.korean).map(|drill| (target, drill)))
            .collect::<Result<VecDeque<_>, _>>()?;
        let total = upcoming.len();
        Ok(upcoming.pop_front().map(|(current, drill)| Round {
            upcoming,
            current,
            drill,
            position: 1,
            total,
            past_keystrokes: 0,
            past_errors: 0,
        }))
    }

    pub fn current(&self) -> &Target {
        &self.current
    }

    /// 1-based index of the current line.
    pub fn position(&self) -> usize {
        self.position
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn press(&mut self, code: &str, shift: bool, at_ms: u64) -> PressOutcome {
        self.drill.press(code, shift, at_ms)
    }

    pub fn snapshot(&self, now_ms: u64) -> DrillSnapshot {
        self.drill.snapshot(now_ms)
    }

    /// Moves to the next line once the current one is typed. `false` when the current line is
    /// unfinished or was the last one.
    pub fn advance(&mut self) -> bool {
        if !self.drill.is_finished() {
            return false;
        }
        let Some((current, drill)) = self.upcoming.pop_front() else {
            return false;
        };
        self.past_keystrokes += self.drill.keystrokes();
        self.past_errors += self.drill.errors();
        self.current = current;
        self.drill = drill;
        self.position += 1;
        true
    }

    /// Every line of the round is typed.
    pub fn is_over(&self) -> bool {
        self.upcoming.is_empty() && self.drill.is_finished()
    }

    /// Correct keystrokes over the whole round so far.
    pub fn keystrokes(&self) -> usize {
        self.past_keystrokes + self.drill.keystrokes()
    }

    /// Wrong keys over the whole round so far.
    pub fn errors(&self) -> usize {
        self.past_errors + self.drill.errors()
    }

    /// Correct keystrokes over all keys pressed in the round (1 before the first press).
    pub fn accuracy(&self) -> f64 {
        let attempts = self.keystrokes() + self.errors();
        if attempts == 0 {
            1.0
        } else {
            self.keystrokes() as f64 / attempts as f64
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::typing::keystrokes_for_text;

    pub(crate) fn target(korean: &str) -> Target {
        Target {
            item_id: korean.into(),
            korean: korean.into(),
            english: String::new(),
        }
    }

    /// Types the current line; `mistakes` wrong keys first.
    pub(crate) fn type_line(round: &mut Round, mistakes: usize) {
        for _ in 0..mistakes {
            assert!(!round.press("Backquote", false, 0).correct);
        }
        let korean = round.current().korean.clone();
        for (_, key) in keystrokes_for_text(&korean).unwrap() {
            assert!(round.press(key.code, key.shift, 0).correct);
        }
    }

    #[test]
    fn lines_advance_only_once_typed_and_the_round_scores_every_line() {
        let mut round = Round::new(vec![target("가"), target("나다")])
            .unwrap()
            .unwrap();
        assert_eq!((round.position(), round.total()), (1, 2));
        assert!(!round.advance(), "an unfinished line does not advance");
        type_line(&mut round, 2);
        assert!(!round.is_over());
        assert!(round.advance());
        assert_eq!(
            (round.current().korean.as_str(), round.position()),
            ("나다", 2)
        );
        type_line(&mut round, 0);
        assert!(round.is_over());
        assert!(!round.advance(), "the last line has no successor");
        // 6 correct keystrokes, 2 errors in the first line.
        assert_eq!((round.keystrokes(), round.errors()), (6, 2));
        assert_eq!(round.accuracy(), 0.75);
    }

    #[test]
    fn empty_or_untypeable_targets_make_no_round() {
        assert!(Round::new(vec![]).unwrap().is_none());
        assert!(Round::new(vec![target("가"), target("abc")]).is_err());
    }
}
