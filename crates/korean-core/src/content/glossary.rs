//! English glosses of Korean word forms, shown when hovering a word after answering.
//!
//! Each pack has its own glossary (`content/<pack>/glossary.json`) holding the words it owns: the
//! words whose first use, in bundle order, is one of its lines (see [`word_owners`]).

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::model::{Item, Pack};
use super::validate::{PUNCTUATION, ValidationError};

/// Longest gloss, in characters: it is a tooltip, not a dictionary entry.
const MAX_GLOSS: usize = 40;

/// English of word forms (space-separated tokens without punctuation), sorted by word.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Glossary(BTreeMap<String, String>);

impl Glossary {
    pub fn new(entries: BTreeMap<String, String>) -> Self {
        Glossary(entries)
    }

    pub fn get(&self, word: &str) -> Option<&str> {
        self.0.get(word).map(String::as_str)
    }

    /// Sets the gloss of `word`, returning the previous one.
    pub fn insert(&mut self, word: String, gloss: String) -> Option<String> {
        self.0.insert(word, gloss)
    }

    pub fn remove(&mut self, word: &str) -> Option<String> {
        self.0.remove(word)
    }

    /// `(word, gloss)` pairs in word order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(w, g)| (w.as_str(), g.as_str()))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
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

impl FromIterator<(String, String)> for Glossary {
    fn from_iter<I: IntoIterator<Item = (String, String)>>(entries: I) -> Self {
        Glossary(entries.into_iter().collect())
    }
}

impl IntoIterator for Glossary {
    type Item = (String, String);
    type IntoIter = std::collections::btree_map::IntoIter<String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
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

/// The glossary keys of every word of an item's lines (its Korean, then its replies), in order.
pub fn item_words(item: &Item) -> impl Iterator<Item = &str> {
    lines(item).flat_map(|line| line.split(' ')).map(word)
}

/// The owner of every word the packs use: the index in `packs` of the first pack whose lines use
/// it, with the first item using it. The word's gloss belongs in that pack's glossary.
pub fn word_owners(packs: &[Pack]) -> HashMap<&str, (usize, &Item)> {
    let mut owners = HashMap::new();
    for (index, pack) in packs.iter().enumerate() {
        for item in &pack.items {
            for word in item_words(item) {
                owners.entry(word).or_insert((index, item));
            }
        }
    }
    owners
}

/// Location of a glossary entry in a [`ValidationError`]: `restaurants glossary "맵다"`.
pub fn glossary_location(pack_id: &str, word: &str) -> String {
    format!("{pack_id} glossary {word:?}")
}

/// Every word used by a line is glossed in the glossary of its owner pack ([`word_owners`]), and
/// every gloss is used, in its owner pack, and short. `glossaries[i]` is the glossary of
/// `packs[i]`. A missing gloss is reported once, at the item using the word first.
pub fn validate_glossary(
    packs: &[Pack],
    glossaries: &[Glossary],
) -> Result<(), Vec<ValidationError>> {
    assert_eq!(packs.len(), glossaries.len(), "one glossary per pack");
    let mut errors = Vec::new();
    let mut seen = HashSet::new();
    for (pack, glossary) in packs.iter().zip(glossaries) {
        for item in &pack.items {
            for word in item_words(item) {
                if seen.insert(word) && glossary.get(word).is_none() {
                    errors.push(ValidationError {
                        location: item.id.clone(),
                        message: format!(
                            "word {word:?} has no gloss in content/{}/glossary.json",
                            pack.id
                        ),
                    });
                }
            }
        }
    }
    let owners = word_owners(packs);
    for (index, (pack, glossary)) in packs.iter().zip(glossaries).enumerate() {
        for (word, gloss) in glossary.iter() {
            let mut err = |message: String| {
                errors.push(ValidationError {
                    location: glossary_location(&pack.id, word),
                    message,
                })
            };
            match owners.get(word) {
                None => err("no line uses this word".into()),
                Some(&(owner, item)) if owner != index => err(format!(
                    "first used by {}: belongs in content/{}/glossary.json",
                    item.id, packs[owner].id
                )),
                Some(_) => {}
            }
            if gloss.trim() != gloss || gloss.is_empty() {
                err("gloss must be non-empty and trimmed".into());
            }
            if gloss.chars().count() > MAX_GLOSS {
                err(format!("gloss longer than {MAX_GLOSS} characters"));
            }
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
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn sentence(id: &str, korean: &str, replies: &[&str]) -> Item {
        let line = |korean: &&str| Line {
            korean: korean.to_string(),
            english: "x".into(),
        };
        Item {
            id: id.into(),
            kind: ItemKind::Sentence,
            korean: korean.into(),
            english: "x".into(),
            context: "t".into(),
            register: Register::Polite,
            note: None,
            lexemes: vec![],
            distractors: vec![],
            replies: (!replies.is_empty()).then(|| Replies {
                good: replies.iter().map(line).collect(),
                bad: vec![],
            }),
            chunks: None,
            image: None,
        }
    }

    fn pack(id: &str, items: Vec<Item>) -> Pack {
        Pack {
            id: id.into(),
            version: 1,
            title: "P".into(),
            description: "d".into(),
            unlock_level: 1,
            items,
        }
    }

    /// `a` asks about the weekend (with a reply); `b` reuses 친구를 and 뭐 and adds new words.
    fn two_packs() -> [Pack; 2] {
        [
            pack(
                "a",
                vec![sentence("a/1", "주말에 뭐 했어요?", &["친구를 만났어요."])],
            ),
            pack(
                "b",
                vec![
                    sentence("b/1", "친구를 만나요.", &[]),
                    sentence("b/2", "뭐 먹어요?", &[]),
                ],
            ),
        ]
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
    fn a_word_belongs_to_the_first_pack_whose_lines_use_it() {
        let packs = two_packs();
        let owners = word_owners(&packs);
        let owner = |word: &str| {
            owners
                .get(word)
                .map(|&(pack, item)| (packs[pack].id.as_str(), item.id.as_str()))
        };
        assert_eq!(owner("친구를"), Some(("a", "a/1")), "replies are lines too");
        assert_eq!(owner("뭐"), Some(("a", "a/1")));
        assert_eq!(owner("만나요"), Some(("b", "b/1")));
        assert_eq!(owner("먹어요"), Some(("b", "b/2")), "punctuation stripped");
        assert_eq!(owner("주말"), None);
    }

    #[test]
    fn every_word_needs_a_gloss_and_every_gloss_a_word() {
        let packs = [pack(
            "p",
            vec![sentence(
                "p/weekend",
                "주말에 뭐 했어요?",
                &["친구를 만났어요."],
            )],
        )];
        let forty = "x".repeat(40);
        let complete = [
            ("주말에", forty.as_str()),
            ("뭐", "what"),
            ("했어요", "did (하다)"),
            ("친구를", "friend (object)"),
            ("만났어요", "met (만나다)"),
        ];
        assert_eq!(validate_glossary(&packs, &[glossary(&complete)]), Ok(()));

        let forty_one = "x".repeat(41);
        let mut entries = complete.to_vec();
        entries.retain(|(k, _)| *k != "만났어요");
        entries.push(("사과", "apple"));
        entries.push(("뭐", " what"));
        entries.push(("했어요", forty_one.as_str()));
        let errors = validate_glossary(&packs, &[glossary(&entries)]).unwrap_err();
        let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
        assert_eq!(
            messages,
            [
                "p/weekend: word \"만났어요\" has no gloss in content/p/glossary.json",
                "p glossary \"뭐\": gloss must be non-empty and trimmed",
                "p glossary \"사과\": no line uses this word",
                "p glossary \"했어요\": gloss longer than 40 characters",
            ]
        );
    }

    #[test]
    fn each_gloss_lives_in_the_glossary_of_its_owner_pack() {
        let packs = two_packs();
        let a = [
            ("주말에", "on the weekend"),
            ("뭐", "what"),
            ("했어요", "did (하다)"),
            ("친구를", "friend (object)"),
            ("만났어요", "met (만나다)"),
        ];
        let b = [("만나요", "meet (만나다)"), ("먹어요", "eat (먹다)")];
        assert_eq!(
            validate_glossary(&packs, &[glossary(&a), glossary(&b)]),
            Ok(())
        );

        // 만나요 glossed in the earlier pack, 친구를 twice, 먹어요 nowhere.
        let mut a = a.to_vec();
        a.push(("만나요", "meet (만나다)"));
        let b = [("친구를", "friend (object)")];
        let errors = validate_glossary(&packs, &[glossary(&a), glossary(&b)]).unwrap_err();
        let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
        assert_eq!(
            messages,
            [
                "b/1: word \"만나요\" has no gloss in content/b/glossary.json",
                "b/2: word \"먹어요\" has no gloss in content/b/glossary.json",
                "a glossary \"만나요\": first used by b/1: belongs in content/b/glossary.json",
                "b glossary \"친구를\": first used by a/1: belongs in content/a/glossary.json",
            ]
        );
    }
}
