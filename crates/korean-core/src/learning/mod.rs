//! Sessions: planning which cards to review, building exercises, applying answers.

mod exercise;
mod plan;
mod session;

use crate::content::Skill;

pub use exercise::{CHOICES, ExerciseView, Prompt};
pub use plan::{Card, Limits, Plan, plan_session};
pub use session::{Answer, Feedback, Outcome, Progress, Session, SessionError};

/// Skills that have a game. Planning only schedules these.
pub const PLAYABLE: &[Skill] = &[Skill::Listening];
