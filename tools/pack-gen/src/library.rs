//! The packs of the content directory, in bundle order, with their glossaries.

use std::path::{Path, PathBuf};

use korean_core::content::{
    Glossary, Pack, ValidationError, validate_glossary, validate_packs, validate_word_cards,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Library {
    pub packs: Vec<Pack>,
    /// `glossaries[i]` belongs to `packs[i]`.
    pub glossaries: Vec<Glossary>,
}

impl Library {
    /// Reads `<dir>/<id>/pack.json` and `glossary.json` for every id, in order, from disk (not the
    /// copies embedded at build time). A missing glossary is empty.
    pub fn load(dir: &Path, ids: &[&str]) -> Result<Library, Error> {
        let mut library = Library {
            packs: Vec::new(),
            glossaries: Vec::new(),
        };
        for id in ids {
            let path = pack_path(dir, id);
            let pack = read_json_opt(&path)?
                .ok_or_else(|| format!("{}: bundled pack file not found", shown(&path)))?;
            library.push(pack, read_glossary(dir, id)?);
        }
        Ok(library)
    }

    pub fn index(&self, id: &str) -> Option<usize> {
        self.packs.iter().position(|p| p.id == id)
    }

    /// Appends a pack after the others, returning its index.
    pub fn push(&mut self, pack: Pack, glossary: Glossary) -> usize {
        self.packs.push(pack);
        self.glossaries.push(glossary);
        self.packs.len() - 1
    }

    /// The pack glossing `word` (the first, if several wrongly do) and its gloss.
    pub fn gloss(&self, word: &str) -> Option<(usize, &str)> {
        self.glossaries
            .iter()
            .enumerate()
            .find_map(|(i, g)| g.get(word).map(|gloss| (i, gloss)))
    }

    /// Every content rule: packs, per-pack glossaries, word cards.
    pub fn check(&self) -> Vec<ValidationError> {
        [
            validate_packs(&self.packs),
            validate_glossary(&self.packs, &self.glossaries),
            validate_word_cards(&self.packs),
        ]
        .into_iter()
        .filter_map(Result::err)
        .flatten()
        .collect()
    }
}

pub fn pack_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(id).join("pack.json")
}

pub fn glossary_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(id).join("glossary.json")
}

pub fn draft_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(id).join("draft.json")
}

pub fn read_glossary(dir: &Path, id: &str) -> Result<Glossary, Error> {
    Ok(read_json_opt(&glossary_path(dir, id))?.unwrap_or_default())
}

/// `path` relative to the working directory when it is inside it, for messages.
pub fn shown(path: &Path) -> String {
    let relative = std::env::current_dir()
        .ok()
        .and_then(|cwd| path.strip_prefix(cwd).ok().map(Path::to_path_buf));
    relative.as_deref().unwrap_or(path).display().to_string()
}

/// Parses a JSON file; `None` when it does not exist. Errors name the file, line and column.
pub fn read_json_opt<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, Error> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{}: {e}", shown(path)).into()),
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|e| format!("{}: {e}", shown(path)).into())
}

/// Pretty JSON with a trailing newline, the format of every content file.
pub fn to_json<T: Serialize>(value: &T) -> String {
    let mut json = serde_json::to_string_pretty(value).expect("serializable");
    json.push('\n');
    json
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), Error> {
    let write = || -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, to_json(value))
    };
    write().map_err(|e| format!("{}: {e}", shown(path)).into())
}
