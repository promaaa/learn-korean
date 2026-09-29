//! Experience points, levels, daily streak and pack unlocks.

use serde::Serialize;

use crate::scheduler::Rating;

const BASE_XP: u32 = 10;
const EASY_BONUS: u32 = 5;
const NEW_CARD_BONUS: u32 = 5;
/// A miss still earns a little: showing up is the habit being built.
const MISS_XP: u32 = 2;
/// The combo multiplier grows by 10 % per consecutive correct answer, up to ×2.
const MAX_COMBO_STEPS: u32 = 10;

/// XP for one answer. `combo` is the streak *including* this answer.
pub fn xp_for_answer(correct: bool, rating: Rating, combo: u32, first_review: bool) -> u32 {
    if !correct {
        return MISS_XP;
    }
    let mut xp = BASE_XP;
    if rating == Rating::Easy {
        xp += EASY_BONUS;
    }
    if first_review {
        xp += NEW_CARD_BONUS;
    }
    let steps = combo.saturating_sub(1).min(MAX_COMBO_STEPS);
    xp * (10 + steps) / 10
}

/// XP needed to go from `level` to `level + 1`.
pub fn xp_to_next(level: u32) -> u64 {
    100 + 50 * u64::from(level.saturating_sub(1))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Level {
    pub level: u32,
    pub total_xp: u64,
    /// XP earned since reaching `level`.
    pub xp_in_level: u64,
    pub xp_for_next: u64,
}

pub fn level_for(total_xp: u64) -> Level {
    let mut level = 1;
    let mut remaining = total_xp;
    while remaining >= xp_to_next(level) {
        remaining -= xp_to_next(level);
        level += 1;
    }
    Level {
        level,
        total_xp,
        xp_in_level: remaining,
        xp_for_next: xp_to_next(level),
    }
}

/// Consecutive days with practice ending today (or yesterday, if today has no practice yet).
/// `days` are local day numbers (days since the epoch in the learner's time zone), any order.
pub fn day_streak(days: &[i64], today: i64) -> u32 {
    let mut sorted: Vec<i64> = days.iter().copied().filter(|d| *d <= today).collect();
    sorted.sort_unstable();
    sorted.dedup();
    let Some(&last) = sorted.last() else {
        return 0;
    };
    if last < today - 1 {
        return 0;
    }
    let mut streak = 1;
    for pair in sorted.windows(2).rev() {
        if pair[1] - pair[0] == 1 {
            streak += 1;
        } else {
            break;
        }
    }
    streak
}

/// Local day number of a unix-millisecond instant for a UTC offset in seconds.
pub fn local_day(unix_ms: i64, utc_offset_seconds: i32) -> i64 {
    (unix_ms + i64::from(utc_offset_seconds) * 1_000).div_euclid(86_400_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_earn_base_xp_with_bonuses_and_combo() {
        assert_eq!(xp_for_answer(true, Rating::Good, 1, false), 10);
        assert_eq!(xp_for_answer(true, Rating::Easy, 1, false), 15);
        assert_eq!(xp_for_answer(true, Rating::Good, 1, true), 15);
        assert_eq!(
            xp_for_answer(true, Rating::Good, 6, false),
            15,
            "×1.5 at streak 6"
        );
        assert_eq!(
            xp_for_answer(true, Rating::Easy, 50, true),
            40,
            "combo capped at ×2"
        );
        assert_eq!(xp_for_answer(false, Rating::Again, 0, true), MISS_XP);
    }

    #[test]
    fn levels_get_progressively_longer() {
        assert_eq!(level_for(0).level, 1);
        assert_eq!(level_for(99).level, 1);
        let two = level_for(100);
        assert_eq!((two.level, two.xp_in_level, two.xp_for_next), (2, 0, 150));
        assert_eq!(level_for(249).level, 2);
        assert_eq!(level_for(250).level, 3);
        let seven = level_for(1_840);
        assert_eq!(seven.level, 8);
        assert_eq!(seven.xp_in_level, 1_840 - 1_750);
    }

    #[test]
    fn day_streak_counts_consecutive_days_up_to_today_or_yesterday() {
        assert_eq!(day_streak(&[], 100), 0);
        assert_eq!(day_streak(&[100], 100), 1);
        assert_eq!(day_streak(&[98, 99, 100, 100], 100), 3);
        assert_eq!(day_streak(&[97, 98, 99], 100), 3, "today not played yet");
        assert_eq!(day_streak(&[90, 91, 98], 100), 0, "broken");
        assert_eq!(day_streak(&[95, 97, 98, 99, 100], 100), 4);
    }

    #[test]
    fn local_days_follow_the_time_zone() {
        // 2026-09-28 23:30 UTC is already the 29th in Seoul (+9), still the 28th in New York (-4).
        let instant = 1_790_638_200_000;
        assert_eq!(
            local_day(instant, 9 * 3600) - local_day(instant, -4 * 3600),
            1
        );
        assert_eq!(local_day(instant, 0), local_day(instant, -4 * 3600));
        assert_eq!(local_day(-1, 0), -1);
    }
}
