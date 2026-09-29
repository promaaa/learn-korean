//! Standard 2-set IME composition automaton.

use super::jamo::{
    combine_final, combine_medial, compose_syllable, final_index, initial_index, medial_index,
    split_final,
};

/// The syllable currently being composed.
#[derive(Default)]
struct Composer {
    out: String,
    initial: Option<char>,
    medial: Option<char>,
    fin: Option<char>,
}

impl Composer {
    /// Emits the pending syllable (or lone jamo) and clears it.
    fn flush(&mut self) {
        match (self.initial.take(), self.medial.take(), self.fin.take()) {
            (Some(i), Some(m), f) => {
                let s = compose_syllable(i, m, f).expect("composer holds only valid jamo");
                self.out.push(s);
            }
            (Some(i), None, _) => self.out.push(i),
            (None, Some(m), _) => self.out.push(m),
            (None, None, _) => {}
        }
    }

    fn start_with_consonant(&mut self, c: char) {
        self.flush();
        if initial_index(c).is_some() {
            self.initial = Some(c);
        } else {
            // Compound consonants (ㄳ, …) cannot start a syllable: render as-is.
            self.out.push(c);
        }
    }

    fn consonant(&mut self, c: char) {
        match (self.initial, self.medial, self.fin) {
            (Some(_), Some(_), None) if final_index(c).is_some() => self.fin = Some(c),
            (Some(_), Some(_), Some(f)) => match combine_final(f, c) {
                Some(compound) => self.fin = Some(compound),
                None => self.start_with_consonant(c),
            },
            _ => self.start_with_consonant(c),
        }
    }

    fn vowel(&mut self, v: char) {
        match (self.initial, self.medial, self.fin) {
            (Some(_), Some(_), Some(f)) => {
                // The final (or the second half of a compound final) starts the next syllable.
                let moved = match split_final(f) {
                    Some((kept, moved)) => {
                        self.fin = Some(kept);
                        moved
                    }
                    None => {
                        self.fin = None;
                        f
                    }
                };
                self.flush();
                self.initial = Some(moved);
                self.medial = Some(v);
            }
            (Some(_), None, _) => self.medial = Some(v),
            (_, Some(m), None) => match combine_medial(m, v) {
                Some(compound) => self.medial = Some(compound),
                None => {
                    self.flush();
                    self.medial = Some(v);
                }
            },
            (None, None, _) => self.medial = Some(v),
            (None, Some(_), Some(_)) => unreachable!("a final always has an initial"),
        }
    }

    fn other(&mut self, c: char) {
        self.flush();
        self.out.push(c);
    }
}

/// Composes compatibility jamo (plus spaces/punctuation, which flush the pending syllable)
/// into text, exactly as a 2-set IME displays it while typing.
pub fn compose(jamo_seq: &[char]) -> String {
    let mut composer = Composer::default();
    for &c in jamo_seq {
        if medial_index(c).is_some() {
            composer.vowel(c);
        } else if ('ㄱ'..='ㅎ').contains(&c) {
            composer.consonant(c);
        } else {
            composer.other(c);
        }
    }
    composer.flush();
    composer.out
}

#[cfg(test)]
mod tests {
    use super::super::jamo::{SYLLABLE_FIRST, SYLLABLE_LAST, decompose_to_jamo};
    use super::*;

    fn c(s: &str) -> String {
        compose(&s.chars().collect::<Vec<_>>())
    }

    fn round_trip(s: &str) {
        let jamo = decompose_to_jamo(s).unwrap();
        assert_eq!(compose(&jamo), s, "jamo {jamo:?}");
    }

    fn prefixes(s: &str) -> Vec<String> {
        let jamo = decompose_to_jamo(s).unwrap();
        (1..=jamo.len()).map(|n| compose(&jamo[..n])).collect()
    }

    #[test]
    fn live_prefixes_of_hanguk() {
        assert_eq!(prefixes("한국"), ["ㅎ", "하", "한", "한ㄱ", "한구", "한국"]);
    }

    #[test]
    fn live_prefixes_of_anja() {
        assert_eq!(prefixes("앉아"), ["ㅇ", "아", "안", "앉", "앉ㅇ", "앉아"]);
    }

    #[test]
    fn compound_final_splits_on_vowel() {
        assert_eq!(c("ㅇㅏㄴㅈㅏ"), "안자");
        assert_eq!(c("ㄷㅏㄹㄱㅗ"), "달고");
        assert_eq!(c("ㅇㅓㅂㅅㅓ"), "업서");
        assert_eq!(c("ㅎㅏㄴㄱㅜ"), "한구");
    }

    #[test]
    fn vowels_combine_only_into_valid_compounds() {
        assert_eq!(c("ㄱㅗㅏ"), "과");
        assert_eq!(c("ㅇㅡㅣ"), "의");
        assert_eq!(c("ㄱㅏㅏ"), "가ㅏ");
        assert_eq!(c("ㄱㅗㅓ"), "고ㅓ");
        assert_eq!(c("ㅗㅏ"), "ㅘ");
        assert_eq!(c("ㅏㅏ"), "ㅏㅏ");
    }

    #[test]
    fn finals_combine_only_into_valid_compounds() {
        assert_eq!(c("ㄷㅏㄹㄱ"), "닭");
        assert_eq!(c("ㄷㅏㄱㄹ"), "닥ㄹ");
        assert_eq!(c("ㅇㅣㄹㄱㄱ"), "읽ㄱ");
        assert_eq!(c("ㄱㅏㄲ"), "갂");
        assert_eq!(c("ㄱㅏㅆㅏ"), "가싸");
    }

    #[test]
    fn tense_stops_never_become_finals() {
        assert_eq!(c("ㄱㅏㄸ"), "가ㄸ");
        assert_eq!(c("ㄱㅏㅃㅏ"), "가빠");
        assert_eq!(c("ㄱㅏㅉ"), "가ㅉ");
    }

    #[test]
    fn lone_jamo_and_punctuation() {
        assert_eq!(c(""), "");
        assert_eq!(c("ㄱ"), "ㄱ");
        assert_eq!(c("ㄱㄴ"), "ㄱㄴ");
        assert_eq!(c("ㅏ"), "ㅏ");
        assert_eq!(c("ㅎㅏㄴ ㄱ"), "한 ㄱ");
        assert_eq!(c("ㄴㅔ."), "네.");
        assert_eq!(c("ㅁㅜㅓ?ㅏ"), "뭐?ㅏ");
        assert_eq!(c("ㄳ"), "ㄳ");
    }

    #[test]
    fn every_syllable_round_trips_in_isolation() {
        for code in SYLLABLE_FIRST..=SYLLABLE_LAST {
            round_trip(&char::from_u32(code).unwrap().to_string());
        }
    }

    #[test]
    fn syllable_pairs_round_trip() {
        for a in (SYLLABLE_FIRST..=SYLLABLE_LAST).step_by(7 * 13) {
            for b in (SYLLABLE_FIRST + 3..=SYLLABLE_LAST).step_by(7 * 11) {
                let s: String = [a, b].iter().map(|&x| char::from_u32(x).unwrap()).collect();
                round_trip(&s);
            }
        }
    }

    #[test]
    fn words_and_sentences_round_trip() {
        for s in [
            "한국",
            "앉아요",
            "괜찮아요",
            "읽었어요",
            "읽어요",
            "닭고기",
            "없어요",
            "의사",
            "뭐 해요?",
            "왜요?",
            "빨리 와요!",
            "쓰다",
            "주세요?",
            "많이 먹었어요.",
            "값이 싸요",
            "삶은 달걀",
            "여덟 시",
            "핥아요",
            "싫어요",
            "넓어요",
            "외국 사람이에요",
            "뭐예요?",
            "네, 괜찮아요.",
            "꽃을 샀어요",
            "밝아요",
            "짧아요",
            "읊어요",
            "몫이에요",
            "앉으세요",
            "안녕하세요?",
            "감사합니다!",
            "귀여워요",
            "희망",
            "쬐다",
        ] {
            round_trip(s);
        }
    }
}
