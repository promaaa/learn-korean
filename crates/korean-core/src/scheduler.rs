//! Review scheduling with FSRS (Free Spaced Repetition Scheduler, ADR-003) through the reference
//! implementation `rs-fsrs`: default FSRS-5 weights, 90 % target retention, short-term learning
//! steps (1 / 5 / 10 minutes) before the first day-scale interval.

use rs_fsrs::{Card as FsrsCard, FSRS, Parameters, State as FsrsState};
use serde::{Deserialize, Serialize};

pub const MINUTE_MS: i64 = 60_000;
pub const DAY_MS: i64 = 24 * 60 * MINUTE_MS;

/// How well an answer went, as in Anki/FSRS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rating {
    Again = 1,
    Hard = 2,
    Good = 3,
    Easy = 4,
}

impl Rating {
    pub fn value(self) -> i64 {
        self as i64
    }

    fn fsrs(self) -> rs_fsrs::Rating {
        match self {
            Rating::Again => rs_fsrs::Rating::Again,
            Rating::Hard => rs_fsrs::Rating::Hard,
            Rating::Good => rs_fsrs::Rating::Good,
            Rating::Easy => rs_fsrs::Rating::Easy,
        }
    }
}

/// FSRS card phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    New = 0,
    Learning = 1,
    Review = 2,
    Relearning = 3,
}

impl Phase {
    pub fn from_code(code: i64) -> Phase {
        match code {
            1 => Phase::Learning,
            2 => Phase::Review,
            3 => Phase::Relearning,
            _ => Phase::New,
        }
    }

    pub fn code(self) -> i64 {
        self as i64
    }
}

/// Memory state of one (item, skill) card.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MemoryState {
    pub phase: Phase,
    /// Days for retrievability to fall to 90 %.
    pub stability: f64,
    /// 1 (easy) to 10 (hard).
    pub difficulty: f64,
    pub due_at: i64,
    pub last_review_at: i64,
    pub scheduled_days: i64,
    pub reps: u32,
    pub lapses: u32,
}

impl MemoryState {
    pub fn is_due(&self, now_ms: i64) -> bool {
        self.due_at <= now_ms
    }

    /// Probability of recalling the card at `now_ms`.
    pub fn retrievability(&self, now_ms: i64) -> f64 {
        if self.phase == Phase::New || self.stability <= 0.0 {
            return 0.0;
        }
        let elapsed_days = (now_ms - self.last_review_at).max(0) as f64 / DAY_MS as f64;
        Parameters::forgetting_curve(elapsed_days, self.stability)
    }
}

fn to_datetime(ms: i64) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp_millis(ms).unwrap_or_default()
}

fn to_fsrs(state: &MemoryState, now_ms: i64) -> FsrsCard {
    FsrsCard {
        due: to_datetime(state.due_at),
        stability: state.stability,
        difficulty: state.difficulty,
        elapsed_days: (now_ms - state.last_review_at).max(0) / DAY_MS,
        scheduled_days: state.scheduled_days,
        reps: state.reps as i32,
        lapses: state.lapses as i32,
        state: match state.phase {
            Phase::New => FsrsState::New,
            Phase::Learning => FsrsState::Learning,
            Phase::Review => FsrsState::Review,
            Phase::Relearning => FsrsState::Relearning,
        },
        last_review: to_datetime(state.last_review_at),
    }
}

/// New state after answering `rating` at `now_ms`. `None` means the card was never reviewed.
pub fn review(previous: Option<&MemoryState>, rating: Rating, now_ms: i64) -> MemoryState {
    let now = to_datetime(now_ms);
    let card = match previous {
        Some(state) => to_fsrs(state, now_ms),
        None => FsrsCard {
            due: now,
            last_review: now,
            ..FsrsCard::default()
        },
    };
    let next = FSRS::new(Parameters::default())
        .next(card, now, rating.fsrs())
        .card;
    MemoryState {
        phase: match next.state {
            FsrsState::New => Phase::New,
            FsrsState::Learning => Phase::Learning,
            FsrsState::Review => Phase::Review,
            FsrsState::Relearning => Phase::Relearning,
        },
        stability: next.stability,
        difficulty: next.difficulty,
        due_at: next.due.timestamp_millis(),
        last_review_at: now_ms,
        scheduled_days: next.scheduled_days,
        reps: next.reps.max(0) as u32,
        lapses: next.lapses.max(0) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_000 * DAY_MS;

    #[test]
    fn new_cards_go_through_short_learning_steps() {
        let again = review(None, Rating::Again, NOW);
        assert_eq!(
            (again.phase, again.due_at - NOW),
            (Phase::Learning, MINUTE_MS)
        );
        let hard = review(None, Rating::Hard, NOW);
        assert_eq!(hard.due_at - NOW, 5 * MINUTE_MS);
        let good = review(None, Rating::Good, NOW);
        assert_eq!(
            (good.phase, good.due_at - NOW),
            (Phase::Learning, 10 * MINUTE_MS)
        );
        let easy = review(None, Rating::Easy, NOW);
        assert_eq!(easy.phase, Phase::Review);
        assert!(
            easy.due_at - NOW >= DAY_MS,
            "easy graduates straight to days"
        );
    }

    #[test]
    fn successful_reviews_grow_the_interval_and_stability() {
        let mut state = review(None, Rating::Good, NOW);
        state = review(Some(&state), Rating::Good, state.due_at);
        assert_eq!(state.phase, Phase::Review);
        let mut interval = state.due_at - state.last_review_at;
        assert!(interval >= DAY_MS);
        for _ in 0..4 {
            let next = review(Some(&state), Rating::Good, state.due_at);
            let next_interval = next.due_at - next.last_review_at;
            assert!(next_interval > interval, "{next_interval} <= {interval}");
            assert!(next.stability > state.stability);
            (state, interval) = (next, next_interval);
        }
        assert_eq!(state.reps, 6);
        assert_eq!(state.lapses, 0);
    }

    #[test]
    fn forgetting_a_review_card_is_a_lapse_and_relearns_within_minutes() {
        let mut state = review(None, Rating::Easy, NOW);
        state = review(Some(&state), Rating::Good, state.due_at);
        let before = state.stability;
        let lapsed = review(Some(&state), Rating::Again, state.due_at);
        assert_eq!(lapsed.phase, Phase::Relearning);
        assert_eq!(lapsed.lapses, 1);
        assert!(lapsed.due_at - state.due_at <= 10 * MINUTE_MS);
        assert!(lapsed.stability < before);
        assert!(lapsed.difficulty > state.difficulty);
    }

    #[test]
    fn retrievability_decays_from_one_to_ninety_percent_at_stability() {
        let state = review(None, Rating::Easy, NOW);
        assert!((state.retrievability(NOW) - 1.0).abs() < 1e-9);
        let at_stability = NOW + (state.stability * DAY_MS as f64) as i64;
        assert!((state.retrievability(at_stability) - 0.9).abs() < 0.01);
        assert_eq!(
            Phase::from_code(Phase::Relearning.code()),
            Phase::Relearning
        );
    }
}
