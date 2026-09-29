//! Review scheduling as Anki does it with FSRS (ADR-003).
//!
//! - Memory model: FSRS-6 through `fsrs`, the library Anki itself uses. Its parameters are the
//!   defaults until they are optimized from the learner's own review log ([`optimize`]).
//! - Learning steps 1 min / 10 min for new cards, one 10 min relearning step after a lapse
//!   (Anki's defaults). Every answer, even within a step, updates the memory state.
//! - Review intervals are whole days for 90 % desired retention, fuzzed like Anki so cards
//!   learned together spread out, and ordered Hard < Good < Easy. A day starts at 4 a.m. local
//!   time, so a card due "tomorrow" is due from 4 a.m.

use fsrs::{ComputeParametersInput, FSRS, FSRS6_DEFAULT_DECAY, FSRSItem, FSRSReview, NextStates};
use serde::{Deserialize, Serialize};

pub const MINUTE_MS: i64 = 60_000;
pub const DAY_MS: i64 = 24 * 60 * MINUTE_MS;

/// Probability of recall a review is scheduled for (Anki's default).
pub const DESIRED_RETENTION: f32 = 0.9;
/// Longest review interval, in days (Anki's default).
pub const MAX_INTERVAL_DAYS: u32 = 36_500;
/// Delays of the learning steps of a new card.
const LEARNING_STEPS_MS: [i64; 2] = [MINUTE_MS, 10 * MINUTE_MS];
/// Delays of the relearning steps after a lapse.
const RELEARNING_STEPS_MS: [i64; 1] = [10 * MINUTE_MS];
/// A new day starts at 4 a.m. local time (Anki's default), not at midnight.
const DAY_ROLLOVER_MS: i64 = 4 * 60 * MINUTE_MS;

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

    pub fn from_value(value: i64) -> Option<Rating> {
        match value {
            1 => Some(Rating::Again),
            2 => Some(Rating::Hard),
            3 => Some(Rating::Good),
            4 => Some(Rating::Easy),
            _ => None,
        }
    }
}

/// Card phase, as in Anki.
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

    /// Learning or relearning: waiting minutes, not days.
    pub fn is_learning(self) -> bool {
        matches!(self, Phase::Learning | Phase::Relearning)
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
    /// Review interval in days; 0 while (re)learning.
    pub scheduled_days: i64,
    pub reps: u32,
    pub lapses: u32,
    /// Index of the current step while (re)learning.
    pub step: u32,
}

impl MemoryState {
    pub fn is_due(&self, now_ms: i64) -> bool {
        self.due_at <= now_ms
    }
}

/// One past answer of a card: its rating and when it was given (unix milliseconds).
pub type Answered = (Rating, i64);

#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    #[error("invalid FSRS parameters: {0}")]
    Parameters(fsrs::FSRSError),
    #[error("optimizing FSRS parameters failed: {0}")]
    Optimize(fsrs::FSRSError),
}

/// FSRS parameters and the learner's day boundaries.
#[derive(Debug, Clone)]
pub struct Scheduler {
    fsrs: FSRS,
    decay: f32,
    utc_offset_ms: i64,
}

impl Scheduler {
    /// `parameters`: optimized FSRS parameters, or empty for the defaults.
    pub fn new(parameters: &[f32], utc_offset_seconds: i32) -> Result<Self, SchedulerError> {
        Ok(Scheduler {
            fsrs: FSRS::new(parameters).map_err(SchedulerError::Parameters)?,
            decay: parameters.get(20).copied().unwrap_or(FSRS6_DEFAULT_DECAY),
            utc_offset_ms: i64::from(utc_offset_seconds) * 1_000,
        })
    }

    /// Default FSRS-6 parameters, until the learner has enough reviews to optimize them.
    pub fn with_defaults(utc_offset_seconds: i32) -> Self {
        Self::new(&[], utc_offset_seconds).expect("default FSRS parameters are valid")
    }

    /// Learner day number of an instant; days roll over at 4 a.m. local time.
    pub fn day(&self, ms: i64) -> i64 {
        (ms + self.utc_offset_ms - DAY_ROLLOVER_MS).div_euclid(DAY_MS)
    }

    /// First instant of learner day `day`.
    pub fn day_start(&self, day: i64) -> i64 {
        day * DAY_MS + DAY_ROLLOVER_MS - self.utc_offset_ms
    }

    /// Probability of recalling the card at `now_ms`; 0 for a card never reviewed.
    pub fn retrievability(&self, state: &MemoryState, now_ms: i64) -> f64 {
        if state.phase == Phase::New || state.stability <= 0.0 {
            return 0.0;
        }
        let elapsed_days = (now_ms - state.last_review_at).max(0) as f32 / DAY_MS as f32;
        let memory = fsrs::MemoryState {
            stability: state.stability as f32,
            difficulty: state.difficulty as f32,
        };
        f64::from(fsrs::current_retrievability(
            memory,
            elapsed_days,
            self.decay,
        ))
    }

    /// FSRS outcomes of each rating. A state that FSRS rejects (never written by this
    /// scheduler) starts over as a new card.
    fn next_states(&self, previous: Option<&MemoryState>, now_ms: i64) -> NextStates {
        let memory = previous
            .filter(|s| s.phase != Phase::New && s.stability > 0.0)
            .map(|s| fsrs::MemoryState {
                stability: s.stability as f32,
                difficulty: s.difficulty as f32,
            });
        let elapsed = previous.map_or(0, |s| {
            (self.day(now_ms) - self.day(s.last_review_at)).clamp(0, i64::from(u32::MAX)) as u32
        });
        self.fsrs
            .next_states(memory, DESIRED_RETENTION, elapsed)
            .or_else(|_| self.fsrs.next_states(None, DESIRED_RETENTION, 0))
            .expect("a new card always has next states")
    }

    /// New state after answering `rating` at `now_ms`. `None` means the card was never
    /// reviewed. `fuzz` is uniform in [0, 1) and stable for a given card and review, as in
    /// Anki; it picks the day within the fuzz range of a review interval.
    pub fn review(
        &self,
        previous: Option<&MemoryState>,
        rating: Rating,
        now_ms: i64,
        fuzz: f64,
    ) -> MemoryState {
        let next = self.next_states(previous, now_ms);
        let memory = match rating {
            Rating::Again => next.again.memory,
            Rating::Hard => next.hard.memory,
            Rating::Good => next.good.memory,
            Rating::Easy => next.easy.memory,
        };
        let phase = previous.map_or(Phase::New, |s| s.phase);
        let reps = previous.map_or(0, |s| s.reps) + 1;
        let mut lapses = previous.map_or(0, |s| s.lapses);
        let fuzz = fuzz as f32;

        let (phase, step, wait) = match phase {
            current @ (Phase::New | Phase::Learning | Phase::Relearning) => {
                let (phase, steps) = if current == Phase::Relearning {
                    (Phase::Relearning, &RELEARNING_STEPS_MS[..])
                } else {
                    (Phase::Learning, &LEARNING_STEPS_MS[..])
                };
                // A new card starts at its first step.
                let step = previous
                    .filter(|_| current != Phase::New)
                    .map_or(0, |s| (s.step as usize).min(steps.len() - 1));
                match rating {
                    Rating::Again => (phase, 0, Wait::Ms(steps[0])),
                    Rating::Hard => (phase, step, Wait::Ms(hard_delay(steps, step))),
                    Rating::Good if step + 1 < steps.len() => {
                        (phase, step + 1, Wait::Ms(steps[step + 1]))
                    }
                    Rating::Good => (Phase::Review, 0, Wait::Days(graduate(&next, fuzz).0)),
                    Rating::Easy => (Phase::Review, 0, Wait::Days(graduate(&next, fuzz).1)),
                }
            }
            Phase::Review => {
                let scheduled = previous.map_or(0, |s| s.scheduled_days.max(0) as u32);
                let (hard, good, easy) = passing_intervals(&next, scheduled, fuzz);
                match rating {
                    Rating::Again => {
                        lapses += 1;
                        (Phase::Relearning, 0, Wait::Ms(RELEARNING_STEPS_MS[0]))
                    }
                    Rating::Hard => (Phase::Review, 0, Wait::Days(hard)),
                    Rating::Good => (Phase::Review, 0, Wait::Days(good)),
                    Rating::Easy => (Phase::Review, 0, Wait::Days(easy)),
                }
            }
        };
        let (due_at, scheduled_days) = match wait {
            Wait::Ms(ms) => (now_ms + ms, 0),
            Wait::Days(days) => (
                self.day_start(self.day(now_ms) + i64::from(days)),
                i64::from(days),
            ),
        };
        MemoryState {
            phase,
            stability: f64::from(memory.stability),
            difficulty: f64::from(memory.difficulty),
            due_at,
            last_review_at: now_ms,
            scheduled_days,
            reps,
            lapses,
            step: step as u32,
        }
    }

    /// Stability and difficulty after replaying a card's answers, oldest first: what
    /// [`Scheduler::review`] would have produced with these parameters. `None` without
    /// answers.
    pub fn memory(&self, history: &[Answered]) -> Option<(f64, f64)> {
        let reviews = self.fsrs_reviews(history);
        if reviews.is_empty() {
            return None;
        }
        let memory = self.fsrs.memory_state(FSRSItem { reviews }, None).ok()?;
        Some((f64::from(memory.stability), f64::from(memory.difficulty)))
    }

    fn fsrs_reviews(&self, history: &[Answered]) -> Vec<FSRSReview> {
        let mut previous_day = None;
        history
            .iter()
            .map(|&(rating, at)| {
                let day = self.day(at);
                let delta_t = previous_day.map_or(0, |p: i64| (day - p).max(0) as u32);
                previous_day = Some(day);
                FSRSReview {
                    rating: rating.value() as u32,
                    delta_t,
                }
            })
            .collect()
    }

    /// Training items as Anki builds them: one per answer given on a later day than the
    /// card's previous one, holding every answer of the card up to it.
    fn training_items<'a>(
        &self,
        histories: impl IntoIterator<Item = &'a [Answered]>,
    ) -> Vec<FSRSItem> {
        let mut items = Vec::new();
        for history in histories {
            let reviews = self.fsrs_reviews(history);
            for end in 1..reviews.len() {
                if reviews[end].delta_t > 0 {
                    items.push(FSRSItem {
                        reviews: reviews[..=end].to_vec(),
                    });
                }
            }
        }
        items
    }

    /// Log loss of these parameters on `items`: lower predicts the learner's answers better.
    fn log_loss(&self, items: &[FSRSItem]) -> Option<f32> {
        self.fsrs
            .evaluate(items.to_vec(), |_| true)
            .ok()
            .map(|e| e.log_loss)
    }
}

enum Wait {
    Ms(i64),
    Days(u32),
}

/// Anki's Hard delay: between the first two steps on the first step (or 1.5 × a single
/// step), the current step otherwise.
fn hard_delay(steps: &[i64], step: usize) -> i64 {
    match (step, steps.get(1)) {
        (0, Some(second)) => (steps[0] + second) / 2,
        (0, None) => (steps[0] * 3 / 2).min(steps[0] + DAY_MS),
        _ => steps[step],
    }
}

/// Good and Easy intervals when a card leaves (re)learning.
fn graduate(next: &NextStates, fuzz: f32) -> (u32, u32) {
    let good = with_fuzz(fuzz, next.good.interval.round().max(1.0), 1);
    let easy = with_fuzz(fuzz, next.easy.interval.round().max(1.0), good + 1);
    (good, easy)
}

/// Hard, Good and Easy intervals of a review card, as Anki orders them.
fn passing_intervals(next: &NextStates, scheduled: u32, fuzz: f32) -> (u32, u32, u32) {
    let hard = with_fuzz(
        fuzz,
        next.hard.interval,
        minimum_fuzz_interval(next.hard.interval, scheduled).max(1),
    );
    let good = with_fuzz(
        fuzz,
        next.good.interval,
        minimum_fuzz_interval(next.good.interval, scheduled).max(hard + 1),
    );
    let easy = with_fuzz(
        fuzz,
        next.easy.interval,
        minimum_fuzz_interval(next.easy.interval, scheduled).max(good + 1),
    );
    (hard, good, easy)
}

/// Anki's fuzz ranges: ±15 % of the days between 2.5 and 7, ±10 % up to 20, ±5 % beyond, plus
/// one day.
const FUZZ_RANGES: [(f32, f32, f32); 3] =
    [(2.5, 7.0, 0.15), (7.0, 20.0, 0.1), (20.0, f32::MAX, 0.05)];

fn fuzz_delta(interval: f32) -> f32 {
    if interval < 2.5 {
        return 0.0;
    }
    FUZZ_RANGES
        .iter()
        .fold(1.0, |delta, &(start, end, factor)| {
            delta + factor * (interval.min(end) - start).max(0.0)
        })
}

/// Fuzz range of `interval`, kept within `[minimum, MAX_INTERVAL_DAYS]`.
fn fuzz_bounds(interval: f32, minimum: u32) -> (u32, u32) {
    let maximum = MAX_INTERVAL_DAYS;
    let minimum = minimum.min(maximum);
    let interval = interval.clamp(minimum as f32, maximum as f32);
    let delta = fuzz_delta(interval);
    let lower = ((interval - delta).round() as u32).clamp(minimum, maximum);
    let mut upper = ((interval + delta).round() as u32).clamp(minimum, maximum);
    if upper == lower && upper > 2 && upper < maximum {
        upper = lower + 1;
    }
    (lower, upper)
}

fn with_fuzz(fuzz: f32, interval: f32, minimum: u32) -> u32 {
    let (lower, upper) = fuzz_bounds(interval, minimum);
    (lower as f32 + fuzz * (1 + upper - lower) as f32).floor() as u32
}

/// Lowest interval fuzz may pick after a review: never shorter than the previous interval
/// unless FSRS shortened it well beyond the fuzz range.
fn minimum_fuzz_interval(interval: f32, previous: u32) -> u32 {
    let (_, upper) = fuzz_bounds(interval, 1);
    if interval.round() as u32 > previous {
        previous + 1
    } else if previous <= upper {
        previous
    } else {
        0
    }
}

/// Parameters fitted to the learner's answers, if they predict them better than `current`'s
/// (as Anki does); `None` keeps the current ones, e.g. with too few reviews to fit.
/// `histories` holds every card's answers, oldest first.
pub fn optimize<'a>(
    current: &Scheduler,
    histories: impl IntoIterator<Item = &'a [Answered]>,
) -> Result<Option<Vec<f32>>, SchedulerError> {
    let items = current.training_items(histories);
    if items.is_empty() {
        return Ok(None);
    }
    let fitted = fsrs::compute_parameters(ComputeParametersInput {
        train_set: items.clone(),
        enable_short_term: true,
        num_relearning_steps: Some(RELEARNING_STEPS_MS.len()),
        ..ComputeParametersInput::default()
    })
    .map_err(SchedulerError::Optimize)?;
    let candidate = Scheduler::new(&fitted, 0)?;
    Ok(
        match (candidate.log_loss(&items), current.log_loss(&items)) {
            (Some(new), Some(old)) if new < old => Some(fitted),
            (Some(_), None) => Some(fitted),
            _ => None,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Monday, 12:00 UTC.
    const NOW: i64 = 1_790_683_200_000;

    fn scheduler() -> Scheduler {
        Scheduler::with_defaults(0)
    }

    fn review(previous: Option<&MemoryState>, rating: Rating, at: i64) -> MemoryState {
        scheduler().review(previous, rating, at, 0.5)
    }

    #[test]
    fn new_cards_go_through_anki_learning_steps() {
        let again = review(None, Rating::Again, NOW);
        assert_eq!(
            (again.phase, again.due_at - NOW),
            (Phase::Learning, MINUTE_MS)
        );
        let hard = review(None, Rating::Hard, NOW);
        assert_eq!(hard.due_at - NOW, MINUTE_MS * 11 / 2);
        let good = review(None, Rating::Good, NOW);
        assert_eq!(
            (good.phase, good.step, good.due_at - NOW),
            (Phase::Learning, 1, 10 * MINUTE_MS)
        );
        let graduated = review(Some(&good), Rating::Good, good.due_at);
        assert_eq!(graduated.phase, Phase::Review);
        assert!(graduated.scheduled_days >= 1);
        let easy = review(None, Rating::Easy, NOW);
        assert_eq!(easy.phase, Phase::Review);
        assert!(easy.scheduled_days > graduated.scheduled_days);
    }

    #[test]
    fn hard_repeats_the_step_and_again_restarts_the_steps() {
        let second = review(None, Rating::Good, NOW);
        let hard = review(Some(&second), Rating::Hard, second.due_at);
        assert_eq!(
            (hard.step, hard.due_at - second.due_at),
            (1, 10 * MINUTE_MS)
        );
        let again = review(Some(&hard), Rating::Again, hard.due_at);
        assert_eq!((again.step, again.due_at - hard.due_at), (0, MINUTE_MS));
    }

    #[test]
    fn review_cards_are_due_at_the_start_of_a_learner_day() {
        let s = Scheduler::with_defaults(9 * 3_600); // UTC+9
        let state = s.review(None, Rating::Easy, NOW, 0.5);
        assert_eq!(s.day_start(s.day(state.due_at)), state.due_at);
        assert_eq!(s.day(state.due_at) - s.day(NOW), state.scheduled_days);
        // 03:59 local still belongs to the previous day, 04:00 starts a new one.
        let four_am = s.day_start(s.day(NOW) + 1);
        assert_eq!(s.day(four_am - 1), s.day(NOW));
        assert_eq!(s.day(four_am), s.day(NOW) + 1);
    }

    #[test]
    fn successful_reviews_grow_intervals_ordered_hard_good_easy() {
        let mut state = review(None, Rating::Easy, NOW);
        for _ in 0..5 {
            let hard = review(Some(&state), Rating::Hard, state.due_at);
            let good = review(Some(&state), Rating::Good, state.due_at);
            let easy = review(Some(&state), Rating::Easy, state.due_at);
            assert!(hard.scheduled_days < good.scheduled_days);
            assert!(good.scheduled_days < easy.scheduled_days);
            assert!(good.scheduled_days > state.scheduled_days);
            assert!(good.stability > state.stability);
            state = good;
        }
        assert_eq!((state.reps, state.lapses), (6, 0));
    }

    #[test]
    fn a_forgotten_review_card_lapses_into_a_relearning_step() {
        let easy = review(None, Rating::Easy, NOW);
        let state = review(Some(&easy), Rating::Good, easy.due_at);
        let lapsed = review(Some(&state), Rating::Again, state.due_at);
        assert_eq!(
            (lapsed.phase, lapsed.lapses, lapsed.due_at - state.due_at),
            (Phase::Relearning, 1, 10 * MINUTE_MS)
        );
        assert!(lapsed.stability < state.stability);
        assert!(lapsed.difficulty > state.difficulty);
        let relearned = review(Some(&lapsed), Rating::Good, lapsed.due_at);
        assert_eq!(relearned.phase, Phase::Review);
        assert!(relearned.scheduled_days >= 1);
        assert!(relearned.scheduled_days < state.scheduled_days);
    }

    #[test]
    fn fuzz_spreads_cards_learned_together_within_anki_bounds() {
        let s = scheduler();
        let mut state = s.review(None, Rating::Easy, NOW, 0.5);
        state = s.review(Some(&state), Rating::Good, state.due_at, 0.5);
        let days: Vec<i64> = [0.0, 0.5, 0.999]
            .iter()
            .map(|&f| {
                s.review(Some(&state), Rating::Good, state.due_at, f)
                    .scheduled_days
            })
            .collect();
        assert!(days[0] < days[2], "{days:?}");
        assert!(days[0] <= days[1] && days[1] <= days[2]);
        // 1 day + 15 % of 4.5 days + 10 % of 13 days = 2.975 days each way.
        assert_eq!(fuzz_bounds(20.0, 1), (17, 23));
        assert_eq!(fuzz_bounds(2.0, 1), (2, 2));
    }

    #[test]
    fn replaying_the_history_reproduces_the_incremental_memory_state() {
        let s = Scheduler::with_defaults(3_600);
        let answers = [
            (Rating::Good, NOW),
            (Rating::Again, NOW + 2 * MINUTE_MS),
            (Rating::Good, NOW + 5 * MINUTE_MS),
            (Rating::Good, NOW + DAY_MS),
            (Rating::Hard, NOW + 4 * DAY_MS),
            (Rating::Easy, NOW + 12 * DAY_MS),
        ];
        let mut state = None;
        for (rating, at) in answers {
            state = Some(s.review(state.as_ref(), rating, at, 0.3));
        }
        let state = state.unwrap();
        let (stability, difficulty) = s.memory(&answers).unwrap();
        assert!(
            (stability - state.stability).abs() < 1e-3,
            "{stability} vs {}",
            state.stability
        );
        assert!((difficulty - state.difficulty).abs() < 1e-3);
        assert_eq!(s.memory(&[]), None);
    }

    #[test]
    fn retrievability_is_ninety_percent_after_stability_days() {
        let s = scheduler();
        let state = s.review(None, Rating::Easy, NOW, 0.5);
        assert!((s.retrievability(&state, NOW) - 1.0).abs() < 1e-6);
        let at_stability = NOW + (state.stability * DAY_MS as f64) as i64;
        assert!((s.retrievability(&state, at_stability) - 0.9).abs() < 0.01);
    }

    /// A learner who forgets much faster than the defaults assume gets shorter intervals.
    #[test]
    fn optimizing_on_a_forgetful_learner_shortens_intervals() {
        let s = scheduler();
        let mut histories: Vec<Vec<Answered>> = Vec::new();
        for card in 0..120i64 {
            let start = NOW + card * MINUTE_MS;
            let mut history = vec![
                (Rating::Good, start),
                (Rating::Good, start + 10 * MINUTE_MS),
            ];
            let mut at = start;
            for (n, gap) in [1, 3, 8].into_iter().enumerate() {
                at += gap * DAY_MS;
                // Most answers after a few days are forgotten.
                let forgot = (card + n as i64) % 3 != 0;
                history.push((if forgot { Rating::Again } else { Rating::Good }, at));
                if forgot {
                    history.push((Rating::Good, at + 10 * MINUTE_MS));
                }
            }
            histories.push(history);
        }
        let fitted = optimize(&s, histories.iter().map(Vec::as_slice))
            .unwrap()
            .expect("a better fit than the defaults");
        let tuned = Scheduler::new(&fitted, 0).unwrap();
        let graduated = |s: &Scheduler| {
            let first = s.review(None, Rating::Good, NOW, 0.5);
            let second = s.review(Some(&first), Rating::Good, first.due_at, 0.5);
            s.review(Some(&second), Rating::Good, second.due_at + DAY_MS, 0.5)
                .scheduled_days
        };
        assert!(graduated(&tuned) < graduated(&s));
    }

    #[test]
    fn optimizing_without_enough_history_keeps_the_current_parameters() {
        let s = scheduler();
        let one = [(Rating::Good, NOW)];
        assert_eq!(optimize(&s, [&one[..]]).unwrap(), None);
    }
}
