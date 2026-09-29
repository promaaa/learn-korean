//! Which cards a session contains.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::content::{Item, ItemKind, Skill};
use crate::scheduler::{MemoryState, Phase};

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

/// How much of the content a session draws from; chosen by the learner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Focus {
    /// Word cards only: build vocabulary before meeting sentences.
    Words,
    /// Words first; a sentence is served once every word of it is known.
    #[default]
    Guided,
    /// Every card, words and sentences alike.
    All,
}

impl Focus {
    pub const ALL: [Focus; 3] = [Focus::Words, Focus::Guided, Focus::All];

    pub fn as_str(self) -> &'static str {
        match self {
            Focus::Words => "words",
            Focus::Guided => "guided",
            Focus::All => "all",
        }
    }

    pub fn parse(s: &str) -> Option<Focus> {
        Focus::ALL.into_iter().find(|focus| focus.as_str() == s)
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

/// Word cards of each lemma. Lemmas without a word card (particles, the copula…) are grammar,
/// taught through sentences only.
fn word_cards(items: &[Item]) -> HashMap<&str, Vec<&str>> {
    let mut words: HashMap<&str, Vec<&str>> = HashMap::new();
    for item in items.iter().filter(|i| i.kind == ItemKind::Word) {
        let lemmas: Vec<&str> = if item.lexemes.is_empty() {
            vec![item.korean.as_str()]
        } else {
            item.lexemes.iter().map(String::as_str).collect()
        };
        for lemma in lemmas {
            words.entry(lemma).or_default().push(item.id.as_str());
        }
    }
    words
}

/// Due cards first (most overdue first), then new cards, interleaved so a session never starts
/// with a wall of unknown material.
///
/// New cards: an item is introduced through `listening`; its other skills become available once
/// that card has been reviewed. New material is taken round-robin across packs, in content order
/// within a pack.
///
/// Vocabulary first (`Focus::Words` and `Focus::Guided`): a word is known once its listening card
/// has graduated to FSRS review. With `Guided`, a sentence whose words are all known is served
/// normally. Any other sentence waits, even when due, and its words not introduced yet are queued
/// as new cards in its place, so the words taught first are the ones sentences need next.
pub fn plan_session(
    items: &[Item],
    states: &HashMap<Card, MemoryState>,
    enabled: &[Skill],
    focus: Focus,
    now_ms: i64,
    limits: Limits,
) -> Plan {
    let words = word_cards(items);
    let listening = |id: &str| states.get(&Card::new(id, Skill::Listening));
    let known = |lemma: &str| {
        words.get(lemma).is_none_or(|ids| {
            ids.iter()
                .any(|id| listening(id).is_some_and(|s| s.phase == Phase::Review))
        })
    };

    let mut due: Vec<(&MemoryState, Card)> = Vec::new();
    let mut new_by_pack: Vec<(&str, VecDeque<Card>)> = Vec::new();
    let mut queued: HashSet<Card> = HashSet::new();
    let mut queue_new = |pack, card: Card| {
        if !queued.insert(card.clone()) {
            return;
        }
        match new_by_pack.iter_mut().find(|(p, _)| *p == pack) {
            Some((_, queue)) => queue.push_back(card),
            None => new_by_pack.push((pack, VecDeque::from([card]))),
        }
    };

    for item in items {
        let pack = pack_of(&item.id);
        if item.kind == ItemKind::Sentence && focus != Focus::All {
            let ready = focus == Focus::Guided && item.lexemes.iter().all(|l| known(l));
            if !ready {
                for lemma in &item.lexemes {
                    let Some(ids) = words.get(lemma.as_str()) else {
                        continue;
                    };
                    if enabled.contains(&Skill::Listening)
                        && ids.iter().all(|id| listening(id).is_none())
                    {
                        queue_new(pack, Card::new(ids[0], Skill::Listening));
                    }
                }
                continue;
            }
        }
        let introduced = listening(&item.id).is_some();
        for skill in item.skills() {
            if !enabled.contains(&skill) {
                continue;
            }
            let card = Card::new(&item.id, skill);
            match states.get(&card) {
                Some(state) if state.is_due(now_ms) => due.push((state, card)),
                Some(_) => {}
                None if skill == Skill::Listening || introduced => queue_new(pack, card),
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
    use crate::content::{Line, Register, Replies};
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

    fn word(id: &str, korean: &str) -> Item {
        Item {
            kind: ItemKind::Word,
            lexemes: vec![korean.into()],
            distractors: vec![],
            ..item(id, korean, false)
        }
    }

    fn sentence(id: &str, korean: &str, lexemes: &[&str]) -> Item {
        Item {
            lexemes: lexemes.iter().map(|l| l.to_string()).collect(),
            ..item(id, korean, false)
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
            Focus::Guided,
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
        let all = plan_session(
            &items,
            &states,
            &Skill::ALL,
            Focus::Guided,
            NOW,
            Limits::default(),
        );
        assert_eq!(ids(&all), [("a/1".into(), Skill::Response)]);
        let only_listening = plan_session(
            &items,
            &states,
            &[Skill::Listening],
            Focus::Guided,
            NOW,
            Limits::default(),
        );
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
            Focus::Guided,
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

    /// 가다 known (graduated), 오다 still in learning steps, 먹다 never seen.
    fn vocabulary() -> (Vec<Item>, HashMap<Card, MemoryState>) {
        let items = vec![
            word("a/go", "가다"),
            word("a/come", "오다"),
            sentence("a/go-eat", "가서 먹어요.", &["가다", "먹다", "이다"]),
            sentence("a/go-come", "가고 와요.", &["가다", "오다"]),
            sentence("a/go-only", "가요.", &["가다", "이다"]),
            word("a/sleep", "자다"),
            word("b/eat", "먹다"),
        ];
        let mut states = HashMap::new();
        let known = review(None, Rating::Easy, NOW - 1);
        assert_eq!(known.phase, Phase::Review);
        states.insert(Card::new("a/go", Skill::Listening), known);
        let learning = review(None, Rating::Good, NOW - 1);
        assert_eq!(learning.phase, Phase::Learning);
        states.insert(Card::new("a/come", Skill::Listening), learning);
        // Introduced earlier under `Focus::All` and due now.
        let mut due = review(None, Rating::Good, NOW - 1);
        due.due_at = NOW - 1;
        states.insert(Card::new("a/go-come", Skill::Listening), due);
        (items, states)
    }

    const WIDE: Limits = Limits {
        max_cards: 10,
        max_new: 10,
    };

    #[test]
    fn guided_serves_sentences_once_their_words_are_known_and_pulls_missing_words_first() {
        let (items, states) = vocabulary();
        let plan = plan_session(&items, &states, &Skill::ALL, Focus::Guided, NOW, WIDE);
        assert_eq!(
            ids(&plan),
            [
                // 먹다 lives in pack b but is needed by a/go-eat: taught in its place, once.
                ("b/eat".into(), Skill::Listening),
                // Only known words (a lemma without word card, 이다, is grammar).
                ("a/go-only".into(), Skill::Listening),
                ("a/sleep".into(), Skill::Listening),
            ]
        );
        // a/go-come is due but waits for 오다, which is still being learned.
        assert_eq!((plan.due, plan.new), (0, 3));
    }

    #[test]
    fn words_focus_serves_no_sentence_and_all_ignores_vocabulary() {
        let (items, states) = vocabulary();
        let words = plan_session(&items, &states, &Skill::ALL, Focus::Words, NOW, WIDE);
        assert_eq!(
            ids(&words),
            [
                ("b/eat".into(), Skill::Listening),
                ("a/sleep".into(), Skill::Listening),
            ]
        );
        let all = plan_session(&items, &states, &Skill::ALL, Focus::All, NOW, WIDE);
        assert_eq!(
            ids(&all),
            [
                ("a/go-come".into(), Skill::Listening),
                ("a/go-eat".into(), Skill::Listening),
                ("b/eat".into(), Skill::Listening),
                ("a/go-only".into(), Skill::Listening),
                ("a/sleep".into(), Skill::Listening),
            ]
        );
    }

    #[test]
    fn focus_names_round_trip() {
        for focus in Focus::ALL {
            assert_eq!(Focus::parse(focus.as_str()), Some(focus));
        }
        assert_eq!(Focus::parse("sentences"), None);
    }
}
