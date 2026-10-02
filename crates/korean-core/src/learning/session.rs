//! A running review session: serves exercises, checks answers, updates memory.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::exercise::{self, Exercise, ExerciseView, Expected, Prompt};
use super::plan::{Card, Plan};
use crate::content::{Glossary, Item, Skill};
use crate::rng::Rng;
use crate::scheduler::{MemoryState, Rating, Scheduler};
use crate::{progression, scoring};

/// A missed card comes back after this many other cards.
const RETRY_GAP: usize = 3;
/// A card is retried at most this many times in one session.
const MAX_RETRIES: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Answer {
    Choice {
        index: usize,
    },
    /// Indices into the displayed chunks, in the order the learner placed them.
    Order {
        order: Vec<usize>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Feedback {
    pub correct: bool,
    /// Right option of a multiple-choice exercise.
    pub correct_index: Option<usize>,
    /// English of each option (reply game), empty when the options already are English.
    pub translations: Vec<String>,
    /// English of each Korean word on screen after answering (the item and, in the reply game,
    /// the options), keyed by the word as displayed, punctuation included.
    pub glosses: BTreeMap<String, String>,
    pub rating: Rating,
    pub korean: String,
    pub english: String,
    pub note: Option<String>,
    /// Consecutive correct answers in this session.
    pub streak: u32,
    /// The card will come back later in this session.
    pub retry: bool,
    /// When the scheduler will show this card again, from now.
    pub due_in_ms: i64,
    /// Experience earned by this answer.
    pub xp: u32,
}

/// Everything the caller must persist after an answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub feedback: Feedback,
    pub card: Card,
    pub state: MemoryState,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub done: usize,
    pub remaining: usize,
    pub correct: usize,
    pub streak: u32,
    pub best_streak: u32,
    /// Experience earned in this session.
    pub xp: u64,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SessionError {
    #[error("no exercise is waiting for an answer")]
    NoCurrentExercise,
    #[error("answer does not fit the exercise")]
    InvalidAnswer,
    #[error("no intro card is on screen")]
    NoIntro,
}

pub struct Session {
    queue: VecDeque<Card>,
    items: HashMap<String, Item>,
    glossary: Glossary,
    states: HashMap<Card, MemoryState>,
    scheduler: Scheduler,
    retries: HashMap<Card, u8>,
    /// Items whose intro card was shown in this session.
    introduced: HashSet<String>,
    current: Option<Exercise>,
    rng: Rng,
    progress: Progress,
}

impl Session {
    /// `items` must contain every item of the plan and serves as the distractor pool.
    pub fn new(
        plan: Plan,
        items: Vec<Item>,
        states: HashMap<Card, MemoryState>,
        scheduler: Scheduler,
        glossary: Glossary,
        seed: u64,
    ) -> Self {
        let items: HashMap<String, Item> = items.into_iter().map(|i| (i.id.clone(), i)).collect();
        let queue: VecDeque<Card> = plan
            .cards
            .into_iter()
            .filter(|c| items.contains_key(&c.item_id))
            .collect();
        let progress = Progress {
            remaining: queue.len(),
            ..Progress::default()
        };
        Session {
            queue,
            items,
            glossary,
            states,
            scheduler,
            retries: HashMap::new(),
            introduced: HashSet::new(),
            current: None,
            rng: Rng::new(seed),
            progress,
        }
    }

    pub fn progress(&self) -> Progress {
        self.progress
    }

    /// The exercise to play now; the same one until it is answered. `None` when finished.
    ///
    /// An item never answered is first shown on an intro card (see [`Session::seen`]), so its
    /// first answer is not a guess.
    pub fn current(&mut self) -> Option<&ExerciseView> {
        if self.current.is_none() {
            let card = self.queue.pop_front()?;
            let item = &self.items[&card.item_id];
            let never_seen = card.skill == Skill::Listening && !self.states.contains_key(&card);
            self.current = Some(
                if never_seen && self.introduced.insert(card.item_id.clone()) {
                    exercise::intro(&card, item, &self.glossary)
                } else {
                    let pool: Vec<&Item> = self
                        .items
                        .values()
                        .filter(|o| o.id.split('/').next() == item.id.split('/').next())
                        .collect();
                    exercise::build(&card, item, &pool, &mut self.rng)
                },
            );
        }
        self.current.as_ref().map(|e| &e.view)
    }

    /// Dismisses the intro card on screen. Nothing is recorded: its card comes back to be
    /// answered after a few others, as a missed card does.
    pub fn seen(&mut self) -> Result<(), SessionError> {
        let Some(exercise) = self.current.take_if(|e| e.expected == Expected::Seen) else {
            return Err(SessionError::NoIntro);
        };
        let at = RETRY_GAP.min(self.queue.len());
        self.queue.insert(at, exercise.view.card);
        Ok(())
    }

    pub fn answer(
        &mut self,
        answer: &Answer,
        elapsed_ms: u64,
        now_ms: i64,
    ) -> Result<Outcome, SessionError> {
        let exercise = self
            .current
            .as_ref()
            .ok_or(SessionError::NoCurrentExercise)?;
        let correct = check(&exercise.view.prompt, &exercise.expected, answer)?;
        let exercise = self.current.take().expect("checked above");
        let card = exercise.view.card;
        let item = &self.items[&card.item_id];
        let chunks = item.build_chunks().len();
        let previous = self.states.get(&card);
        let first_review = previous.is_none();
        let mut rating = scoring::grade(card.skill, correct, elapsed_ms, chunks);
        if self.introduced.contains(&card.item_id) {
            // Introduced minutes ago: a quick answer is short-term memory, not yet recall.
            rating = rating.min(Rating::Good);
        }
        let reps = previous.map_or(0, |s| s.reps);
        let state = self
            .scheduler
            .review(previous, rating, now_ms, fuzz_factor(&card, reps));
        self.states.insert(card.clone(), state);

        let p = &mut self.progress;
        p.done += 1;
        let mut retry = false;
        if correct {
            p.correct += 1;
            p.streak += 1;
            p.best_streak = p.best_streak.max(p.streak);
        } else {
            p.streak = 0;
            let tries = self.retries.entry(card.clone()).or_default();
            if *tries < MAX_RETRIES {
                *tries += 1;
                retry = true;
                let at = RETRY_GAP.min(self.queue.len());
                self.queue.insert(at, card.clone());
            }
        }
        p.remaining = self.queue.len();
        let xp = progression::xp_for_answer(correct, rating, p.streak, first_review);
        p.xp += u64::from(xp);
        let options: &[String] = match &exercise.view.prompt {
            Prompt::Response { options, .. } => options,
            Prompt::Listening { .. } | Prompt::Build { .. } | Prompt::Intro { .. } => &[],
        };
        let glosses = self.glossary.glosses(
            std::iter::once(item.korean.as_str()).chain(options.iter().map(String::as_str)),
        );
        Ok(Outcome {
            feedback: Feedback {
                correct,
                correct_index: match exercise.expected {
                    Expected::Choice(index) => Some(index),
                    Expected::Order(_) | Expected::Seen => None,
                },
                translations: exercise.translations,
                glosses,
                rating,
                korean: item.korean.clone(),
                english: item.english.clone(),
                note: item.note.clone(),
                streak: p.streak,
                retry,
                due_in_ms: state.due_at - now_ms,
                xp,
            },
            card,
            state,
            elapsed_ms,
        })
    }
}

/// Anki's fuzz factor: uniform in [0, 1), the same for a card and its number of reviews, so
/// replaying an answer (or answering on another device) picks the same day.
fn fuzz_factor(card: &Card, reps: u32) -> f64 {
    // FNV-1a: stable across builds and platforms, unlike `DefaultHasher`.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let bytes = card
        .item_id
        .bytes()
        .chain([0])
        .chain(card.skill.as_str().bytes());
    for byte in bytes.chain(reps.to_le_bytes()) {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
    }
    (Rng::new(hash).next_u64() >> 11) as f64 / (1u64 << 53) as f64
}

/// Whether `answer` solves the exercise; `InvalidAnswer` if it does not even fit it.
fn check(prompt: &Prompt, expected: &Expected, answer: &Answer) -> Result<bool, SessionError> {
    match (prompt, expected, answer) {
        (
            Prompt::Listening { options, .. } | Prompt::Response { options, .. },
            Expected::Choice(correct),
            Answer::Choice { index },
        ) if *index < options.len() => Ok(index == correct),
        (Prompt::Build { chunks, .. }, Expected::Order(solution), Answer::Order { order }) => {
            let mut seen = vec![false; chunks.len()];
            let is_permutation = order.len() == chunks.len()
                && order
                    .iter()
                    .all(|&i| i < seen.len() && !std::mem::replace(&mut seen[i], true));
            if !is_permutation {
                return Err(SessionError::InvalidAnswer);
            }
            Ok(order.iter().map(|&i| &chunks[i]).eq(solution.iter()))
        }
        _ => Err(SessionError::InvalidAnswer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{ItemKind, Register};
    use crate::scheduler::MINUTE_MS;

    const NOW: i64 = 1_000_000_000;

    fn sentence(id: &str, korean: &str, english: &str) -> Item {
        Item {
            id: id.into(),
            kind: ItemKind::Sentence,
            korean: korean.into(),
            english: english.into(),
            context: "t".into(),
            register: Register::Polite,
            note: Some("note".into()),
            lexemes: vec![],
            distractors: vec![
                format!("{english} 1"),
                format!("{english} 2"),
                format!("{english} 3"),
            ],
            replies: None,
            chunks: None,
            image: None,
        }
    }

    /// `n` listening cards, `seen` before (in learning steps) or never.
    fn session_of(n: usize, seen: bool) -> Session {
        let items: Vec<Item> = (0..n)
            .map(|i| {
                let korean = format!("{}예요.", char::from_u32(0xAC00 + i as u32).unwrap());
                sentence(&format!("a/{i}"), &korean, &format!("E{i}"))
            })
            .collect();
        let cards: Vec<Card> = items
            .iter()
            .map(|i| Card::new(&i.id, Skill::Listening))
            .collect();
        let scheduler = Scheduler::with_defaults(0);
        let states = cards
            .iter()
            .filter(|_| seen)
            .map(|c| {
                let state = scheduler.review(None, Rating::Good, NOW - MINUTE_MS * 60, 0.5);
                (c.clone(), state)
            })
            .collect();
        let plan = Plan {
            cards,
            due: 0,
            new: n,
        };
        Session::new(plan, items, states, scheduler, Glossary::default(), 7)
    }

    fn session(n: usize) -> Session {
        session_of(n, true)
    }

    fn correct_index(s: &mut Session) -> usize {
        let view = s.current().unwrap().clone();
        let english = &s.items[&view.card.item_id].english;
        let Prompt::Listening { options, .. } = &view.prompt else {
            panic!("listening expected");
        };
        options.iter().position(|o| o == english).unwrap()
    }

    #[test]
    fn correct_answers_advance_and_build_a_streak() {
        let mut s = session(3);
        for expected_streak in 1..=3 {
            let index = correct_index(&mut s);
            let out = s.answer(&Answer::Choice { index }, 2_000, NOW).unwrap();
            assert!(out.feedback.correct);
            assert_eq!(out.feedback.rating, Rating::Easy);
            assert_eq!(out.feedback.streak, expected_streak);
            assert_eq!(out.state.reps, 2);
            assert!(
                out.feedback.due_in_ms >= crate::scheduler::DAY_MS,
                "easy graduates to days"
            );
        }
        assert!(s.current().is_none());
        let p = s.progress();
        assert_eq!(
            (p.done, p.correct, p.best_streak, p.remaining),
            (3, 3, 3, 0)
        );
    }

    #[test]
    fn a_new_card_is_introduced_then_asked_after_a_gap() {
        let mut s = session_of(5, false);
        let first = s.current().unwrap().clone();
        let Prompt::Intro { english, .. } = &first.prompt else {
            panic!("intro expected");
        };
        assert_eq!(english, "E0");
        assert_eq!(
            s.answer(&Answer::Choice { index: 0 }, 1_000, NOW),
            Err(SessionError::InvalidAnswer)
        );
        s.seen().unwrap();
        assert_eq!(
            s.progress(),
            Progress {
                remaining: 5,
                ..Progress::default()
            }
        );

        // Intros of the next cards, then the first card's question, RETRY_GAP cards later.
        for _ in 0..RETRY_GAP {
            assert!(matches!(s.current().unwrap().prompt, Prompt::Intro { .. }));
            s.seen().unwrap();
        }
        assert_eq!(s.current().unwrap().card, first.card);
        assert_eq!(s.seen(), Err(SessionError::NoIntro));
        let wrong = (correct_index(&mut s) + 1) % 4;
        let out = s
            .answer(&Answer::Choice { index: wrong }, 1_000, NOW)
            .unwrap();
        assert!(out.feedback.retry);

        // Retried without a second intro; fast answers in the session of the intro are not Easy.
        let mut ratings = Vec::new();
        while let Some(view) = s.current() {
            if matches!(view.prompt, Prompt::Intro { .. }) {
                s.seen().unwrap();
                continue;
            }
            let index = correct_index(&mut s);
            ratings.push(
                s.answer(&Answer::Choice { index }, 1_000, NOW)
                    .unwrap()
                    .feedback
                    .rating,
            );
        }
        assert_eq!(ratings, [Rating::Good; 5]);
    }

    #[test]
    fn a_miss_resets_the_streak_and_comes_back_after_a_gap() {
        let mut s = session(5);
        let first = s.current().unwrap().card.clone();
        let wrong = (correct_index(&mut s) + 1) % 4;
        let out = s
            .answer(&Answer::Choice { index: wrong }, 1_000, NOW)
            .unwrap();
        assert!(!out.feedback.correct && out.feedback.retry);
        assert_eq!(out.feedback.rating, Rating::Again);
        assert_eq!(out.feedback.streak, 0);
        assert_eq!(s.progress().remaining, 5);
        let mut order = Vec::new();
        while let Some(view) = s.current() {
            order.push(view.card.clone());
            let index = correct_index(&mut s);
            s.answer(&Answer::Choice { index }, 5_000, NOW).unwrap();
        }
        assert_eq!(order.iter().position(|c| *c == first), Some(RETRY_GAP));
    }

    #[test]
    fn retries_are_bounded() {
        let mut s = session(1);
        let mut answers = 0;
        while s.current().is_some() {
            let wrong = (correct_index(&mut s) + 1) % 4;
            s.answer(&Answer::Choice { index: wrong }, 1_000, NOW)
                .unwrap();
            answers += 1;
        }
        assert_eq!(answers, 1 + MAX_RETRIES as usize);
    }

    #[test]
    fn build_answers_are_checked_by_chunk_text_and_must_be_permutations() {
        let prompt = Prompt::Build {
            english: "x".into(),
            chunks: vec!["좀".into(), "천천히".into(), "좀".into()],
        };
        let expected = Expected::Order(vec!["좀".into(), "좀".into(), "천천히".into()]);
        let order = |o: &[usize]| Answer::Order { order: o.to_vec() };
        // Either 좀 can go first.
        assert_eq!(check(&prompt, &expected, &order(&[0, 2, 1])), Ok(true));
        assert_eq!(check(&prompt, &expected, &order(&[2, 0, 1])), Ok(true));
        assert_eq!(check(&prompt, &expected, &order(&[1, 0, 2])), Ok(false));
        for bad in [&[0, 0, 1][..], &[0, 1], &[0, 1, 3]] {
            assert_eq!(
                check(&prompt, &expected, &order(bad)),
                Err(SessionError::InvalidAnswer)
            );
        }
        assert_eq!(
            check(&prompt, &expected, &Answer::Choice { index: 0 }),
            Err(SessionError::InvalidAnswer)
        );
    }

    #[test]
    fn reply_feedback_glosses_the_line_and_every_option() {
        let line = |korean: &str| crate::content::Line {
            korean: korean.into(),
            english: "x".into(),
        };
        let mut item = sentence("a/0", "밥 먹었어요?", "Did you eat?");
        item.replies = Some(crate::content::Replies {
            good: vec![line("네, 먹었어요.")],
            bad: vec![line("아니요."), line("괜찮아요."), line("몰라요.")],
        });
        let glossary = Glossary::new(
            [
                ("밥", "rice, meal"),
                ("먹었어요", "ate (먹다)"),
                ("네", "yes"),
                ("아니요", "no"),
                ("괜찮아요", "is fine (괜찮다)"),
                ("몰라요", "don't know (모르다)"),
                ("사과", "apple"),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        );
        let plan = Plan {
            cards: vec![Card::new("a/0", Skill::Response)],
            due: 0,
            new: 1,
        };
        let states = HashMap::new();
        let scheduler = Scheduler::with_defaults(0);
        let mut s = Session::new(plan, vec![item], states, scheduler, glossary, 7);
        s.current();
        let out = s.answer(&Answer::Choice { index: 0 }, 1_000, NOW).unwrap();
        let words: Vec<&str> = out.feedback.glosses.keys().map(String::as_str).collect();
        assert_eq!(
            words,
            [
                "괜찮아요.",
                "네,",
                "먹었어요.",
                "먹었어요?",
                "몰라요.",
                "밥",
                "아니요."
            ]
        );
        assert_eq!(out.feedback.glosses["먹었어요?"], "ate (먹다)");
    }

    #[test]
    fn answers_are_validated() {
        let mut s = session(1);
        assert_eq!(
            s.answer(&Answer::Choice { index: 0 }, 0, NOW).unwrap_err(),
            SessionError::NoCurrentExercise
        );
        s.current();
        assert_eq!(
            s.answer(&Answer::Choice { index: 9 }, 0, NOW).unwrap_err(),
            SessionError::InvalidAnswer
        );
        // The exercise is still waiting.
        assert!(s.answer(&Answer::Choice { index: 0 }, 0, NOW).is_ok());
    }
}
