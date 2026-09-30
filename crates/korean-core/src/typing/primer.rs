//! Hangul primer: ordered lessons that introduce the jamo a few at a time and drill them on the
//! 2-set keyboard, from the lone jamo to syllables to real words (`content/hangul/primer.json`).

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::compose::compose;
use super::jamo::{decompose_syllable, indexed_jamo};
use super::round::Round;
use super::targets::Target;
use crate::content::{Line, ValidationError, is_hangul_text, is_kebab};

const BUNDLED: &str = include_str!("../../../../content/hangul/primer.json");

/// A lesson is passed when a full round is typed with at least this accuracy, in percent.
pub const PASS_PERCENT: usize = 90;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Primer {
    pub lessons: Vec<Lesson>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lesson {
    /// Kebab-case, permanent once released: passes are stored by lesson id.
    pub id: String,
    pub title: String,
    /// The jamo this lesson introduces, in teaching order.
    pub jamo: Vec<NewJamo>,
    /// The round to type, from the lone new jamo to syllables to words.
    pub lines: Vec<PrimerLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewJamo {
    /// A compatibility jamo (ㄱ, ㅏ, ㅘ, ㄺ…).
    pub jamo: char,
    /// Its Hangul name: 기역, 니은, 아…
    pub name: String,
    /// A word containing it.
    pub example: Line,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimerLine {
    pub korean: String,
    /// Words carry their meaning; syllable drills omit it and show their assembly instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub english: Option<String>,
}

/// The primer shipped inside the binary, validated in CI.
pub fn bundled_primer() -> Primer {
    serde_json::from_str(BUNDLED)
        .unwrap_or_else(|e| panic!("bundled content/hangul/primer.json is invalid: {e}"))
}

/// A modern compatibility jamo (U+3131 ㄱ..=U+3163 ㅣ), compounds included.
fn is_jamo(c: char) -> bool {
    ('ㄱ'..='ㅣ').contains(&c)
}

/// Every jamo a learner must know to read and type `text`: each keystroke jamo, plus compound
/// vowels and finals themselves (과 needs ㄱ, ㅗ, ㅏ and ㅘ). `None` when it cannot be typed.
fn required_jamo(text: &str) -> Option<HashSet<char>> {
    let mut jamo: HashSet<char> = indexed_jamo(text)
        .ok()?
        .into_iter()
        .map(|(_, c)| c)
        .collect();
    for (_, medial, fin) in text.chars().filter_map(decompose_syllable) {
        jamo.insert(medial);
        jamo.extend(fin);
    }
    jamo.retain(|&c| is_jamo(c));
    Some(jamo)
}

/// Sorted for stable error messages.
fn sorted(jamo: impl IntoIterator<Item = char>) -> String {
    let mut jamo: Vec<char> = jamo.into_iter().collect();
    jamo.sort_unstable();
    jamo.into_iter().collect()
}

/// `text` is typed on the keyboard and displayed back exactly as written: single spaces, and no
/// lone jamo that the IME would merge into a syllable (ㄱㅏ would show as 가).
fn types_back(text: &str) -> bool {
    let Ok(jamo) = indexed_jamo(text) else {
        return false;
    };
    let keys: Vec<char> = jamo.into_iter().map(|(_, c)| c).collect();
    !text.is_empty() && text.trim() == text && !text.contains("  ") && compose(&keys) == text
}

impl Primer {
    /// Checks the primer contract (see `docs/content.md`, "Hangul primer").
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        let mut err =
            |location: String, message: String| errors.push(ValidationError { location, message });
        if self.lessons.is_empty() {
            err("hangul".into(), "the primer has no lessons".into());
        }
        let mut ids = HashSet::new();
        // Jamo introduced by the lessons so far, this one included.
        let mut known: HashSet<char> = HashSet::new();
        for (index, lesson) in self.lessons.iter().enumerate() {
            let here = format!("hangul/{}", lesson.id);
            if !is_kebab(&lesson.id) {
                err(here.clone(), "lesson id must be kebab-case".into());
            }
            if !ids.insert(lesson.id.as_str()) {
                err(here.clone(), "duplicate lesson id".into());
            }
            if lesson.title.trim().is_empty() {
                err(here.clone(), "lesson has no title".into());
            }
            if lesson.jamo.is_empty() {
                err(here.clone(), "lesson introduces no jamo".into());
            }
            if lesson.lines.is_empty() {
                err(here.clone(), "lesson has no lines".into());
            }
            for new in &lesson.jamo {
                if !is_jamo(new.jamo) {
                    err(here.clone(), format!("{:?} is not a Hangul jamo", new.jamo));
                } else if !known.insert(new.jamo) {
                    err(here.clone(), format!("{} is introduced twice", new.jamo));
                }
            }
            for new in lesson.jamo.iter().filter(|new| is_jamo(new.jamo)) {
                let at = format!("{here} {}", new.jamo);
                if !is_hangul_text(&new.name) {
                    err(at.clone(), format!("name {:?} must be Hangul", new.name));
                }
                if new.example.english.trim().is_empty() {
                    err(at.clone(), "example has no English".into());
                }
                let Some(example) = required_jamo(&new.example.korean)
                    .filter(|_| is_hangul_text(&new.example.korean))
                else {
                    err(
                        at,
                        format!("example {:?} is not Hangul", new.example.korean),
                    );
                    continue;
                };
                if !example.contains(&new.jamo) {
                    err(
                        at.clone(),
                        format!("example {:?} does not contain it", new.example.korean),
                    );
                }
                // The first lesson's six vowels and ㅇ make too few words (none with ㅓ or ㅡ), so
                // its examples may borrow later jamo. They are heard and read, never typed.
                let unknown: Vec<char> = example.difference(&known).copied().collect();
                if index > 0 && !unknown.is_empty() {
                    err(
                        at,
                        format!(
                            "example {:?} uses {} before it is introduced",
                            new.example.korean,
                            sorted(unknown)
                        ),
                    );
                }
            }
            let mut drilled: HashSet<char> = HashSet::new();
            for (n, line) in lesson.lines.iter().enumerate() {
                let at = format!("{here}/{}", n + 1);
                if line.english.as_ref().is_some_and(|e| e.trim().is_empty()) {
                    err(at.clone(), "english is empty (omit it instead)".into());
                }
                let Some(jamo) = required_jamo(&line.korean).filter(|_| types_back(&line.korean))
                else {
                    err(at, format!("{:?} cannot be typed as written", line.korean));
                    continue;
                };
                let unknown: Vec<char> = jamo.difference(&known).copied().collect();
                if !unknown.is_empty() {
                    err(
                        at,
                        format!(
                            "{:?} uses {} before it is introduced",
                            line.korean,
                            sorted(unknown)
                        ),
                    );
                }
                drilled.extend(jamo);
            }
            let untyped: Vec<char> = lesson
                .jamo
                .iter()
                .map(|new| new.jamo)
                .filter(|&c| is_jamo(c) && !drilled.contains(&c))
                .collect();
            if !untyped.is_empty() {
                err(
                    here,
                    format!("{} introduced but never typed", sorted(untyped)),
                );
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// The Hangul name of an introduced jamo.
    pub fn name(&self, jamo: char) -> Option<&str> {
        self.lessons
            .iter()
            .flat_map(|lesson| &lesson.jamo)
            .find(|new| new.jamo == jamo)
            .map(|new| new.name.as_str())
    }

    /// How `text` is built, for lines without English: `ㄱ + ㅏ = 가 · ㄱ + ㅏ + ㄴ = 간`; a lone
    /// jamo shows its name: `ㄱ (기역)`.
    pub fn assembly(&self, text: &str) -> String {
        let parts: Vec<String> = text
            .chars()
            .filter_map(|c| match decompose_syllable(c) {
                Some((initial, medial, Some(fin))) => {
                    Some(format!("{initial} + {medial} + {fin} = {c}"))
                }
                Some((initial, medial, None)) => Some(format!("{initial} + {medial} = {c}")),
                None if is_jamo(c) => Some(match self.name(c) {
                    Some(name) => format!("{c} ({name})"),
                    None => c.to_string(),
                }),
                None => None,
            })
            .collect();
        parts.join(" · ")
    }

    /// The lesson's lines as drill targets (`hangul/<lesson>/<n>`), hinted by their English or,
    /// without one, by their assembly.
    pub fn targets(&self, lesson: &Lesson) -> Vec<Target> {
        lesson
            .lines
            .iter()
            .enumerate()
            .map(|(n, line)| Target {
                item_id: format!("hangul/{}/{}", lesson.id, n + 1),
                korean: line.korean.clone(),
                english: line
                    .english
                    .clone()
                    .unwrap_or_else(|| self.assembly(&line.korean)),
            })
            .collect()
    }

    /// The first lesson is always open; the next one opens once the previous is passed. A passed
    /// lesson stays open (a lesson inserted before it by a later release does not lock it).
    pub fn is_unlocked(&self, index: usize, passed: &HashSet<String>) -> bool {
        let is_passed = |i: usize| self.lessons.get(i).is_some_and(|l| passed.contains(&l.id));
        index < self.lessons.len() && (index == 0 || is_passed(index) || is_passed(index - 1))
    }
}

/// A lesson is passed when its whole round is typed with at least [`PASS_PERCENT`] accuracy.
pub fn round_passes(round: &Round) -> bool {
    let (correct, errors) = (round.keystrokes(), round.errors());
    round.is_over() && correct * 100 >= (correct + errors) * PASS_PERCENT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typing::round::tests::{target, type_line};

    fn new(jamo: char, name: &str, example: &str) -> NewJamo {
        NewJamo {
            jamo,
            name: name.into(),
            example: Line {
                korean: example.into(),
                english: "e".into(),
            },
        }
    }

    fn lesson(id: &str, jamo: Vec<NewJamo>, lines: &[&str]) -> Lesson {
        Lesson {
            id: id.into(),
            title: id.into(),
            jamo,
            lines: lines
                .iter()
                .map(|korean| PrimerLine {
                    korean: (*korean).into(),
                    english: None,
                })
                .collect(),
        }
    }

    /// A valid two-lesson primer: vowels with ㅇ, then ㄱ and ㄴ.
    fn fixture() -> Primer {
        Primer {
            lessons: vec![
                lesson(
                    "vowels",
                    vec![
                        new('ㅇ', "이응", "아이"),
                        new('ㅏ', "아", "아이"),
                        new('ㅣ', "이", "이"),
                    ],
                    &["ㅏ ㅣ", "아이"],
                ),
                lesson(
                    "g-n",
                    vec![new('ㄱ', "기역", "가"), new('ㄴ', "니은", "나이")],
                    &["ㄱ ㄴ", "가 간", "나이"],
                ),
            ],
        }
    }

    fn messages(primer: &Primer) -> Vec<String> {
        primer
            .validate()
            .err()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn bundled_primer_is_valid() {
        let report = messages(&bundled_primer());
        assert!(report.is_empty(), "invalid primer:\n{}", report.join("\n"));
    }

    #[test]
    fn fixture_is_valid() {
        assert_eq!(messages(&fixture()), Vec::<String>::new());
    }

    #[test]
    fn lines_use_only_jamo_introduced_so_far() {
        let mut primer = fixture();
        // ㄱ comes in lesson 2; 가 in lesson 1 is too early.
        primer.lessons[0].lines.push(PrimerLine {
            korean: "가".into(),
            english: None,
        });
        // 과 needs the compound ㅘ itself, not only ㅗ and ㅏ.
        primer.lessons[1].jamo.push(new('ㅗ', "오", "오이"));
        primer.lessons[1].lines.push(PrimerLine {
            korean: "오 과".into(),
            english: None,
        });
        assert_eq!(
            messages(&primer),
            [
                "hangul/vowels/3: \"가\" uses ㄱ before it is introduced",
                "hangul/g-n/4: \"오 과\" uses ㅘ before it is introduced",
            ]
        );
    }

    #[test]
    fn compound_finals_count_as_introduced_jamo() {
        let mut primer = fixture();
        primer.lessons[1].lines.push(PrimerLine {
            korean: "앉".into(),
            english: None,
        });
        assert_eq!(
            messages(&primer),
            ["hangul/g-n/4: \"앉\" uses ㄵㅈ before it is introduced"]
        );
    }

    #[test]
    fn every_jamo_is_introduced_once_and_typed_in_its_lesson() {
        let mut primer = fixture();
        primer.lessons[1].jamo.push(new('ㅏ', "아", "가"));
        primer.lessons[1].jamo.push(new('a', "아", "가"));
        primer.lessons[0].jamo.push(new('ㅡ', "으", "그"));
        assert_eq!(
            messages(&primer),
            [
                "hangul/vowels: ㅡ introduced but never typed",
                "hangul/g-n: ㅏ is introduced twice",
                "hangul/g-n: 'a' is not a Hangul jamo",
            ]
        );
    }

    #[test]
    fn lesson_ids_are_unique_kebab_case() {
        let mut primer = fixture();
        primer.lessons[1].id = "vowels".into();
        primer.lessons[0].title = " ".into();
        primer.lessons.push(lesson("Third", vec![], &[]));
        assert_eq!(
            messages(&primer),
            [
                "hangul/vowels: lesson has no title",
                "hangul/vowels: duplicate lesson id",
                "hangul/Third: lesson id must be kebab-case",
                "hangul/Third: lesson introduces no jamo",
                "hangul/Third: lesson has no lines",
            ]
        );
    }

    #[test]
    fn lines_must_type_back_as_written() {
        let mut primer = fixture();
        for korean in ["ㄱㅏ", "가  나", "가!a", " 가", ""] {
            primer.lessons[1].lines.push(PrimerLine {
                korean: korean.into(),
                english: Some(" ".into()),
            });
        }
        let report = messages(&primer);
        assert_eq!(report.len(), 10, "{report:#?}");
        assert!(report.contains(&"hangul/g-n/4: \"ㄱㅏ\" cannot be typed as written".to_owned()));
        assert!(report.contains(&"hangul/g-n/4: english is empty (omit it instead)".to_owned()));
    }

    #[test]
    fn examples_contain_their_jamo_and_use_known_jamo_after_the_first_lesson() {
        let mut primer = fixture();
        // Lesson 1 may borrow later jamo (ㅁ in 이마).
        primer.lessons[0].jamo[2].example.korean = "이마".into();
        primer.lessons[1].jamo[0].example.korean = "나이".into();
        primer.lessons[1].jamo[1].example.korean = "눈".into();
        primer.lessons[1].jamo[1].name = "nieun".into();
        assert_eq!(
            messages(&primer),
            [
                "hangul/g-n ㄱ: example \"나이\" does not contain it",
                "hangul/g-n ㄴ: name \"nieun\" must be Hangul",
                "hangul/g-n ㄴ: example \"눈\" uses ㅜ before it is introduced",
            ]
        );
    }

    #[test]
    fn lines_without_english_are_hinted_by_their_assembly() {
        let primer = fixture();
        let targets = primer.targets(&primer.lessons[1]);
        let hints: Vec<(&str, &str)> = targets
            .iter()
            .map(|t| (t.item_id.as_str(), t.english.as_str()))
            .collect();
        assert_eq!(
            hints,
            [
                ("hangul/g-n/1", "ㄱ (기역) · ㄴ (니은)"),
                ("hangul/g-n/2", "ㄱ + ㅏ = 가 · ㄱ + ㅏ + ㄴ = 간"),
                ("hangul/g-n/3", "ㄴ + ㅏ = 나 · ㅇ + ㅣ = 이"),
            ]
        );
        // Compounds are shown whole; authored English wins over the assembly.
        assert_eq!(primer.assembly("왜 닭"), "ㅇ + ㅙ = 왜 · ㄷ + ㅏ + ㄺ = 닭");
        let mut with_english = primer.lessons[1].clone();
        with_english.lines[2].english = Some("age".into());
        assert_eq!(primer.targets(&with_english)[2].english, "age");
    }

    #[test]
    fn next_lesson_unlocks_once_the_previous_is_passed() {
        let mut primer = fixture();
        primer.lessons.push(lesson("third", vec![], &[]));
        let passed = |ids: &[&str]| ids.iter().map(|&s| s.to_owned()).collect::<HashSet<_>>();
        let open = |set: &HashSet<String>| -> Vec<bool> {
            (0..4).map(|i| primer.is_unlocked(i, set)).collect()
        };
        assert_eq!(open(&passed(&[])), [true, false, false, false]);
        assert_eq!(open(&passed(&["vowels"])), [true, true, false, false]);
        assert_eq!(open(&passed(&["vowels", "g-n"])), [true, true, true, false]);
        // A passed lesson stays open even when the one before it is not passed.
        assert_eq!(open(&passed(&["g-n"])), [true, true, true, false]);
    }

    #[test]
    fn a_full_round_passes_from_ninety_percent_accuracy() {
        // 9 keystrokes: 5 jamo and 4 spaces.
        let round_with = |mistakes: usize| {
            let mut round = Round::new(vec![target("ㄱ ㄴ ㄷ ㄹ ㅁ")]).unwrap().unwrap();
            type_line(&mut round, mistakes);
            round
        };
        assert!(round_passes(&round_with(0)));
        assert!(round_passes(&round_with(1)), "9 / 10 = 90 %");
        assert!(!round_passes(&round_with(2)), "9 / 11 < 90 %");

        let mut unfinished = Round::new(vec![target("가"), target("나")])
            .unwrap()
            .unwrap();
        type_line(&mut unfinished, 0);
        assert!(!round_passes(&unfinished), "the whole round must be typed");
    }
}
