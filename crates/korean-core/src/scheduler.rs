//! Review scheduling: when should a card come back?
//!
//! First scheduler: a fixed interval ladder. Each successful review climbs one step (two on
//! Easy), a lapse drops back to the first step.

use serde::{Deserialize, Serialize};

pub const MINUTE_MS: i64 = 60_000;
pub const DAY_MS: i64 = 24 * 60 * MINUTE_MS;

/// Intervals after a successful review, by step.
const LADDER_MS: [i64; 8] = [
    10 * MINUTE_MS,
    DAY_MS,
    3 * DAY_MS,
    7 * DAY_MS,
    16 * DAY_MS,
    35 * DAY_MS,
    80 * DAY_MS,
    180 * DAY_MS,
];
/// Wait before a lapsed card comes back.
const RELEARN_MS: i64 = MINUTE_MS;

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
}

/// Memory state of one (item, skill) card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryState {
    /// Rung of the ladder the card sits on.
    pub step: u32,
    pub due_at: i64,
    pub reps: u32,
    pub lapses: u32,
    pub last_review_at: i64,
}

impl MemoryState {
    pub fn is_due(&self, now_ms: i64) -> bool {
        self.due_at <= now_ms
    }
}

/// New state after answering `rating` at `now_ms`. `None` means the card was never reviewed.
pub fn review(previous: Option<&MemoryState>, rating: Rating, now_ms: i64) -> MemoryState {
    let (step, reps, lapses) = previous.map_or((0, 0, 0), |s| (s.step, s.reps, s.lapses));
    let top = LADDER_MS.len() as u32 - 1;
    let (next_step, interval, lapses) = match (previous, rating) {
        (_, Rating::Again) => (0, RELEARN_MS, lapses + u32::from(previous.is_some())),
        // A first-time card rated Hard stays on the first rung.
        (None, Rating::Hard) => (0, LADDER_MS[0], lapses),
        (Some(_), Rating::Hard) => (step, LADDER_MS[step as usize] / 2, lapses),
        (None, Rating::Good) => (0, LADDER_MS[0], lapses),
        (Some(_), Rating::Good) => {
            let s = (step + 1).min(top);
            (s, LADDER_MS[s as usize], lapses)
        }
        (None, Rating::Easy) => (1, LADDER_MS[1], lapses),
        (Some(_), Rating::Easy) => {
            let s = (step + 2).min(top);
            (s, LADDER_MS[s as usize], lapses)
        }
    };
    MemoryState {
        step: next_step,
        due_at: now_ms + interval.max(MINUTE_MS),
        reps: reps + 1,
        lapses,
        last_review_at: now_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_000 * DAY_MS;

    #[test]
    fn a_new_card_comes_back_soon_then_climbs_the_ladder() {
        let s1 = review(None, Rating::Good, NOW);
        assert_eq!((s1.step, s1.due_at - NOW), (0, 10 * MINUTE_MS));
        let s2 = review(Some(&s1), Rating::Good, s1.due_at);
        assert_eq!((s2.step, s2.due_at - s1.due_at), (1, DAY_MS));
        let s3 = review(Some(&s2), Rating::Good, s2.due_at);
        assert_eq!((s3.step, s3.due_at - s2.due_at), (2, 3 * DAY_MS));
        assert_eq!(s3.reps, 3);
    }

    #[test]
    fn easy_skips_a_rung_and_hard_repeats_it_with_half_the_wait() {
        let s = review(None, Rating::Easy, NOW);
        assert_eq!(s.step, 1);
        let easy = review(Some(&s), Rating::Easy, NOW);
        assert_eq!(easy.step, 3);
        let hard = review(Some(&easy), Rating::Hard, NOW);
        assert_eq!((hard.step, hard.due_at - NOW), (3, 7 * DAY_MS / 2));
    }

    #[test]
    fn a_lapse_resets_the_card_and_counts() {
        let mut s = review(None, Rating::Easy, NOW);
        s = review(Some(&s), Rating::Good, NOW);
        let lapsed = review(Some(&s), Rating::Again, NOW);
        assert_eq!((lapsed.step, lapsed.lapses), (0, 1));
        assert_eq!(lapsed.due_at - NOW, MINUTE_MS);
        // Failing a never-seen card is not a lapse.
        assert_eq!(review(None, Rating::Again, NOW).lapses, 0);
    }

    #[test]
    fn the_ladder_tops_out() {
        let mut s = review(None, Rating::Easy, NOW);
        for _ in 0..20 {
            s = review(Some(&s), Rating::Easy, NOW);
        }
        assert_eq!(s.step as usize, LADDER_MS.len() - 1);
        assert_eq!(s.due_at - NOW, 180 * DAY_MS);
    }
}
