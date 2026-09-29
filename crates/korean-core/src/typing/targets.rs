//! What to type in the Typing Gym.

use std::collections::HashSet;

use serde::Serialize;

use super::jamo::keystrokes_for_text;
use crate::content::Item;
use crate::rng::Rng;

/// Longer lines do not fit the drill screen.
const MAX_CHARS: usize = 28;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub item_id: String,
    pub korean: String,
    pub english: String,
}

/// Up to `count` lines to type. Items the learner has already met come first (the meaning is
/// known, so typing reinforces it); the chosen lines are ordered short to long as a warm-up.
pub fn pick_targets(
    items: &[Item],
    seen: &HashSet<String>,
    count: usize,
    seed: u64,
) -> Vec<Target> {
    let mut rng = Rng::new(seed);
    let typeable = |item: &&Item| {
        item.korean.chars().count() <= MAX_CHARS && keystrokes_for_text(&item.korean).is_ok()
    };
    let (mut known, mut fresh): (Vec<&Item>, Vec<&Item>) = items
        .iter()
        .filter(typeable)
        .partition(|item| seen.contains(&item.id));
    rng.shuffle(&mut known);
    rng.shuffle(&mut fresh);
    let mut chosen: Vec<&Item> = known.into_iter().chain(fresh).take(count).collect();
    chosen.sort_by_key(|item| item.korean.chars().count());
    chosen
        .into_iter()
        .map(|item| Target {
            item_id: item.id.clone(),
            korean: item.korean.clone(),
            english: item.english.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{ItemKind, Register};

    fn item(id: &str, korean: &str) -> Item {
        Item {
            id: id.into(),
            kind: ItemKind::Sentence,
            korean: korean.into(),
            english: id.into(),
            context: "t".into(),
            register: Register::Polite,
            note: None,
            lexemes: vec![],
            distractors: vec![],
            replies: None,
            chunks: None,
            image: None,
        }
    }

    #[test]
    fn known_items_first_then_short_to_long() {
        let items = vec![
            item("a/long", "주말에 친구를 만나서 영화를 봤어요."),
            item("a/new", "새로운 거예요."),
            item("a/short", "네."),
            item("a/mid", "괜찮아요?"),
            item("a/huge", &"가".repeat(40)),
        ];
        let seen: HashSet<String> = ["a/long", "a/short", "a/mid"].map(String::from).into();
        for seed in 0..10 {
            let picked = pick_targets(&items, &seen, 3, seed);
            let ids: Vec<&str> = picked.iter().map(|t| t.item_id.as_str()).collect();
            assert_eq!(ids, ["a/short", "a/mid", "a/long"]);
        }
        let all = pick_targets(&items, &seen, 10, 1);
        assert_eq!(all.len(), 4, "over-long lines are skipped");
    }
}
