//! 2-set (dubeolsik, KS X 5002) layout on physical keys identified by DOM `KeyboardEvent.code`.

use serde::Serialize;

/// One physical key press: DOM `KeyboardEvent.code` plus whether Shift is held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Keystroke {
    pub code: &'static str,
    pub shift: bool,
}

/// Touch-typing finger assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Finger {
    LeftPinky,
    LeftRing,
    LeftMiddle,
    LeftIndex,
    RightIndex,
    RightMiddle,
    RightRing,
    RightPinky,
    Thumb,
}

impl Finger {
    fn is_left(self) -> bool {
        matches!(
            self,
            Finger::LeftPinky | Finger::LeftRing | Finger::LeftMiddle | Finger::LeftIndex
        )
    }
}

/// The pinky that holds Shift while `for_key` presses its key: always the opposite hand.
/// The thumb (Space) is paired with the left pinky.
pub fn shift_finger(for_key: Finger) -> Finger {
    if for_key.is_left() {
        Finger::RightPinky
    } else {
        Finger::LeftPinky
    }
}

/// A key for on-screen keyboard rendering. `row` 0 is the digit row, 4 the space bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyCap {
    pub code: &'static str,
    pub row: u8,
    pub base: char,
    pub shifted: Option<char>,
    pub finger: Finger,
}

const fn key(
    code: &'static str,
    row: u8,
    base: char,
    shifted: Option<char>,
    finger: Finger,
) -> KeyCap {
    KeyCap {
        code,
        row,
        base,
        shifted,
        finger,
    }
}

use Finger::*;

static LAYOUT: [KeyCap; 31] = [
    key("Digit1", 0, '1', Some('!'), LeftPinky),
    key("KeyQ", 1, 'ㅂ', Some('ㅃ'), LeftPinky),
    key("KeyW", 1, 'ㅈ', Some('ㅉ'), LeftRing),
    key("KeyE", 1, 'ㄷ', Some('ㄸ'), LeftMiddle),
    key("KeyR", 1, 'ㄱ', Some('ㄲ'), LeftIndex),
    key("KeyT", 1, 'ㅅ', Some('ㅆ'), LeftIndex),
    key("KeyY", 1, 'ㅛ', None, RightIndex),
    key("KeyU", 1, 'ㅕ', None, RightIndex),
    key("KeyI", 1, 'ㅑ', None, RightMiddle),
    key("KeyO", 1, 'ㅐ', Some('ㅒ'), RightRing),
    key("KeyP", 1, 'ㅔ', Some('ㅖ'), RightPinky),
    key("KeyA", 2, 'ㅁ', None, LeftPinky),
    key("KeyS", 2, 'ㄴ', None, LeftRing),
    key("KeyD", 2, 'ㅇ', None, LeftMiddle),
    key("KeyF", 2, 'ㄹ', None, LeftIndex),
    key("KeyG", 2, 'ㅎ', None, LeftIndex),
    key("KeyH", 2, 'ㅗ', None, RightIndex),
    key("KeyJ", 2, 'ㅓ', None, RightIndex),
    key("KeyK", 2, 'ㅏ', None, RightMiddle),
    key("KeyL", 2, 'ㅣ', None, RightRing),
    key("KeyZ", 3, 'ㅋ', None, LeftPinky),
    key("KeyX", 3, 'ㅌ', None, LeftRing),
    key("KeyC", 3, 'ㅊ', None, LeftMiddle),
    key("KeyV", 3, 'ㅍ', None, LeftIndex),
    key("KeyB", 3, 'ㅠ', None, LeftIndex),
    key("KeyN", 3, 'ㅜ', None, RightIndex),
    key("KeyM", 3, 'ㅡ', None, RightIndex),
    key("Comma", 3, ',', None, RightMiddle),
    key("Period", 3, '.', None, RightRing),
    key("Slash", 3, '/', Some('?'), RightPinky),
    key("Space", 4, ' ', None, Thumb),
];

/// All rendered keys, ordered by row then left to right.
pub fn layout() -> &'static [KeyCap] {
    &LAYOUT
}

/// The key cap for a DOM `KeyboardEvent.code`, if it is part of the layout.
pub fn keycap(code: &str) -> Option<&'static KeyCap> {
    LAYOUT.iter().find(|k| k.code == code)
}

/// Characters the drill can ask for: the 33 keyboard jamo plus space and `. , ? !`.
fn is_typeable(c: char) -> bool {
    matches!(c, 'ㄱ'..='ㅣ' | ' ' | '.' | ',' | '?' | '!')
}

/// The keystroke producing `c`: one of the 33 jamo on the keyboard, or `' '`, `'.'`, `','`,
/// `'?'`, `'!'`. Compound jamo (ㅘ, ㄳ, …) are not single keystrokes and yield `None`.
pub fn keystroke_for_jamo(c: char) -> Option<Keystroke> {
    if !is_typeable(c) {
        return None;
    }
    LAYOUT.iter().find_map(|k| {
        if k.base == c {
            Some(Keystroke {
                code: k.code,
                shift: false,
            })
        } else if k.shifted == Some(c) {
            Some(Keystroke {
                code: k.code,
                shift: true,
            })
        } else {
            None
        }
    })
}

/// The character a key press produces. Shift on a key without a shifted character yields the
/// base character, like real IMEs.
pub fn jamo_for_keystroke(code: &str, shift: bool) -> Option<char> {
    keycap(code).map(|k| {
        if shift {
            k.shifted.unwrap_or(k.base)
        } else {
            k.base
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYBOARD_JAMO: &str =
        "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎㅏㅐㅑㅒㅓㅔㅕㅖㅗㅛㅜㅠㅡㅣ";

    #[test]
    fn every_keyboard_jamo_round_trips() {
        assert_eq!(KEYBOARD_JAMO.chars().count(), 33);
        for c in KEYBOARD_JAMO.chars().chain(" .,?!".chars()) {
            let ks = keystroke_for_jamo(c).unwrap_or_else(|| panic!("no key for {c}"));
            assert_eq!(jamo_for_keystroke(ks.code, ks.shift), Some(c), "{c}");
        }
    }

    #[test]
    fn every_layout_char_round_trips() {
        for k in layout() {
            for (c, shift) in [(Some(k.base), false), (k.shifted, true)] {
                let Some(c) = c else { continue };
                assert_eq!(jamo_for_keystroke(k.code, shift), Some(c));
                if let Some(ks) = keystroke_for_jamo(c) {
                    assert_eq!(
                        ks,
                        Keystroke {
                            code: k.code,
                            shift
                        }
                    );
                }
            }
        }
    }

    #[test]
    fn doubles_are_shifted_and_compounds_are_not_keys() {
        assert_eq!(
            keystroke_for_jamo('ㄲ'),
            Some(Keystroke {
                code: "KeyR",
                shift: true
            })
        );
        assert_eq!(
            keystroke_for_jamo('ㅖ'),
            Some(Keystroke {
                code: "KeyP",
                shift: true
            })
        );
        for c in "ㅘㅙㅚㅝㅞㅟㅢㄳㄵㄶㄺㄻㄼㄽㄾㄿㅀㅄ1/a가".chars() {
            assert_eq!(keystroke_for_jamo(c), None, "{c}");
        }
    }

    #[test]
    fn shift_without_shifted_char_yields_base() {
        assert_eq!(jamo_for_keystroke("KeyA", true), Some('ㅁ'));
        assert_eq!(jamo_for_keystroke("KeyA", false), Some('ㅁ'));
        assert_eq!(jamo_for_keystroke("KeyQ", true), Some('ㅃ'));
        assert_eq!(jamo_for_keystroke("Slash", true), Some('?'));
        assert_eq!(jamo_for_keystroke("Digit1", true), Some('!'));
        assert_eq!(jamo_for_keystroke("ShiftLeft", false), None);
    }

    #[test]
    fn finger_assignment() {
        let f = |code| keycap(code).unwrap().finger;
        assert_eq!(f("KeyQ"), LeftPinky);
        assert_eq!(f("KeyZ"), LeftPinky);
        assert_eq!(f("KeyG"), LeftIndex);
        assert_eq!(f("KeyH"), RightIndex);
        assert_eq!(f("KeyM"), RightIndex);
        assert_eq!(f("KeyK"), RightMiddle);
        assert_eq!(f("Period"), RightRing);
        assert_eq!(f("Slash"), RightPinky);
        assert_eq!(f("Space"), Thumb);
        assert_eq!(shift_finger(LeftIndex), RightPinky);
        assert_eq!(shift_finger(RightPinky), LeftPinky);
        assert_eq!(shift_finger(Thumb), LeftPinky);
    }

    #[test]
    fn layout_codes_are_unique_and_serialize_camel_case() {
        let mut codes: Vec<_> = layout().iter().map(|k| k.code).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), layout().len());
        let json = serde_json::to_value(keycap("KeyR").unwrap()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "code": "KeyR", "row": 1, "base": "ㄱ", "shifted": "ㄲ", "finger": "leftIndex"
            })
        );
    }
}
