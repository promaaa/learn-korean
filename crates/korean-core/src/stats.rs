//! Learning statistics: activity, retention, vocabulary and upcoming workload, derived from the
//! review log and the memory states. Day boundaries are local days, as for the streak.

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::content::{Item, ItemKind, Skill};
use crate::learning::{Card, is_known_word, word_cards};
use crate::progression::{day_streak, local_day};
use crate::scheduler::MemoryState;

/// Days in the activity heatmap: 26 weeks ending today.
pub const HEATMAP_DAYS: usize = 26 * 7;
/// Days in the recent window: answer time per day and recent retention.
pub const RECENT_DAYS: usize = 30;
/// Days in the workload forecast, today first.
pub const FORECAST_DAYS: usize = 14;

/// One review-log row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggedAnswer {
    pub item_id: String,
    pub skill: Skill,
    pub correct: bool,
    pub elapsed_ms: i64,
    pub answered_at: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayActivity {
    pub answers: u32,
    pub correct: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Heatmap {
    /// Local day number of `days[0]`.
    pub first_day: i64,
    /// Weekday of `days[0]`: 0 = Monday … 6 = Sunday.
    pub first_weekday: u8,
    /// Oldest first; the last entry is today.
    pub days: Vec<DayActivity>,
}

/// True retention: how often a card is remembered once a night has passed since its last answer.
/// Same-day repeats (learning steps, retries) are not reviews.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Retention {
    /// Answers whose previous answer to the same card was on an earlier local day.
    pub reviews: u32,
    pub correct: u32,
    /// `correct / reviews`; `None` without reviews.
    pub rate: Option<f64>,
}

impl Retention {
    fn add(&mut self, correct: bool) {
        self.reviews += 1;
        self.correct += u32::from(correct);
        self.rate = Some(f64::from(self.correct) / f64::from(self.reviews));
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRetention {
    pub skill: Skill,
    /// Reviews answered in the last `RECENT_DAYS` days.
    pub recent: Retention,
    pub all_time: Retention,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Words {
    /// Word items known (see `learning::is_known_word`).
    pub known: u32,
    /// Word items introduced but not known yet (still learning, or forgotten).
    pub learning: u32,
    pub total: u32,
    /// Sentences whose words are all known: the ones `Focus::Guided` serves.
    pub sentences_ready: u32,
    pub sentences: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub answers: u32,
    pub correct: u32,
    pub time_ms: i64,
    /// Days with at least one correct answer, as counted by the streak.
    pub practice_days: u32,
    pub day_streak: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    /// Today's local day number: the last day of every history series, the first of the forecast.
    pub today: i64,
    pub heatmap: Heatmap,
    /// Answer time per day in milliseconds, the last `RECENT_DAYS` days, oldest first.
    pub time_per_day: Vec<i64>,
    /// One entry per skill, in `Skill::ALL` order.
    pub retention: Vec<SkillRetention>,
    pub words: Words,
    /// Cards due per day, today first; today includes every overdue card.
    pub forecast: Vec<u32>,
    pub totals: Totals,
}

/// Index of `day` in a window of `len` days starting at `first`.
fn slot(day: i64, first: i64, len: usize) -> Option<usize> {
    usize::try_from(day - first).ok().filter(|&i| i < len)
}

/// Statistics at `now_ms`. `answers` must be in answer order (oldest first); `items` is the
/// whole bundled content, so vocabulary totals and the forecast cover every pack.
pub fn compute(
    answers: &[LoggedAnswer],
    states: &HashMap<Card, MemoryState>,
    items: &[Item],
    now_ms: i64,
    utc_offset_seconds: i32,
) -> Stats {
    let today = local_day(now_ms, utc_offset_seconds);
    let heatmap_first = today - (HEATMAP_DAYS as i64 - 1);
    let recent_first = today - (RECENT_DAYS as i64 - 1);

    let mut heatmap = vec![DayActivity::default(); HEATMAP_DAYS];
    let mut time_per_day = vec![0; RECENT_DAYS];
    let mut retention: Vec<SkillRetention> = Skill::ALL
        .iter()
        .map(|&skill| SkillRetention {
            skill,
            recent: Retention::default(),
            all_time: Retention::default(),
        })
        .collect();
    let mut last_answered: HashMap<(&str, Skill), i64> = HashMap::new();
    let mut practice_days = Vec::new();
    let mut totals = Totals::default();

    for answer in answers {
        let day = local_day(answer.answered_at, utc_offset_seconds);
        totals.answers += 1;
        totals.correct += u32::from(answer.correct);
        totals.time_ms += answer.elapsed_ms;
        if answer.correct {
            practice_days.push(day);
        }
        if let Some(i) = slot(day, heatmap_first, HEATMAP_DAYS) {
            heatmap[i].answers += 1;
            heatmap[i].correct += u32::from(answer.correct);
        }
        if let Some(i) = slot(day, recent_first, RECENT_DAYS) {
            time_per_day[i] += answer.elapsed_ms;
        }
        let previous = last_answered.insert((answer.item_id.as_str(), answer.skill), day);
        if previous.is_some_and(|p| p < day)
            && let Some(entry) = retention.iter_mut().find(|r| r.skill == answer.skill)
        {
            entry.all_time.add(answer.correct);
            if slot(day, recent_first, RECENT_DAYS).is_some() {
                entry.recent.add(answer.correct);
            }
        }
    }
    practice_days.sort_unstable();
    practice_days.dedup();
    totals.practice_days = practice_days.len() as u32;
    totals.day_streak = day_streak(&practice_days, today);

    let lemmas = word_cards(items);
    let known_lemma = |lemma: &str| {
        lemmas
            .get(lemma)
            .is_none_or(|ids| ids.iter().any(|id| is_known_word(states, id)))
    };
    let mut words = Words::default();
    for item in items {
        match item.kind {
            ItemKind::Word => {
                words.total += 1;
                if is_known_word(states, &item.id) {
                    words.known += 1;
                } else if states.contains_key(&Card::new(&item.id, Skill::Listening)) {
                    words.learning += 1;
                }
            }
            ItemKind::Sentence => {
                words.sentences += 1;
                if item.lexemes.iter().all(|l| known_lemma(l)) {
                    words.sentences_ready += 1;
                }
            }
        }
    }

    let content: HashSet<&str> = items.iter().map(|i| i.id.as_str()).collect();
    let mut forecast = vec![0; FORECAST_DAYS];
    for (card, state) in states {
        if !content.contains(card.item_id.as_str()) {
            continue;
        }
        let ahead = (local_day(state.due_at, utc_offset_seconds) - today).max(0);
        if let Some(i) = slot(ahead, 0, FORECAST_DAYS) {
            forecast[i] += 1;
        }
    }

    Stats {
        today,
        heatmap: Heatmap {
            first_day: heatmap_first,
            // The epoch, day 0, was a Thursday.
            first_weekday: (heatmap_first + 3).rem_euclid(7) as u8,
            days: heatmap,
        },
        time_per_day,
        retention,
        words,
        forecast,
        totals,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Register;
    use crate::scheduler::{DAY_MS, Phase, Rating, Scheduler};

    const HOUR_MS: i64 = 3_600_000;

    fn item(id: &str, kind: ItemKind, korean: &str, lexemes: &[&str]) -> Item {
        Item {
            id: id.into(),
            kind,
            korean: korean.into(),
            english: id.into(),
            context: "t".into(),
            register: Register::Polite,
            note: None,
            lexemes: lexemes.iter().map(|l| l.to_string()).collect(),
            distractors: vec!["a".into(), "b".into(), "c".into()],
            replies: None,
            chunks: None,
            image: None,
        }
    }

    fn word(id: &str, lemma: &str) -> Item {
        item(id, ItemKind::Word, lemma, &[lemma])
    }

    fn sentence(id: &str, lexemes: &[&str]) -> Item {
        item(id, ItemKind::Sentence, "문장이에요.", lexemes)
    }

    fn state(phase: Phase, due_at: i64) -> MemoryState {
        MemoryState {
            phase,
            due_at,
            ..Scheduler::with_defaults(0).review(None, Rating::Good, 0, 0.5)
        }
    }

    fn answer(item_id: &str, skill: Skill, correct: bool, answered_at: i64) -> LoggedAnswer {
        LoggedAnswer {
            item_id: item_id.into(),
            skill,
            correct,
            elapsed_ms: 2_000,
            answered_at,
        }
    }

    fn retention(stats: &Stats, skill: Skill) -> SkillRetention {
        stats
            .retention
            .iter()
            .find(|r| r.skill == skill)
            .unwrap()
            .clone()
    }

    #[test]
    fn retention_only_counts_answers_after_an_earlier_day() {
        let at = |day: i64, hour: i64| day * DAY_MS + hour * HOUR_MS;
        let answers = [
            answer("a/1", Skill::Listening, true, at(1, 9)), // first answer
            answer("a/1", Skill::Listening, false, at(1, 10)), // same day
            answer("a/1", Skill::Listening, true, at(2, 9)), // review, remembered
            answer("a/1", Skill::Listening, false, at(2, 10)), // same day again
            answer("a/1", Skill::Build, true, at(5, 9)),     // another card of the item: first
            answer("a/1", Skill::Listening, false, at(5, 9)), // review, forgotten
            answer("a/1", Skill::Listening, true, at(40, 9)), // review, within 30 days
        ];
        let stats = compute(&answers, &HashMap::new(), &[], at(40, 12), 0);

        let listening = retention(&stats, Skill::Listening);
        assert_eq!(
            (listening.all_time.reviews, listening.all_time.correct),
            (3, 2)
        );
        assert_eq!((listening.recent.reviews, listening.recent.correct), (1, 1));
        assert_eq!(listening.recent.rate, Some(1.0));
        let build = retention(&stats, Skill::Build);
        assert_eq!((build.all_time.reviews, build.all_time.rate), (0, None));
        assert_eq!(stats.totals.answers, 7);
    }

    #[test]
    fn retention_days_follow_the_time_zone() {
        // 23:00 and 01:00 UTC: two days in London, the same evening in New York (-4).
        let answers = [
            answer("a/1", Skill::Response, true, 10 * DAY_MS + 23 * HOUR_MS),
            answer("a/1", Skill::Response, true, 11 * DAY_MS + HOUR_MS),
        ];
        let now = 11 * DAY_MS + 12 * HOUR_MS;
        let reviews = |offset| {
            retention(
                &compute(&answers, &HashMap::new(), &[], now, offset),
                Skill::Response,
            )
            .all_time
            .reviews
        };
        assert_eq!(reviews(0), 1);
        assert_eq!(reviews(-4 * 3600), 0);
    }

    #[test]
    fn forecast_puts_overdue_cards_today_and_uses_local_days() {
        let items = [word("a/1", "하나"), word("a/2", "둘"), word("a/3", "셋")];
        let now = 100 * DAY_MS + 12 * HOUR_MS;
        let mut states = HashMap::new();
        let mut due = |id: &str, skill, due_at| {
            states.insert(Card::new(id, skill), state(Phase::Review, due_at));
        };
        due("a/1", Skill::Listening, 90 * DAY_MS); // overdue
        due("a/1", Skill::Response, 100 * DAY_MS + 20 * HOUR_MS); // later today
        due("a/2", Skill::Listening, 101 * DAY_MS + HOUR_MS / 2); // just after midnight UTC
        due("a/2", Skill::Build, 113 * DAY_MS + HOUR_MS); // last forecast day
        due("a/3", Skill::Listening, 114 * DAY_MS); // beyond the forecast
        due("gone/1", Skill::Listening, 90 * DAY_MS); // no longer bundled

        let utc = compute(&[], &states, &items, now, 0).forecast;
        assert_eq!(utc.len(), FORECAST_DAYS);
        assert_eq!((utc[0], utc[1], utc[13]), (2, 1, 1));
        assert_eq!(utc.iter().sum::<u32>(), 4);

        // One hour behind UTC, 00:30 UTC tomorrow is still today.
        let west = compute(&[], &states, &items, now, -3600).forecast;
        assert_eq!((west[0], west[1]), (3, 0));
    }

    #[test]
    fn heatmap_and_recent_time_cover_windows_ending_today() {
        let today = 200;
        let now = today * DAY_MS + 12 * HOUR_MS;
        let first = today - (HEATMAP_DAYS as i64 - 1);
        let recent_first = today - (RECENT_DAYS as i64 - 1);
        let answers = [
            answer("a/1", Skill::Listening, true, first * DAY_MS - 1), // day before the window
            answer("a/1", Skill::Listening, false, first * DAY_MS),
            answer("a/1", Skill::Listening, true, (recent_first - 1) * DAY_MS),
            answer("a/1", Skill::Listening, true, recent_first * DAY_MS),
            answer("a/1", Skill::Listening, true, (today - 1) * DAY_MS),
            answer("a/1", Skill::Listening, true, today * DAY_MS + HOUR_MS),
            answer("a/1", Skill::Listening, true, (today + 1) * DAY_MS), // another device's clock
        ];
        let stats = compute(&answers, &HashMap::new(), &[], now, 0);

        let heatmap = &stats.heatmap;
        assert_eq!(
            (heatmap.first_day, heatmap.days.len()),
            (first, HEATMAP_DAYS)
        );
        // 1970-01-20 was a Tuesday.
        assert_eq!(heatmap.first_weekday, 1);
        assert_eq!(
            heatmap.days[0],
            DayActivity {
                answers: 1,
                correct: 0
            }
        );
        assert_eq!(heatmap.days[HEATMAP_DAYS - 1].answers, 1);
        let in_window: u32 = heatmap.days.iter().map(|d| d.answers).sum();
        assert_eq!(in_window, 5);

        assert_eq!(stats.time_per_day.len(), RECENT_DAYS);
        assert_eq!(stats.time_per_day[0], 2_000);
        assert_eq!(stats.time_per_day.iter().sum::<i64>(), 3 * 2_000);

        assert_eq!(stats.totals.answers, 7);
        assert_eq!(stats.totals.time_ms, 7 * 2_000);
        assert_eq!(stats.totals.practice_days, 6);
        assert_eq!(stats.totals.day_streak, 2);
    }

    #[test]
    fn known_words_follow_the_planner_rule() {
        let items = [
            word("w/1", "먹다"),
            word("w/2", "가다"),
            item("w/3", ItemKind::Word, "가요", &["가다"]),
            word("w/4", "마시다"),
            word("w/5", "보다"),
            sentence("s/1", &["먹다", "이다"]),
            sentence("s/2", &["먹다", "가다"]),
            sentence("s/3", &["마시다"]),
        ];
        let mut states = HashMap::new();
        for (id, phase) in [
            ("w/1", Phase::Review),
            ("w/2", Phase::Learning),
            ("w/3", Phase::Review),
            ("w/4", Phase::Relearning),
        ] {
            states.insert(Card::new(id, Skill::Listening), state(phase, 0));
        }
        // Only listening decides: a reviewed build card does not make a word known.
        states.insert(Card::new("w/5", Skill::Build), state(Phase::Review, 0));

        let words = compute(&[], &states, &items, DAY_MS, 0).words;
        assert_eq!(
            words,
            Words {
                known: 2,
                learning: 2,
                total: 5,
                // s/1: 이다 has no word card; s/2: 가다 is known through w/3.
                sentences_ready: 2,
                sentences: 3,
            }
        );
    }
}
