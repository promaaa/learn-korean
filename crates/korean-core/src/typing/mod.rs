//! Hangul typing engine: 2-set (dubeolsik) layout on physical keys, jamo decomposition,
//! IME-style composition, a strict typing drill, rounds of drills, the keyboard course and the
//! Hangul primer. Pure logic: no I/O, no clock reads.

pub mod compose;
pub mod drill;
pub mod jamo;
pub mod keyboard;
pub mod layout;
pub mod primer;
mod round;
mod targets;

pub use compose::compose;
pub use drill::{Drill, DrillSnapshot, NextKey, PressOutcome};
pub use jamo::{
    UnsupportedChar, decompose_syllable, decompose_to_jamo, indexed_jamo, keystrokes_for_text,
};
pub use layout::{
    Finger, KeyCap, Keystroke, jamo_for_keystroke, keycap, keystroke_for_jamo, layout, shift_finger,
};
pub use round::Round;
pub use targets::{Target, pick_targets};
