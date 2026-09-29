//! Which cards a session contains.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::content::{Item, ItemKind, Skill};
use crate::scheduler::{MINUTE_MS, MemoryState, Phase, Scheduler};

/// A (re)learning card due within this delay is shown now rather than left for a later session
/// (Anki's learn-ahead limit).
pub const LEARN_AHEAD_MS: i64 = 20 * MINUTE_MS;

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
    /// New cards introduced per learner day, across sessions (Anki's default: 20).
    pub new_per_day: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_cards: 12,
            max_new: 4,
            new_per_day: 20,
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

/// Due cards first, then new cards, interleaved so a session never starts with a wall of unknown
/// material. As in Anki:
///
/// - Cards in (re)learning steps come first, including those due within [`LEARN_AHEAD_MS`];
///   review cards follow, the least likely to be recalled first.
/// - One card per item per session: a sibling (the same item through another skill) waits for
///   a later session instead of being primed by the first.
/// - At most `limits.new_per_day` new cards per learner day; `introduced_today` were already.
///
/// New cards: an item is introduced through `listening`; its other skills become available once
/// that card has been reviewed. New material is taken round-robin across packs, in content order
/// within a pack.
///
/// Vocabulary first (`Focus::Words` and `Focus::Guided`): a word is known once its listening card
/// has graduated to FSRS review. With `Guided`, a sentence whose words are all known is served
/// normally. Any other sentence waits, even when due, and its words not introduced yet are queued
/// as new cards in its place, so the words taught first are the ones sentences need next.
#[allow(clippy::too_many_arguments)]
pub fn plan_session(
    items: &[Item],
    states: &HashMap<Card, MemoryState>,
    enabled: &[Skill],
    focus: Focus,
    now_ms: i64,
    limits: Limits,
    scheduler: &Scheduler,
    introduced_today: usize,
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
                Some(state)
                    if state.is_due(now_ms)
                        || (state.phase.is_learning()
                            && state.due_at <= now_ms + LEARN_AHEAD_MS) =>
                {
                    due.push((state, card))
                }
                Some(_) => {}
                None if skill == Skill::Listening || introduced => queue_new(pack, card),
                None => {}
            }
        }
    }

    // Learning cards by due time, then review cards by retrievability, lowest first.
    let priority = |state: &MemoryState| {
        if state.phase.is_learning() {
            (0, state.due_at as f64)
        } else {
            (1, scheduler.retrievability(state, now_ms))
        }
    };
    let mut due: Vec<((u8, f64), Card)> = due
        .into_iter()
        .map(|(state, card)| (priority(state), card))
        .collect();
    due.sort_by(|(a, ca), (b, cb)| {
        a.0.cmp(&b.0)
            .then(a.1.total_cmp(&b.1))
            .then_with(|| ca.item_id.cmp(&cb.item_id))
    });
    let mut served: HashSet<String> = HashSet::new();
    let due: Vec<Card> = due
        .into_iter()
        .map(|(_, card)| card)
        .filter(|card| served.insert(card.item_id.clone()))
        .take(limits.max_cards)
        .collect();

    let room = limits
        .max_new
        .min(limits.max_cards - due.len())
        .min(limits.new_per_day.saturating_sub(introduced_today));
    let mut new = Vec::with_capacity(room);
    while new.len() < room && new_by_pack.iter().any(|(_, q)| !q.is_empty()) {
        for (_, queue) in new_by_pack.iter_mut() {
            if new.len() == room {
                break;
            }
            if let Some(card) = queue.pop_front()
                && served.insert(card.item_id.clone())
            {
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
    use crate::scheduler::{DAY_MS, Rating};

    const NOW: i64 = 10_000_000_000;

    fn scheduler() -> Scheduler {
        Scheduler::with_defaults(0)
    }

    fn review(previous: Option<&MemoryState>, rating: Rating, at: i64) -> MemoryState {
        scheduler().review(previous, rating, at, 0.5)
    }

    fn plan(
        items: &[Item],
        states: &HashMap<Card, MemoryState>,
        enabled: &[Skill],
        focus: Focus,
        limits: Limits,
    ) -> Plan {
        plan_session(items, states, enabled, focus, NOW, limits, &scheduler(), 0)
    }

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

    fn numbered(n: u32) -> Vec<Item> {
        (0..n)
            .map(|i| {
                let korean = format!("{}예요.", char::from_u32(0xAC00 + i).unwrap());
                item(&format!("a/{i}"), &korean, false)
            })
            .collect()
    }

    fn ids(plan: &Plan) -> Vec<(String, Skill)> {
        plan.cards
            .iter()
            .map(|c| (c.item_id.clone(), c.skill))
            .collect()
    }

    fn limits(max_cards: usize, max_new: usize) -> Limits {
        Limits {
            max_cards,
            max_new,
            new_per_day: 20,
        }
    }

    #[test]
    fn new_items_start_with_listening_round_robin_across_packs() {
        let items = vec![
            item("a/1", "하나예요.", true),
            item("a/2", "둘이에요.", false),
            item("b/1", "셋이에요.", false),
        ];
        let plan = plan(
            &items,
            &HashMap::new(),
            &Skill::ALL,
            Focus::Guided,
            limits(10, 3),
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
        let seen = review(None, Rating::Easy, NOW - 1);
        states.insert(Card::new("a/1", Skill::Listening), seen);
        let all = plan(
            &items,
            &states,
            &Skill::ALL,
            Focus::Guided,
            Limits::default(),
        );
        assert_eq!(ids(&all), [("a/1".into(), Skill::Response)]);
        let only_listening = plan(
            &items,
            &states,
            &[Skill::Listening],
            Focus::Guided,
            Limits::default(),
        );
        assert!(only_listening.cards.is_empty());
    }

    #[test]
    fn learning_cards_come_first_by_due_time_within_the_learn_ahead_limit() {
        let items = numbered(6);
        let mut states = HashMap::new();
        for (i, due_in) in [(0, -5), (1, -50), (2, 15 * MINUTE_MS), (3, 25 * MINUTE_MS)] {
            let mut s = review(None, Rating::Good, NOW);
            assert_eq!(s.phase, Phase::Learning);
            s.due_at = NOW + due_in;
            states.insert(Card::new(format!("a/{i}"), Skill::Listening), s);
        }
        let plan = plan(&items, &states, &Skill::ALL, Focus::Guided, limits(4, 2));
        assert_eq!(
            ids(&plan),
            [
                ("a/1".into(), Skill::Listening),
                ("a/0".into(), Skill::Listening),
                ("a/4".into(), Skill::Listening),
                // Due in 15 minutes: learned ahead. a/3, due in 25, waits.
                ("a/2".into(), Skill::Listening),
            ]
        );
        assert_eq!((plan.due, plan.new), (3, 1));
    }

    #[test]
    fn review_cards_least_likely_to_be_recalled_come_first() {
        let items = numbered(3);
        let mut states = HashMap::new();
        // Same interval, reviewed longer ago = lower retrievability; a/2 is not due.
        for (i, last_review_days_ago) in [(0, 5), (1, 30), (2, 1)] {
            let mut s = review(None, Rating::Easy, NOW - last_review_days_ago * DAY_MS);
            s.due_at = if i == 2 { NOW + DAY_MS } else { NOW - 1 };
            states.insert(Card::new(format!("a/{i}"), Skill::Listening), s);
        }
        let plan = plan(&items, &states, &Skill::ALL, Focus::Guided, limits(4, 0));
        assert_eq!(
            ids(&plan),
            [
                ("a/1".into(), Skill::Listening),
                ("a/0".into(), Skill::Listening),
            ]
        );
    }

    #[test]
    fn one_card_per_item_per_session() {
        let items = vec![
            item("a/1", "하나예요.", true),
            item("a/2", "둘이에요.", true),
        ];
        let mut states = HashMap::new();
        for skill in [Skill::Listening, Skill::Response] {
            let mut s = review(None, Rating::Easy, NOW - 10 * DAY_MS);
            s.due_at = NOW - 1;
            states.insert(Card::new("a/1", skill), s);
        }
        // a/2 listening reviewed: its response card is new, a/2 listening not due.
        states.insert(
            Card::new("a/2", Skill::Listening),
            review(None, Rating::Easy, NOW - 1),
        );
        let plan = plan(&items, &states, &Skill::ALL, Focus::Guided, limits(10, 10));
        assert_eq!(plan.due, 1, "the sibling due card waits: {:?}", ids(&plan));
        assert_eq!(plan.cards.len(), 2);
        assert_eq!(plan.cards[0].item_id, "a/1");
        assert_eq!(plan.cards[1], Card::new("a/2", Skill::Response));
    }

    #[test]
    fn new_cards_stop_at_the_daily_limit_across_sessions() {
        let items = numbered(10);
        let daily = Limits {
            max_cards: 12,
            max_new: 4,
            new_per_day: 5,
        };
        let s = scheduler();
        let none = HashMap::new();
        let new_after = |introduced| {
            plan_session(
                &items,
                &none,
                &Skill::ALL,
                Focus::Guided,
                NOW,
                daily,
                &s,
                introduced,
            )
            .new
        };
        assert_eq!((new_after(0), new_after(4), new_after(5)), (4, 1, 0));
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
        // In learning steps, next due beyond the learn-ahead limit.
        let mut learning = review(None, Rating::Good, NOW - 1);
        assert_eq!(learning.phase, Phase::Learning);
        learning.due_at = NOW + LEARN_AHEAD_MS + 1;
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
        new_per_day: 20,
    };

    #[test]
    fn guided_serves_sentences_once_their_words_are_known_and_pulls_missing_words_first() {
        let (items, states) = vocabulary();
        let plan = plan(&items, &states, &Skill::ALL, Focus::Guided, WIDE);
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
        let words = plan(&items, &states, &Skill::ALL, Focus::Words, WIDE);
        assert_eq!(
            ids(&words),
            [
                ("b/eat".into(), Skill::Listening),
                ("a/sleep".into(), Skill::Listening),
            ]
        );
        let all = plan(&items, &states, &Skill::ALL, Focus::All, WIDE);
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
