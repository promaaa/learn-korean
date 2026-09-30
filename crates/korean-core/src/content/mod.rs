//! Normalized content model (see `docs/architecture.md`, "Data").
//!
//! Three levels: [`RawEntry`] (imported source rows, immutable), [`Lexeme`] (normalized dictionary
//! entries) and [`Item`] (what the games teach), grouped in [`Pack`]s. Each pack's [`Glossary`]
//! gives the English of the word forms it is the first to use.

mod bundled;
mod glossary;
mod model;
mod validate;

pub use bundled::{bundled_glossary, bundled_pack_ids, bundled_packs};
pub use glossary::{Glossary, glossary_location, item_words, validate_glossary, word_owners};
pub use model::{
    Item, ItemKind, Lexeme, Line, Pack, PartOfSpeech, RawEntry, Register, Replies, Skill,
};
pub(crate) use validate::is_kebab;
pub use validate::{
    GRAMMAR, ValidationError, carded_lemmas, is_hangul_text, validate_packs, validate_word_cards,
};
