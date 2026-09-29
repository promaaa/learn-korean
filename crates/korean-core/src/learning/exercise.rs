//! Turning a card into something to play.

use serde::Serialize;

use super::plan::Card;
use crate::content::{Item, ItemKind, Line, Skill};
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
    /// Someone says `korean` to you: pick the natural Korean reply.
    #[serde(rename_all = "camelCase")]
    Response {
        korean: String,
        options: Vec<String>,
    },
}

impl Prompt {
    /// Options of a multiple-choice prompt.
    pub fn options(&self) -> &[String] {
        match self {
            Prompt::Listening { options, .. } | Prompt::Response { options, .. } => options,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseView {
    pub card: Card,
    pub kind: ItemKind,
    /// The item has a photo (fetched separately by id).
    pub image: bool,
    pub prompt: Prompt,
}

/// A built exercise with its hidden answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Exercise {
    pub view: ExerciseView,
    pub correct: usize,
    /// English for each option, revealed after answering (empty when options are English).
    pub translations: Vec<String>,
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
    let view = |prompt| ExerciseView {
        card: card.clone(),
        kind: item.kind,
        image: item.image.is_some(),
        prompt,
    };
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
                view: view(Prompt::Listening {
                    korean: item.korean.clone(),
                    options,
                }),
                correct,
                translations: Vec::new(),
            }
        }
        Skill::Response => {
            let replies = item
                .replies
                .as_ref()
                .expect("response cards are only planned for items with replies");
            let good = &replies.good[rng.below(replies.good.len())];
            let mut lines: Vec<&Line> = replies.bad.iter().take(CHOICES - 1).collect();
            lines.push(good);
            rng.shuffle(&mut lines);
            let correct = lines
                .iter()
                .position(|l| std::ptr::eq(*l, good))
                .expect("good reply present");
            Exercise {
                view: view(Prompt::Response {
                    korean: item.korean.clone(),
                    options: lines.iter().map(|l| l.korean.clone()).collect(),
                }),
                correct,
                translations: lines.iter().map(|l| l.english.clone()).collect(),
            }
        }
        Skill::Build => unreachable!("build exercises are not playable yet"),
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
        ex.view.prompt.options()
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

    #[test]
    fn response_offers_one_good_reply_among_the_bad_ones_with_translations() {
        use crate::content::Replies;
        let line = |k: &str, e: &str| Line {
            korean: k.into(),
            english: e.into(),
        };
        let mut item = word("a/1", "What did you do?");
        item.kind = ItemKind::Sentence;
        item.korean = "뭐 했어요?".into();
        item.replies = Some(Replies {
            good: vec![
                line("쉬었어요.", "I rested."),
                line("일했어요.", "I worked."),
            ],
            bad: vec![
                line("네, 맛있어요.", "Yes, it's tasty."),
                line("저는 학생이에요.", "I'm a student."),
                line("세 시예요.", "It's three."),
            ],
        });
        let mut seen_good = std::collections::HashSet::new();
        for seed in 0..30 {
            let ex = build(
                &Card::new("a/1", Skill::Response),
                &item,
                &[],
                &mut Rng::new(seed),
            );
            let Prompt::Response { korean, options } = &ex.view.prompt else {
                panic!("not a response prompt");
            };
            assert_eq!(korean, "뭐 했어요?");
            assert_eq!(options.len(), CHOICES);
            let answer = &options[ex.correct];
            assert!(answer == "쉬었어요." || answer == "일했어요.");
            seen_good.insert(answer.clone());
            let good_count = options
                .iter()
                .filter(|o| *o == "쉬었어요." || *o == "일했어요.")
                .count();
            assert_eq!(good_count, 1, "exactly one acceptable reply");
            assert_eq!(ex.translations.len(), CHOICES);
            let i = options.iter().position(|o| o == "세 시예요.").unwrap();
            assert_eq!(ex.translations[i], "It's three.");
        }
        assert_eq!(seen_good.len(), 2, "every good reply gets its turn");
    }
}
