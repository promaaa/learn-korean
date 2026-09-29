//! Turning an answer into a scheduler rating.

use crate::content::Skill;
use crate::scheduler::Rating;

/// Response-time bands `(easy_below, hard_above)` in milliseconds for a correct answer.
fn bands(skill: Skill, chunks: usize) -> (u64, u64) {
    match skill {
        Skill::Listening => (3_000, 9_000),
        Skill::Response => (4_000, 12_000),
        Skill::Build => {
            let n = chunks.max(1) as u64;
            (1_500 * n, 5_000 * n)
        }
    }
}

/// Wrong → Again; correct → Easy, Good or Hard depending on how long it took.
pub fn grade(skill: Skill, correct: bool, elapsed_ms: u64, chunks: usize) -> Rating {
    if !correct {
        return Rating::Again;
    }
    let (easy, hard) = bands(skill, chunks);
    if elapsed_ms < easy {
        Rating::Easy
    } else if elapsed_ms > hard {
        Rating::Hard
    } else {
        Rating::Good
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_is_always_again() {
        assert_eq!(grade(Skill::Listening, false, 500, 0), Rating::Again);
    }

    #[test]
    fn speed_decides_between_easy_good_and_hard() {
        assert_eq!(grade(Skill::Listening, true, 2_000, 0), Rating::Easy);
        assert_eq!(grade(Skill::Listening, true, 5_000, 0), Rating::Good);
        assert_eq!(grade(Skill::Listening, true, 20_000, 0), Rating::Hard);
    }

    #[test]
    fn build_time_scales_with_the_number_of_chunks() {
        assert_eq!(grade(Skill::Build, true, 5_000, 4), Rating::Easy);
        assert_eq!(grade(Skill::Build, true, 5_000, 2), Rating::Good);
        assert_eq!(grade(Skill::Build, true, 11_000, 2), Rating::Hard);
    }
}
