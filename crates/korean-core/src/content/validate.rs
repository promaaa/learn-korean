use std::collections::{HashMap, HashSet};

use super::model::{Item, ItemKind, Pack};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{location}: {message}")]
pub struct ValidationError {
    pub location: String,
    pub message: String,
}

const PUNCTUATION: [char; 5] = ['?', '.', ',', '!', '~'];
const MAX_NOTE: usize = 120;

/// Hangul syllables separated by single spaces, with `? . , ! ~` punctuation. Must start with a
/// syllable; ellipses (`...` templates from the legacy deck) are rejected.
pub fn is_hangul_text(text: &str) -> bool {
    text.chars().next().is_some_and(is_syllable)
        && text.trim() == text
        && !text.contains("  ")
        && !text.contains("..")
        && text
            .chars()
            .all(|c| is_syllable(c) || c == ' ' || PUNCTUATION.contains(&c))
}

fn is_syllable(c: char) -> bool {
    ('\u{AC00}'..='\u{D7A3}').contains(&c)
}

fn is_kebab(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('-')
        && !s.ends_with('-')
        && !s.contains("--")
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Checks every rule of the content contract across all packs (ids and Korean are global).
pub fn validate_packs(packs: &[Pack]) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();
    let mut ids: HashMap<&str, &str> = HashMap::new();
    let mut pack_ids = HashSet::new();
    // The same Korean in two items would be the same card twice. A trailing full stop does not
    // make a different line; `?` does (괜찮아요 "I'm fine" vs 괜찮아요? "Are you okay?").
    let mut korean: HashMap<&str, &str> = HashMap::new();

    for pack in packs {
        let mut err =
            |location: String, message: String| errors.push(ValidationError { location, message });
        if !is_kebab(&pack.id) {
            err(pack.id.clone(), "pack id must be kebab-case".into());
        }
        if !pack_ids.insert(pack.id.as_str()) {
            err(pack.id.clone(), "duplicate pack id".into());
        }
        if pack.unlock_level == 0 {
            err(pack.id.clone(), "unlock_level must be >= 1".into());
        }
        if pack.items.is_empty() {
            err(pack.id.clone(), "pack has no items".into());
        }
        let mut english: HashSet<&str> = HashSet::new();
        for item in &pack.items {
            if let Some(other) = ids.insert(&item.id, &pack.id) {
                err(
                    item.id.clone(),
                    format!("duplicate item id (also in {other})"),
                );
            }
            if !english.insert(&item.english) {
                err(
                    item.id.clone(),
                    format!("english {:?} is used twice in the pack", item.english),
                );
            }
            if let Some(other) = korean.insert(item.korean.trim_end_matches('.'), &item.id) {
                err(item.id.clone(), format!("same Korean as {other}"));
            }
            for message in item_errors(&pack.id, item) {
                err(item.id.clone(), message);
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn item_errors(pack_id: &str, item: &Item) -> Vec<String> {
    let mut errors = Vec::new();
    match item.id.split_once('/') {
        Some((prefix, slug)) if prefix == pack_id && is_kebab(slug) => {}
        _ => errors.push(format!("id must be `{pack_id}/<kebab-slug>`")),
    }
    if !is_hangul_text(&item.korean) {
        errors.push(format!("korean {:?} must be Hangul text", item.korean));
    }
    if item.english.trim().is_empty() {
        errors.push("english is empty".into());
    }
    if !is_kebab(&item.context) {
        errors.push(format!("context {:?} must be kebab-case", item.context));
    }
    if item
        .note
        .as_ref()
        .is_some_and(|n| n.chars().count() > MAX_NOTE)
    {
        errors.push(format!("note longer than {MAX_NOTE} characters"));
    }
    for lexeme in &item.lexemes {
        if !is_hangul_text(lexeme) {
            errors.push(format!("lexeme {lexeme:?} must be Hangul text"));
        }
    }

    let needs_distractors = item.kind == ItemKind::Sentence || !item.distractors.is_empty();
    if needs_distractors {
        if item.distractors.len() != 3 {
            errors.push(format!(
                "expected exactly 3 distractors, found {}",
                item.distractors.len()
            ));
        }
        let unique: HashSet<&str> = item.distractors.iter().map(String::as_str).collect();
        if unique.len() != item.distractors.len() {
            errors.push("distractors must be distinct".into());
        }
        if unique.contains(item.english.as_str()) {
            errors.push("a distractor equals the correct meaning".into());
        }
    }

    if let Some(replies) = &item.replies {
        if item.kind != ItemKind::Sentence {
            errors.push("only sentences can have replies".into());
        }
        if !(1..=3).contains(&replies.good.len()) {
            errors.push("replies.good must have 1 to 3 lines".into());
        }
        if replies.bad.len() != 3 {
            errors.push("replies.bad must have exactly 3 lines".into());
        }
        let mut seen = HashSet::new();
        for line in replies.good.iter().chain(&replies.bad) {
            if !is_hangul_text(&line.korean) {
                errors.push(format!("reply {:?} must be Hangul text", line.korean));
            }
            if line.english.trim().is_empty() {
                errors.push(format!("reply {:?} has no english", line.korean));
            }
            if !seen.insert(line.korean.as_str()) {
                errors.push(format!("reply {:?} appears twice", line.korean));
            }
        }
    }

    if item
        .chunks
        .as_ref()
        .is_some_and(|c| c.join(" ") != item.korean)
    {
        errors.push("chunks joined with spaces must equal korean".into());
    }
    if let Some(image) = &item.image {
        let words = image.split_whitespace().count();
        if !(1..=3).contains(&words) {
            errors.push("image query must be 1 to 3 words".into());
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{Line, Register, Replies};

    /// Sentence item whose Korean is unique per id.
    fn item(id: &str, english: &str) -> Item {
        let syllable = char::from_u32(0xAC00 + id.bytes().map(u32::from).sum::<u32>() % 11172);
        Item {
            id: id.into(),
            kind: ItemKind::Sentence,
            korean: format!("주말에 {} 했어요?", syllable.unwrap()),
            english: english.into(),
            context: "weekend".into(),
            register: Register::Polite,
            note: None,
            lexemes: vec!["주말".into()],
            distractors: vec!["a".into(), "b".into(), "c".into()],
            replies: None,
            chunks: None,
            image: None,
        }
    }

    fn pack(items: Vec<Item>) -> Pack {
        Pack {
            id: "p".into(),
            version: 1,
            title: "P".into(),
            description: "".into(),
            unlock_level: 1,
            items,
        }
    }

    fn messages(packs: &[Pack]) -> Vec<String> {
        validate_packs(packs)
            .err()
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.to_string())
            .collect()
    }

    #[test]
    fn hangul_text_rules() {
        assert!(is_hangul_text("주말에 뭐 했어요?"));
        assert!(is_hangul_text("네, 좋아요!"));
        assert!(!is_hangul_text("피곤해요 (pi gon hae yo)"));
        assert!(!is_hangul_text("...주세요"), "templates are not text");
        assert!(!is_hangul_text("فقط"));
        assert!(!is_hangul_text("커피 2잔"));
        assert!(!is_hangul_text(" 앞 공백"));
        assert!(!is_hangul_text("두  칸"));
        assert!(!is_hangul_text("?"));
    }

    #[test]
    fn a_valid_pack_passes() {
        assert_eq!(validate_packs(&[pack(vec![item("p/a", "A")])]), Ok(()));
    }

    #[test]
    fn ids_must_be_prefixed_and_globally_unique() {
        let mut other = pack(vec![item("q/a", "A")]);
        other.id = "q".into();
        let dup = pack(vec![item("p/a", "A")]);
        let mut dup2 = dup.clone();
        dup2.id = "r".into();
        dup2.items[0].id = "p/a".into();
        let msgs = messages(&[dup, other, dup2]);
        assert!(msgs.iter().any(|m| m.contains("duplicate item id")));
        assert!(msgs.iter().any(|m| m.contains("id must be `r/")));
    }

    #[test]
    fn english_must_be_unique_within_a_pack() {
        let msgs = messages(&[pack(vec![item("p/a", "Same"), item("p/b", "Same")])]);
        assert_eq!(msgs.len(), 1, "{msgs:?}");
        assert!(msgs[0].contains("used twice"));
    }

    #[test]
    fn the_same_korean_cannot_appear_twice_across_packs() {
        let mut a = item("p/a", "A");
        a.korean = "감사합니다".into();
        let mut b = item("q/b", "B");
        b.korean = "감사합니다.".into();
        let mut question = item("q/c", "C");
        question.korean = "감사합니다?".into();
        let mut q = pack(vec![b, question]);
        q.id = "q".into();
        let msgs = messages(&[pack(vec![a]), q]);
        assert_eq!(msgs, ["q/b: same Korean as p/a"]);
    }

    #[test]
    fn sentence_distractors_are_three_distinct_wrong_meanings() {
        let mut two = item("p/a", "A");
        two.distractors.pop();
        let mut correct = item("p/b", "B");
        correct.distractors[0] = "B".into();
        let mut dup = item("p/c", "C");
        dup.distractors[1] = "a".into();
        let mut word = item("p/d", "D");
        word.kind = ItemKind::Word;
        word.korean = "자전거".into();
        word.distractors.clear();
        let msgs = messages(&[pack(vec![two, correct, dup, word])]);
        assert_eq!(msgs.len(), 3, "{msgs:?}");
    }

    #[test]
    fn replies_and_chunks_are_checked() {
        let line = |k: &str| Line {
            korean: k.into(),
            english: "x".into(),
        };
        let mut bad_replies = item("p/a", "A");
        bad_replies.replies = Some(Replies {
            good: vec![],
            bad: vec![line("네."), line("네.")],
        });
        let mut bad_chunks = item("p/b", "B");
        bad_chunks.chunks = Some(vec!["주말에".into(), "뭐했어요?".into()]);
        let msgs = messages(&[pack(vec![bad_replies, bad_chunks])]);
        assert!(msgs.iter().any(|m| m.contains("replies.good")));
        assert!(msgs.iter().any(|m| m.contains("replies.bad")));
        assert!(msgs.iter().any(|m| m.contains("appears twice")));
        assert!(msgs.iter().any(|m| m.contains("chunks joined")));
    }
}
