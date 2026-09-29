//! Which cards a session contains.

use std::collections::{HashMap, VecDeque};

use serde::Serialize;

use crate::content::{Item, Skill};
use crate::scheduler::MemoryState;

/// One schedulable unit: an item trained through one skill.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub item_id: String,
    pub skill: Skill,
}

impl Card {
    pub fn new(item_id: impl Into<String>, skill: Skill) -> Self {
        Card {
            item_id: item_id.into(),
            skill,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_cards: usize,
    pub max_new: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_cards: 12,
            max_new: 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub cards: Vec<Card>,
    pub due: usize,
    pub new: usize,
}

fn pack_of(item_id: &str) -> &str {
    item_id.split_once('/').map_or(item_id, |(pack, _)| pack)
}

/// Due cards first (most overdue first), then new cards, interleaved so a session never starts
/// with a wall of unknown material.
///
/// New cards: an item is introduced through `listening`; its other skills become available once
/// that card has been reviewed. New material is taken round-robin across packs, in content order
/// within a pack.
pub fn plan_session(
    items: &[Item],
    states: &HashMap<Card, MemoryState>,
    enabled: &[Skill],
    now_ms: i64,
    limits: Limits,
) -> Plan {
    let mut due: Vec<(&MemoryState, Card)> = Vec::new();
    let mut new_by_pack: Vec<(&str, VecDeque<Card>)> = Vec::new();

    for item in items {
        let introduced = states.contains_key(&Card::new(&item.id, Skill::Listening));
        for skill in item.skills() {
            if !enabled.contains(&skill) {
                continue;
            }
            let card = Card::new(&item.id, skill);
            match states.get(&card) {
                Some(state) if state.is_due(now_ms) => due.push((state, card)),
                Some(_) => {}
                None if skill == Skill::Listening || introduced => {
                    let pack = pack_of(&item.id);
                    match new_by_pack.iter_mut().find(|(p, _)| *p == pack) {
                        Some((_, queue)) => queue.push_back(card),
                        None => new_by_pack.push((pack, VecDeque::from([card]))),
                    }
                }
                None => {}
            }
        }
    }

    due.sort_by_key(|(state, card)| (state.due_at, card.item_id.clone()));
    let due: Vec<Card> = due
        .into_iter()
        .take(limits.max_cards)
        .map(|(_, c)| c)
        .collect();

    let room = limits.max_new.min(limits.max_cards - due.len());
    let mut new = Vec::with_capacity(room);
    while new.len() < room && new_by_pack.iter().any(|(_, q)| !q.is_empty()) {
        for (_, queue) in new_by_pack.iter_mut() {
            if new.len() == room {
                break;
            }
            if let Some(card) = queue.pop_front() {
                new.push(card);
            }
        }
    }

    let (due_count, new_count) = (due.len(), new.len());
    let mut cards = Vec::with_capacity(due_count + new_count);
    let mut due = due.into_iter();
    let mut new = new.into_iter();
    loop {
        let a = due.next();
        let b = due.next();
        let c = new.next();
        if a.is_none() && b.is_none() && c.is_none() {
            break;
        }
        cards.extend(a.into_iter().chain(b).chain(c));
    }
    Plan {
        cards,
        due: due_count,
        new: new_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{ItemKind, Line, Register, Replies};
    use crate::scheduler::{Rating, review};

    const NOW: i64 = 10_000_000_000;

    fn item(id: &str, korean: &str, replies: bool) -> Item {
        let line = |k: &str| Line {
            korean: k.into(),
            english: "x".into(),
        };
        Item {
            id: id.into(),
            kind: ItemKind::Sentence,
            korean: korean.into(),
            english: id.into(),
            context: "t".into(),
            register: Register::Polite,
            note: None,
            lexemes: vec![],
            distractors: vec!["a".into(), "b".into(), "c".into()],
            replies: replies.then(|| Replies {
                good: vec![line("네.")],
                bad: vec![line("가."), line("나."), line("다.")],
            }),
            chunks: None,
            image: None,
        }
    }

    fn ids(plan: &Plan) -> Vec<(String, Skill)> {
        plan.cards
            .iter()
            .map(|c| (c.item_id.clone(), c.skill))
            .collect()
    }

    #[test]
    fn new_items_start_with_listening_round_robin_across_packs() {
        let items = vec![
            item("a/1", "하나예요.", true),
            item("a/2", "둘이에요.", false),
            item("b/1", "셋이에요.", false),
        ];
        let plan = plan_session(
            &items,
            &HashMap::new(),
            &Skill::ALL,
            NOW,
            Limits {
                max_cards: 10,
                max_new: 3,
            },
        );
        assert_eq!(
            ids(&plan),
            [
                ("a/1".into(), Skill::Listening),
                ("b/1".into(), Skill::Listening),
                ("a/2".into(), Skill::Listening),
            ]
        );
        assert_eq!((plan.due, plan.new), (0, 3));
    }

    #[test]
    fn other_skills_unlock_after_listening_and_disabled_skills_are_skipped() {
        let items = vec![item("a/1", "하나예요.", true)];
        let mut states = HashMap::new();
        let seen = review(None, Rating::Good, NOW - 1);
        states.insert(Card::new("a/1", Skill::Listening), seen);
        let all = plan_session(&items, &states, &Skill::ALL, NOW, Limits::default());
        assert_eq!(ids(&all), [("a/1".into(), Skill::Response)]);
        let only_listening =
            plan_session(&items, &states, &[Skill::Listening], NOW, Limits::default());
        assert!(only_listening.cards.is_empty());
    }

    #[test]
    fn due_cards_come_first_most_overdue_first_and_respect_limits() {
        let items: Vec<Item> = (0..6)
            .map(|i| {
                let korean = format!("{}예요.", char::from_u32(0xAC00 + i).unwrap());
                item(&format!("a/{i}"), &korean, false)
            })
            .collect();
        let mut states = HashMap::new();
        for (i, overdue) in [(0, 5), (1, 50), (2, 20)] {
            let mut s = review(None, Rating::Good, NOW);
            s.due_at = NOW - overdue;
            states.insert(Card::new(format!("a/{i}"), Skill::Listening), s);
        }
        let mut later = review(None, Rating::Good, NOW);
        later.due_at = NOW + 1;
        states.insert(Card::new("a/3", Skill::Listening), later);

        let plan = plan_session(
            &items,
            &states,
            &Skill::ALL,
            NOW,
            Limits {
                max_cards: 4,
                max_new: 2,
            },
        );
        assert_eq!(
            ids(&plan),
            [
                ("a/1".into(), Skill::Listening),
                ("a/2".into(), Skill::Listening),
                ("a/4".into(), Skill::Listening),
                ("a/0".into(), Skill::Listening),
            ]
        );
        assert_eq!((plan.due, plan.new), (3, 1));
    }
}
