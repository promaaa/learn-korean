//! English glosses of Korean word forms, shown when hovering a word after answering.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::Deserialize;

use super::model::{Item, Pack};
use super::validate::{PUNCTUATION, ValidationError};

/// Longest gloss, in characters: it is a tooltip, not a dictionary entry.
const MAX_GLOSS: usize = 40;

/// English of every word form (space-separated token without punctuation) used by the packs.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(transparent)]
pub struct Glossary(HashMap<String, String>);

impl Glossary {
    pub fn new(entries: HashMap<String, String>) -> Self {
        Glossary(entries)
    }

    /// Glosses of the words of `lines`, keyed by the word as displayed (`괜찮아요?`).
    pub fn glosses<'a>(
        &self,
        lines: impl IntoIterator<Item = &'a str>,
    ) -> BTreeMap<String, String> {
        lines
            .into_iter()
            .flat_map(|line| line.split(' '))
            .filter_map(|token| {
                let gloss = self.0.get(word(token))?;
                Some((token.to_string(), gloss.clone()))
            })
            .collect()
    }
}

/// The glossary key of a displayed token: `괜찮아요?` → `괜찮아요`.
fn word(token: &str) -> &str {
    token.trim_matches(PUNCTUATION)
}

/// Every Korean line of an item: the item itself and its replies.
fn lines(item: &Item) -> impl Iterator<Item = &str> {
    let replies = item
        .replies
        .iter()
        .flat_map(|r| r.good.iter().chain(&r.bad))
        .map(|line| line.korean.as_str());
    std::iter::once(item.korean.as_str()).chain(replies)
}

/// Every word of every line has a gloss, and every gloss is used by some line.
pub fn validate_glossary(packs: &[Pack], glossary: &Glossary) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();
    let mut used = HashSet::new();
    for item in packs.iter().flat_map(|p| &p.items) {
        for token in lines(item).flat_map(|line| line.split(' ')) {
            let word = word(token);
            if !glossary.0.contains_key(word) {
                errors.push(ValidationError {
                    location: item.id.clone(),
                    message: format!("word {word:?} has no gloss in content/glossary.json"),
                });
            }
            used.insert(word);
        }
    }
    let mut entries: Vec<(&String, &String)> = glossary.0.iter().collect();
    entries.sort_unstable();
    for (word, gloss) in entries {
        let mut err = |message: String| {
            errors.push(ValidationError {
                location: format!("glossary {word:?}"),
                message,
            })
        };
        if !used.contains(word.as_str()) {
            err("no line uses this word".into());
        }
        if gloss.trim() != gloss || gloss.is_empty() {
            err("gloss must be non-empty and trimmed".into());
        }
        if gloss.chars().count() > MAX_GLOSS {
            err(format!("gloss longer than {MAX_GLOSS} characters"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{ItemKind, Line, Register, Replies};

    fn glossary(entries: &[(&str, &str)]) -> Glossary {
        Glossary::new(
            entries
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
    }

    fn pack() -> Pack {
        let line = |korean: &str| Line {
            korean: korean.into(),
            english: "x".into(),
        };
        Pack {
            id: "p".into(),
            version: 1,
            title: "P".into(),
            description: "d".into(),
            unlock_level: 1,
            items: vec![Item {
                id: "p/weekend".into(),
                kind: ItemKind::Sentence,
                korean: "주말에 뭐 했어요?".into(),
                english: "What did you do on the weekend?".into(),
                context: "t".into(),
                register: Register::Polite,
                note: None,
                lexemes: vec![],
                distractors: vec![],
                replies: Some(Replies {
                    good: vec![line("친구를 만났어요.")],
                    bad: vec![],
                }),
                chunks: None,
                image: None,
            }],
        }
    }

    #[test]
    fn glosses_are_keyed_by_the_displayed_token() {
        let g = glossary(&[("괜찮아요", "it's fine"), ("네", "yes")]);
        let glosses = g.glosses(["네, 괜찮아요?", "모르는"]);
        assert_eq!(
            glosses.into_iter().collect::<Vec<_>>(),
            [
                ("괜찮아요?".to_string(), "it's fine".to_string()),
                ("네,".to_string(), "yes".to_string()),
            ]
        );
    }

    #[test]
    fn every_word_of_items_and_replies_needs_a_gloss_and_every_gloss_a_word() {
        let complete = [
            ("주말에", "on the weekend"),
            ("뭐", "what"),
            ("했어요", "did (하다)"),
            ("친구를", "friend (object)"),
            ("만났어요", "met (만나다)"),
        ];
        assert_eq!(validate_glossary(&[pack()], &glossary(&complete)), Ok(()));

        let mut entries = complete.to_vec();
        entries.retain(|(k, _)| *k != "만났어요");
        entries.push(("사과", "apple"));
        entries.push(("뭐", " what"));
        let errors = validate_glossary(&[pack()], &glossary(&entries)).unwrap_err();
        let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
        assert_eq!(
            messages,
            [
                "p/weekend: word \"만났어요\" has no gloss in content/glossary.json",
                "glossary \"뭐\": gloss must be non-empty and trimmed",
                "glossary \"사과\": no line uses this word",
            ]
        );
    }
}
