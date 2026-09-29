//! Normalized content model (see `docs/architecture.md`, "Data").
//!
//! Three levels: [`RawEntry`] (imported source rows, immutable), [`Lexeme`] (normalized dictionary
//! entries) and [`Item`] (what the games teach), grouped in [`Pack`]s. The [`Glossary`] gives the
//! English of every word form the packs use.

mod bundled;
mod glossary;
mod model;
mod validate;

pub use bundled::{bundled_glossary, bundled_packs};
pub use glossary::{Glossary, validate_glossary};
pub use model::{
    Item, ItemKind, Lexeme, Line, Pack, PartOfSpeech, RawEntry, Register, Replies, Skill,
};
pub use validate::{ValidationError, is_hangul_text, validate_packs};
