//! A running review session: serves exercises, checks answers, updates memory.

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use super::exercise::{self, Exercise, ExerciseView};
use super::plan::{Card, Plan};
use crate::content::Item;
use crate::rng::Rng;
use crate::scheduler::{self, MemoryState, Rating};
use crate::scoring;

/// A missed card comes back after this many other cards.
const RETRY_GAP: usize = 3;
/// A card is retried at most this many times in one session.
const MAX_RETRIES: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Answer {
    Choice { index: usize },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Feedback {
    pub correct: bool,
    pub correct_index: usize,
    pub rating: Rating,
    pub korean: String,
    pub english: String,
    pub note: Option<String>,
    /// Consecutive correct answers in this session.
    pub streak: u32,
    /// The card will come back later in this session.
    pub retry: bool,
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
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SessionError {
    #[error("no exercise is waiting for an answer")]
    NoCurrentExercise,
    #[error("answer does not fit the exercise")]
    InvalidAnswer,
}

pub struct Session {
    queue: VecDeque<Card>,
    items: HashMap<String, Item>,
    states: HashMap<Card, MemoryState>,
    retries: HashMap<Card, u8>,
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
            states,
            retries: HashMap::new(),
            current: None,
            rng: Rng::new(seed),
            progress,
        }
    }

    pub fn progress(&self) -> Progress {
        self.progress
    }

    /// The exercise to play now; the same one until it is answered. `None` when finished.
    pub fn current(&mut self) -> Option<&ExerciseView> {
        if self.current.is_none() {
            let card = self.queue.pop_front()?;
            let item = &self.items[&card.item_id];
            let pool: Vec<&Item> = self
                .items
                .values()
                .filter(|o| o.id.split('/').next() == item.id.split('/').next())
                .collect();
            self.current = Some(exercise::build(&card, item, &pool, &mut self.rng));
        }
        self.current.as_ref().map(|e| &e.view)
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
        let Answer::Choice { index } = answer;
        let crate::learning::Prompt::Listening { options, .. } = &exercise.view.prompt;
        if *index >= options.len() {
            return Err(SessionError::InvalidAnswer);
        }
        let exercise = self.current.take().expect("checked above");
        let card = exercise.view.card;
        let item = &self.items[&card.item_id];
        let correct = *index == exercise.correct;
        let chunks = item.build_chunks().len();
        let rating = scoring::grade(card.skill, correct, elapsed_ms, chunks);
        let state = scheduler::review(self.states.get(&card), rating, now_ms);
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

        Ok(Outcome {
            feedback: Feedback {
                correct,
                correct_index: exercise.correct,
                rating,
                korean: item.korean.clone(),
                english: item.english.clone(),
                note: item.note.clone(),
                streak: p.streak,
                retry,
            },
            card,
            state,
            elapsed_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{ItemKind, Register, Skill};
    use crate::learning::Prompt;

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

    fn session(n: usize) -> Session {
        let items: Vec<Item> = (0..n)
            .map(|i| {
                let korean = format!("{}예요.", char::from_u32(0xAC00 + i as u32).unwrap());
                sentence(&format!("a/{i}"), &korean, &format!("E{i}"))
            })
            .collect();
        let plan = Plan {
            cards: items
                .iter()
                .map(|i| Card::new(&i.id, Skill::Listening))
                .collect(),
            due: 0,
            new: n,
        };
        Session::new(plan, items, HashMap::new(), 7)
    }

    fn correct_index(s: &mut Session) -> usize {
        let view = s.current().unwrap().clone();
        let Prompt::Listening { options, .. } = &view.prompt;
        let english = &s.items[&view.card.item_id].english;
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
            assert_eq!(out.state.reps, 1);
        }
        assert!(s.current().is_none());
        let p = s.progress();
        assert_eq!(
            (p.done, p.correct, p.best_streak, p.remaining),
            (3, 3, 3, 0)
        );
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
