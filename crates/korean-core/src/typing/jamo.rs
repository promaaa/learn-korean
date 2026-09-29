//! Decomposition of precomposed Hangul syllables into compatibility jamo and keystrokes.

use std::fmt;

use serde::Serialize;

use super::layout::{Keystroke, keystroke_for_jamo};

pub(crate) const SYLLABLE_FIRST: u32 = 0xAC00;
pub(crate) const SYLLABLE_LAST: u32 = 0xD7A3;
const MEDIAL_COUNT: u32 = 21;
const FINAL_COUNT: u32 = 28;

/// The 19 initial consonants in Unicode order, as compatibility jamo.
pub(crate) const INITIALS: [char; 19] = [
    'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ',
    'ㅌ', 'ㅍ', 'ㅎ',
];

/// The 21 medial vowels in Unicode order, as compatibility jamo.
pub(crate) const MEDIALS: [char; 21] = [
    'ㅏ', 'ㅐ', 'ㅑ', 'ㅒ', 'ㅓ', 'ㅔ', 'ㅕ', 'ㅖ', 'ㅗ', 'ㅘ', 'ㅙ', 'ㅚ', 'ㅛ', 'ㅜ', 'ㅝ', 'ㅞ',
    'ㅟ', 'ㅠ', 'ㅡ', 'ㅢ', 'ㅣ',
];

/// The 27 final consonants in Unicode order (index + 1 = final index), as compatibility jamo.
pub(crate) const FINALS: [char; 27] = [
    'ㄱ', 'ㄲ', 'ㄳ', 'ㄴ', 'ㄵ', 'ㄶ', 'ㄷ', 'ㄹ', 'ㄺ', 'ㄻ', 'ㄼ', 'ㄽ', 'ㄾ', 'ㄿ', 'ㅀ', 'ㅁ',
    'ㅂ', 'ㅄ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];

/// Compound medials: (compound, first key, second key).
const COMPOUND_MEDIALS: [(char, char, char); 7] = [
    ('ㅘ', 'ㅗ', 'ㅏ'),
    ('ㅙ', 'ㅗ', 'ㅐ'),
    ('ㅚ', 'ㅗ', 'ㅣ'),
    ('ㅝ', 'ㅜ', 'ㅓ'),
    ('ㅞ', 'ㅜ', 'ㅔ'),
    ('ㅟ', 'ㅜ', 'ㅣ'),
    ('ㅢ', 'ㅡ', 'ㅣ'),
];

/// Compound finals: (compound, first key, second key).
const COMPOUND_FINALS: [(char, char, char); 11] = [
    ('ㄳ', 'ㄱ', 'ㅅ'),
    ('ㄵ', 'ㄴ', 'ㅈ'),
    ('ㄶ', 'ㄴ', 'ㅎ'),
    ('ㄺ', 'ㄹ', 'ㄱ'),
    ('ㄻ', 'ㄹ', 'ㅁ'),
    ('ㄼ', 'ㄹ', 'ㅂ'),
    ('ㄽ', 'ㄹ', 'ㅅ'),
    ('ㄾ', 'ㄹ', 'ㅌ'),
    ('ㄿ', 'ㄹ', 'ㅍ'),
    ('ㅀ', 'ㄹ', 'ㅎ'),
    ('ㅄ', 'ㅂ', 'ㅅ'),
];

pub(crate) fn initial_index(c: char) -> Option<u32> {
    INITIALS.iter().position(|&x| x == c).map(|i| i as u32)
}

pub(crate) fn medial_index(c: char) -> Option<u32> {
    MEDIALS.iter().position(|&x| x == c).map(|i| i as u32)
}

/// Final index as used in the syllable formula (1..=27; 0 means no final).
pub(crate) fn final_index(c: char) -> Option<u32> {
    FINALS.iter().position(|&x| x == c).map(|i| i as u32 + 1)
}

pub(crate) fn combine_medial(a: char, b: char) -> Option<char> {
    COMPOUND_MEDIALS
        .iter()
        .find(|&&(_, x, y)| x == a && y == b)
        .map(|&(c, _, _)| c)
}

pub(crate) fn combine_final(a: char, b: char) -> Option<char> {
    COMPOUND_FINALS
        .iter()
        .find(|&&(_, x, y)| x == a && y == b)
        .map(|&(c, _, _)| c)
}

pub(crate) fn split_final(c: char) -> Option<(char, char)> {
    COMPOUND_FINALS
        .iter()
        .find(|&&(x, _, _)| x == c)
        .map(|&(_, a, b)| (a, b))
}

fn split_medial(c: char) -> Option<(char, char)> {
    COMPOUND_MEDIALS
        .iter()
        .find(|&&(x, _, _)| x == c)
        .map(|&(_, a, b)| (a, b))
}

/// Builds a precomposed syllable from compatibility jamo, if the combination is valid.
pub(crate) fn compose_syllable(initial: char, medial: char, fin: Option<char>) -> Option<char> {
    let i = initial_index(initial)?;
    let m = medial_index(medial)?;
    let f = match fin {
        Some(c) => final_index(c)?,
        None => 0,
    };
    char::from_u32(SYLLABLE_FIRST + (i * MEDIAL_COUNT + m) * FINAL_COUNT + f)
}

/// Splits a precomposed syllable (U+AC00..U+D7A3) into `(initial, medial, final)`
/// compatibility jamo. Compound medials/finals are returned as single jamo (ㅘ, ㄳ, …).
pub fn decompose_syllable(c: char) -> Option<(char, char, Option<char>)> {
    let code = c as u32;
    if !(SYLLABLE_FIRST..=SYLLABLE_LAST).contains(&code) {
        return None;
    }
    let offset = code - SYLLABLE_FIRST;
    let i = offset / (MEDIAL_COUNT * FINAL_COUNT);
    let m = (offset / FINAL_COUNT) % MEDIAL_COUNT;
    let f = offset % FINAL_COUNT;
    let fin = (f > 0).then(|| FINALS[(f - 1) as usize]);
    Some((INITIALS[i as usize], MEDIALS[m as usize], fin))
}

/// A character of the input text that cannot be typed on the 2-set layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsupportedChar {
    /// Char (not byte) index in the text.
    pub index: usize,
    pub ch: char,
}

impl fmt::Display for UnsupportedChar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "character {:?} at index {} cannot be typed",
            self.ch, self.index
        )
    }
}

impl std::error::Error for UnsupportedChar {}

fn push_split(out: &mut Vec<(usize, char)>, index: usize, c: char, split: Option<(char, char)>) {
    match split {
        Some((a, b)) => {
            out.push((index, a));
            out.push((index, b));
        }
        None => out.push((index, c)),
    }
}

/// Every keystroke-level jamo (or space/punctuation) of `text`, tagged with the char index it
/// belongs to. Compound medials/finals are split; double consonants stay single.
pub fn indexed_jamo(text: &str) -> Result<Vec<(usize, char)>, UnsupportedChar> {
    let mut out = Vec::with_capacity(text.len());
    for (index, ch) in text.chars().enumerate() {
        if let Some((i, m, f)) = decompose_syllable(ch) {
            out.push((index, i));
            push_split(&mut out, index, m, split_medial(m));
            if let Some(f) = f {
                push_split(&mut out, index, f, split_final(f));
            }
        } else if keystroke_for_jamo(ch).is_some() {
            out.push((index, ch));
        } else {
            return Err(UnsupportedChar { index, ch });
        }
    }
    Ok(out)
}

/// The keystroke-level jamo sequence of `text` (input for [`super::compose::compose`]).
pub fn decompose_to_jamo(text: &str) -> Result<Vec<char>, UnsupportedChar> {
    Ok(indexed_jamo(text)?.into_iter().map(|(_, c)| c).collect())
}

/// The physical keystrokes needed to type `text`, tagged with the char index they belong to.
pub fn keystrokes_for_text(text: &str) -> Result<Vec<(usize, Keystroke)>, UnsupportedChar> {
    Ok(indexed_jamo(text)?
        .into_iter()
        .map(|(index, c)| {
            let ks = keystroke_for_jamo(c).expect("indexed_jamo yields only typeable jamo");
            (index, ks)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jamo(text: &str) -> String {
        decompose_to_jamo(text).unwrap().into_iter().collect()
    }

    #[test]
    fn decomposes_syllables() {
        assert_eq!(decompose_syllable('가'), Some(('ㄱ', 'ㅏ', None)));
        assert_eq!(decompose_syllable('힣'), Some(('ㅎ', 'ㅣ', Some('ㅎ'))));
        assert_eq!(decompose_syllable('괜'), Some(('ㄱ', 'ㅙ', Some('ㄴ'))));
        assert_eq!(decompose_syllable('닭'), Some(('ㄷ', 'ㅏ', Some('ㄺ'))));
        assert_eq!(decompose_syllable('ㄱ'), None);
        assert_eq!(decompose_syllable('a'), None);
    }

    #[test]
    fn syllable_formula_round_trips() {
        for code in SYLLABLE_FIRST..=SYLLABLE_LAST {
            let c = char::from_u32(code).unwrap();
            let (i, m, f) = decompose_syllable(c).unwrap();
            assert_eq!(compose_syllable(i, m, f), Some(c));
        }
    }

    #[test]
    fn decomposes_words_to_keystroke_jamo() {
        let cases = [
            ("한국", "ㅎㅏㄴㄱㅜㄱ"),
            ("앉아요", "ㅇㅏㄴㅈㅇㅏㅇㅛ"),
            ("괜찮아요", "ㄱㅗㅐㄴㅊㅏㄴㅎㅇㅏㅇㅛ"),
            ("읽어요", "ㅇㅣㄹㄱㅇㅓㅇㅛ"),
            ("뭐", "ㅁㅜㅓ"),
            ("왜", "ㅇㅗㅐ"),
            ("의사", "ㅇㅡㅣㅅㅏ"),
            ("없어요", "ㅇㅓㅂㅅㅇㅓㅇㅛ"),
            ("닭", "ㄷㅏㄹㄱ"),
            ("빨리", "ㅃㅏㄹㄹㅣ"),
            ("쓰다", "ㅆㅡㄷㅏ"),
            ("주세요?", "ㅈㅜㅅㅔㅇㅛ?"),
            ("뭐 해요?", "ㅁㅜㅓ ㅎㅐㅇㅛ?"),
        ];
        for (text, expected) in cases {
            assert_eq!(jamo(text), expected, "{text}");
        }
    }

    #[test]
    fn keystrokes_carry_char_index_and_shift() {
        let ks = keystrokes_for_text("빨리!").unwrap();
        let expected = [
            (0, "KeyQ", true),
            (0, "KeyK", false),
            (0, "KeyF", false),
            (1, "KeyF", false),
            (1, "KeyL", false),
            (2, "Digit1", true),
        ];
        assert_eq!(ks.len(), expected.len());
        for ((i, k), (ei, code, shift)) in ks.into_iter().zip(expected) {
            assert_eq!((i, k.code, k.shift), (ei, code, shift));
        }
        // Double consonants are one keystroke; compounds are two.
        assert_eq!(keystrokes_for_text("쓰").unwrap().len(), 2);
        assert_eq!(keystrokes_for_text("의").unwrap().len(), 3);
        assert_eq!(keystrokes_for_text("없").unwrap().len(), 4);
    }

    #[test]
    fn unsupported_chars_are_errors() {
        assert_eq!(
            keystrokes_for_text("안녕a"),
            Err(UnsupportedChar { index: 2, ch: 'a' })
        );
        assert_eq!(
            decompose_to_jamo("ㄳ"),
            Err(UnsupportedChar {
                index: 0, ch: 'ㄳ'
            })
        );
        assert!(keystrokes_for_text("1").is_err());
        assert_eq!(keystrokes_for_text("").unwrap(), vec![]);
    }
}
