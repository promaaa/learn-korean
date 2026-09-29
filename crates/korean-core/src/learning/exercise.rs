//! Turning a card into something to play.

use serde::Serialize;

use super::plan::Card;
use crate::content::{Item, ItemKind, Skill};
use crate::rng::Rng;

/// Number of options in a multiple-choice exercise.
pub const CHOICES: usize = 4;

/// What the UI shows. The correct answer is never part of it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Prompt {
    /// Hear/read Korean, pick the English meaning.
    #[serde(rename_all = "camelCase")]
    Listening {
        korean: String,
        options: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseView {
    pub card: Card,
    pub kind: ItemKind,
    pub prompt: Prompt,
}

/// A built exercise with its hidden answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Exercise {
    pub view: ExerciseView,
    pub correct: usize,
}

/// English meanings usable as distractors for word items without authored ones.
fn generated_distractors<'a>(item: &Item, pool: &[&'a Item], rng: &mut Rng) -> Vec<&'a str> {
    let verb_like = |e: &str| e.starts_with("to ");
    let same_shape = |other: &Item| verb_like(&other.english) == verb_like(&item.english);
    let mut candidates: Vec<&str> = pool
        .iter()
        .filter(|o| o.id != item.id && o.kind == item.kind && o.english != item.english)
        .filter(|o| same_shape(o))
        .map(|o| o.english.as_str())
        .collect();
    candidates.sort_unstable();
    candidates.dedup();
    rng.shuffle(&mut candidates);
    candidates.truncate(CHOICES - 1);
    candidates
}

pub fn build(card: &Card, item: &Item, pool: &[&Item], rng: &mut Rng) -> Exercise {
    match card.skill {
        Skill::Listening => {
            let mut options: Vec<String> = if item.distractors.is_empty() {
                generated_distractors(item, pool, rng)
                    .into_iter()
                    .map(String::from)
                    .collect()
            } else {
                item.distractors.clone()
            };
            options.push(item.english.clone());
            rng.shuffle(&mut options);
            let correct = options
                .iter()
                .position(|o| *o == item.english)
                .expect("correct option present");
            Exercise {
                view: ExerciseView {
                    card: card.clone(),
                    kind: item.kind,
                    prompt: Prompt::Listening {
                        korean: item.korean.clone(),
                        options,
                    },
                },
                correct,
            }
        }
        Skill::Response | Skill::Build => {
            unreachable!("{:?} exercises are not playable yet", card.skill)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Register;

    fn word(id: &str, english: &str) -> Item {
        Item {
            id: id.into(),
            kind: ItemKind::Word,
            korean: "고양이".into(),
            english: english.into(),
            context: "vocabulary".into(),
            register: Register::Neutral,
            note: None,
            lexemes: vec![],
            distractors: vec![],
            replies: None,
            chunks: None,
            image: None,
        }
    }

    fn options(ex: &Exercise) -> &[String] {
        let Prompt::Listening { options, .. } = &ex.view.prompt;
        options
    }

    #[test]
    fn authored_distractors_plus_the_answer_shuffled() {
        let mut item = word("a/1", "cat");
        item.kind = ItemKind::Sentence;
        item.distractors = vec!["dog".into(), "bird".into(), "fish".into()];
        let ex = build(
            &Card::new("a/1", Skill::Listening),
            &item,
            &[],
            &mut Rng::new(3),
        );
        let opts = options(&ex);
        assert_eq!(opts.len(), CHOICES);
        assert_eq!(opts[ex.correct], "cat");
        let mut sorted = opts.to_vec();
        sorted.sort();
        assert_eq!(sorted, ["bird", "cat", "dog", "fish"]);
    }

    #[test]
    fn word_distractors_come_from_words_of_the_same_shape() {
        let pool_items = [
            word("a/1", "cat"),
            word("a/2", "dog"),
            word("a/3", "to eat"),
            word("a/4", "house"),
            word("a/5", "to sleep"),
            word("a/6", "cat"),
            word("a/7", "tree"),
        ];
        let pool: Vec<&Item> = pool_items.iter().collect();
        for seed in 0..20 {
            let ex = build(
                &Card::new("a/1", Skill::Listening),
                &pool_items[0],
                &pool,
                &mut Rng::new(seed),
            );
            let opts = options(&ex);
            assert_eq!(opts.len(), CHOICES);
            assert_eq!(opts[ex.correct], "cat");
            assert_eq!(opts.iter().filter(|o| *o == "cat").count(), 1);
            assert!(opts.iter().all(|o| !o.starts_with("to ")), "{opts:?}");
        }
    }
}
