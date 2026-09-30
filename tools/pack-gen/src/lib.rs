//! Content pack authoring: an LLM drafts items ([`draft`]), `check` validates the packs on disk
//! with a draft merged in ([`Library`]), and a human reviews the draft item by item
//! ([`draft::accept`]).

pub mod draft;
pub mod library;
pub mod prompt;

pub use library::Library;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct Error(pub String);

impl From<String> for Error {
    fn from(message: String) -> Self {
        Error(message)
    }
}

impl From<&str> for Error {
    fn from(message: &str) -> Self {
        Error(message.into())
    }
}
