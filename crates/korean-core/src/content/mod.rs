//! Normalized content model (see `docs/architecture.md`, "Data").
//!
//! Three levels: [`RawEntry`] (imported source rows, immutable), [`Lexeme`] (normalized dictionary
//! entries) and [`Item`] (what the games teach), grouped in [`Pack`]s.

mod bundled;
mod model;
mod validate;

pub use bundled::bundled_packs;
pub use model::{
    Item, ItemKind, Lexeme, Line, Pack, PartOfSpeech, RawEntry, Register, Replies, Skill,
};
pub use validate::{ValidationError, is_hangul_text, validate_packs};
