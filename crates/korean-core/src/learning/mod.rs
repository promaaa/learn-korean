//! Sessions: planning which cards to review, building exercises, applying answers.

mod exercise;
mod plan;
mod session;

use crate::content::Skill;

pub use exercise::{CHOICES, ExerciseView, Prompt};
pub(crate) use plan::word_cards;
pub use plan::{Card, Focus, Limits, Plan, is_known_word, plan_session};
pub use session::{Answer, Feedback, Outcome, Progress, Session, SessionError};

/// Skills that have a game. Planning only schedules these.
pub const PLAYABLE: &[Skill] = &Skill::ALL;
