//! `content/<pack>/draft.json`: items waiting for review, with the glosses of their new words.

use std::collections::{BTreeSet, HashSet};

use korean_core::content::{
    Glossary, Item, Pack, ValidationError, glossary_location, item_words, word_owners,
};
use serde::{Deserialize, Serialize};

use crate::Library;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    /// Metadata of a pack that has no `pack.json` yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack: Option<PackMeta>,
    #[serde(default)]
    pub items: Vec<Item>,
    #[serde(default)]
    pub glossary: Glossary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackMeta {
    pub title: String,
    pub description: String,
    pub unlock_level: u32,
}

impl PackMeta {
    /// The new pack, still without items.
    pub fn pack(&self, id: &str) -> Pack {
        Pack {
            id: id.into(),
            version: 1,
            title: self.title.clone(),
            description: self.description.clone(),
            unlock_level: self.unlock_level,
            items: Vec::new(),
        }
    }
}

/// What the LLM is asked to answer.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Generated {
    pub items: Vec<Item>,
    #[serde(default)]
    pub glossary: Glossary,
}

/// Parses the LLM's answer: the JSON object alone, or in a fenced code block amid prose. The error
/// is the parse error of the most likely candidate (first code block, else the outer braces).
pub fn parse_generated(output: &str) -> Result<Generated, String> {
    let mut first_error = None;
    for candidate in json_candidates(output) {
        match serde_json::from_str(candidate) {
            Ok(generated) => return Ok(generated),
            Err(e) => {
                first_error.get_or_insert(e.to_string());
            }
        }
    }
    Err(first_error.unwrap_or_else(|| "no JSON object in the answer".into()))
}

/// Contents of the fenced code blocks, then the text from the first `{` to the last `}`.
fn json_candidates(output: &str) -> Vec<&str> {
    let mut candidates: Vec<&str> = output
        .split("```")
        .skip(1)
        .step_by(2)
        .map(|block| match block.split_once('\n') {
            // Drop the info string (```json).
            Some((info, body)) if !info.trim_start().starts_with('{') => body.trim(),
            _ => block.trim(),
        })
        .collect();
    if let (Some(start), Some(end)) = (output.find('{'), output.rfind('}'))
        && start < end
    {
        candidates.push(&output[start..=end]);
    }
    candidates
}

impl Draft {
    /// Appends generated items. A generated gloss of a word the draft already glosses is dropped:
    /// the draft may have been edited.
    pub fn extend(&mut self, generated: Generated) {
        self.items.extend(generated.items);
        for (word, gloss) in generated.glossary {
            if self.glossary.get(&word).is_none() {
                self.glossary.insert(word, gloss);
            }
        }
    }

    /// Removes item `index` without adding it to the pack.
    pub fn discard(&mut self, index: usize) -> Item {
        let item = self.items.remove(index);
        self.prune_glossary();
        item
    }

    /// Drops the glosses no remaining item uses.
    fn prune_glossary(&mut self) {
        let used: HashSet<&str> = self.items.iter().flat_map(item_words).collect();
        self.glossary = std::mem::take(&mut self.glossary)
            .into_iter()
            .filter(|(word, _)| used.contains(word.as_str()))
            .collect();
    }
}

/// Location suffix of errors caused by the draft.
const FROM_DRAFT: &str = " (draft)";

/// Every content error once `draft` is merged into pack `target`: its items appended, its glosses
/// added to the pack's glossary. A draft gloss of a word some pack already glosses is reported
/// instead of merged (unless it is the target's identical gloss). Errors located at a draft item
/// or gloss are marked `(draft)`.
pub fn check_draft(mut library: Library, target: usize, draft: &Draft) -> Vec<ValidationError> {
    let pack_id = library.packs[target].id.clone();
    library.packs[target]
        .items
        .extend(draft.items.iter().cloned());
    let mut conflicts = Vec::new();
    for (word, gloss) in draft.glossary.iter() {
        match library.gloss(word) {
            Some((owner, existing)) if owner == target && existing == gloss => {}
            Some((owner, existing)) => conflicts.push(ValidationError {
                location: glossary_location(&pack_id, word) + FROM_DRAFT,
                message: format!(
                    "already glossed {existing:?} in content/{}/glossary.json: remove it from the draft",
                    library.packs[owner].id
                ),
            }),
            None => {
                library.glossaries[target].insert(word.into(), gloss.into());
            }
        }
    }
    let draft_ids: HashSet<&str> = draft.items.iter().map(|i| i.id.as_str()).collect();
    let draft_glosses: HashSet<String> = draft
        .glossary
        .iter()
        .map(|(word, _)| glossary_location(&pack_id, word))
        .collect();
    let mut errors = library.check();
    for error in &mut errors {
        if draft_ids.contains(error.location.as_str()) || draft_glosses.contains(&error.location) {
            error.location.push_str(FROM_DRAFT);
        }
    }
    errors.extend(conflicts);
    errors
}

/// Moves draft item `index` to the end of pack `target` and gives the pack's glossary the gloss of
/// each word the item makes it own and it lacks: moved from the later pack that owned the word so
/// far, else taken from the draft. Returns the packs whose glossary changed.
pub fn accept(
    library: &mut Library,
    target: usize,
    draft: &mut Draft,
    index: usize,
) -> BTreeSet<usize> {
    let item = draft.items.remove(index);
    let words: BTreeSet<String> = item_words(&item).map(str::to_string).collect();
    library.packs[target].items.push(item);
    let owners = word_owners(&library.packs);
    let needed: Vec<String> = words
        .into_iter()
        .filter(|word| {
            owners
                .get(word.as_str())
                .is_some_and(|&(owner, _)| owner == target)
        })
        .filter(|word| library.glossaries[target].get(word).is_none())
        .collect();
    let mut changed = BTreeSet::new();
    for word in needed {
        let moved = library.gloss(&word).map(|(from, _)| from);
        let gloss = match moved {
            Some(from) => {
                changed.insert(from);
                library.glossaries[from].remove(&word)
            }
            None => draft.glossary.get(&word).map(str::to_string),
        };
        if let Some(gloss) = gloss {
            library.glossaries[target].insert(word, gloss);
            changed.insert(target);
        }
    }
    draft.prune_glossary();
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use korean_core::content::{ItemKind, Register};

    fn item(id: &str, kind: ItemKind, korean: &str, lexemes: &[&str]) -> Item {
        Item {
            id: id.into(),
            kind,
            korean: korean.into(),
            english: format!("english of {id}"),
            context: "test".into(),
            register: Register::Polite,
            note: None,
            lexemes: lexemes.iter().map(|l| l.to_string()).collect(),
            distractors: match kind {
                ItemKind::Sentence => vec!["x".into(), "y".into(), "z".into()],
                ItemKind::Word => vec![],
            },
            replies: None,
            chunks: None,
            image: None,
        }
    }

    fn glossary(entries: &[(&str, &str)]) -> Glossary {
        entries
            .iter()
            .map(|(w, g)| (w.to_string(), g.to_string()))
            .collect()
    }

    fn pack(id: &str, items: Vec<Item>) -> Pack {
        Pack {
            id: id.into(),
            version: 1,
            title: id.into(),
            description: String::new(),
            unlock_level: 1,
            items,
        }
    }

    /// `a` teaches 물 and 주세요; `b` reuses 주세요 and adds 커피.
    fn library() -> Library {
        Library {
            packs: vec![
                pack(
                    "a",
                    vec![
                        item("a/water", ItemKind::Word, "물", &["물"]),
                        item("a/give", ItemKind::Word, "주다", &["주다"]),
                        item(
                            "a/water-please",
                            ItemKind::Sentence,
                            "물 주세요.",
                            &["물", "주다"],
                        ),
                    ],
                ),
                pack(
                    "b",
                    vec![
                        item("b/coffee", ItemKind::Word, "커피", &["커피"]),
                        item(
                            "b/coffee-please",
                            ItemKind::Sentence,
                            "커피 주세요.",
                            &["커피", "주다"],
                        ),
                    ],
                ),
            ],
            glossaries: vec![
                glossary(&[
                    ("물", "water"),
                    ("주다", "to give"),
                    ("주세요", "please give (주다)"),
                ]),
                glossary(&[("커피", "coffee")]),
            ],
        }
    }

    #[test]
    fn the_answer_is_found_alone_fenced_or_amid_prose() {
        let json = r#"{"items": [], "glossary": {"물": "water"}}"#;
        let expected = Generated {
            items: vec![],
            glossary: glossary(&[("물", "water")]),
        };
        for answer in [
            json.to_string(),
            format!("Here is the draft:\n```json\n{json}\n```\nEnjoy {{no}} braces."),
            format!("```\n{json}\n```"),
            format!("Sure! {json} Hope it helps."),
        ] {
            assert_eq!(parse_generated(&answer), Ok(expected.clone()), "{answer}");
        }
    }

    #[test]
    fn an_unreadable_answer_reports_the_parse_error_of_the_code_block() {
        let answer = "```json\n{\"items\": [{\"id\": \"a/x\", \"colour\": 1}]}\n```";
        let error = parse_generated(answer).unwrap_err();
        assert!(error.contains("colour"), "{error}");
        assert_eq!(
            parse_generated("I cannot help with that."),
            Err("no JSON object in the answer".into())
        );
    }

    #[test]
    fn a_valid_draft_merges_cleanly_and_its_errors_are_marked() {
        let mut draft = Draft {
            pack: None,
            items: vec![
                item("b/tea", ItemKind::Word, "차", &["차"]),
                item(
                    "b/tea-please",
                    ItemKind::Sentence,
                    "차 주세요.",
                    &["차", "주다"],
                ),
            ],
            glossary: glossary(&[("차", "tea"), ("커피", "coffee")]),
        };
        assert_eq!(
            check_draft(library(), 1, &draft),
            vec![],
            "identical glosses merge"
        );

        draft.items.push(item(
            "b/milk-please",
            ItemKind::Sentence,
            "우유 주세요.",
            &["우유"],
        ));
        draft.glossary.insert("물".into(), "water".into());
        draft.glossary.insert("빵".into(), "bread".into());
        let messages: Vec<String> = check_draft(library(), 1, &draft)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            messages,
            [
                "b/milk-please (draft): word \"우유\" has no gloss in content/b/glossary.json",
                "b glossary \"빵\" (draft): no line uses this word",
                "b/milk-please (draft): lexeme \"우유\" has no word card (a word item listing it)",
                "b glossary \"물\" (draft): already glossed \"water\" in content/a/glossary.json: remove it from the draft",
            ]
        );
    }

    #[test]
    fn a_draft_gloss_differing_from_the_pack_is_a_conflict() {
        let draft = Draft {
            items: vec![],
            glossary: glossary(&[("커피", "coffee (drink)")]),
            ..Draft::default()
        };
        let errors = check_draft(library(), 1, &draft);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].message.contains("already glossed \"coffee\""));
    }

    #[test]
    fn accepting_takes_the_new_glosses_from_the_draft_and_prunes_it() {
        let mut library = library();
        let mut draft = Draft {
            pack: None,
            items: vec![
                item("b/tea-please", ItemKind::Sentence, "차 주세요.", &["차"]),
                item("b/hot-tea", ItemKind::Sentence, "뜨거운 차.", &["차"]),
            ],
            glossary: glossary(&[("차", "tea"), ("뜨거운", "hot (뜨겁다)")]),
        };
        let changed = accept(&mut library, 1, &mut draft, 0);
        assert_eq!(changed, BTreeSet::from([1]));
        assert_eq!(library.packs[1].items.last().unwrap().id, "b/tea-please");
        assert_eq!(library.glossaries[1].get("차"), Some("tea"));
        assert_eq!(library.glossaries[1].get("주세요"), None, "owned by a");
        assert_eq!(draft.items.len(), 1);
        assert_eq!(
            draft.glossary,
            glossary(&[("차", "tea"), ("뜨거운", "hot (뜨겁다)")])
        );

        draft.discard(0);
        assert_eq!(draft, Draft::default());
    }

    #[test]
    fn accepting_into_an_earlier_pack_moves_the_glosses_it_now_owns() {
        let mut library = library();
        let mut draft = Draft {
            pack: None,
            items: vec![item("a/coffee-water", ItemKind::Sentence, "커피 물.", &[])],
            glossary: glossary(&[("커피", "coffee (draft)")]),
        };
        let changed = accept(&mut library, 0, &mut draft, 0);
        assert_eq!(changed, BTreeSet::from([0, 1]));
        assert_eq!(
            library.glossaries[0].get("커피"),
            Some("coffee"),
            "curated gloss wins"
        );
        assert_eq!(library.glossaries[1].get("커피"), None);
        assert_eq!(library.check(), vec![], "the move leaves valid content");
    }
}
