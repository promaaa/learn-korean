//! Keyboard course: touch typing taught a key pair at a time, from the home row out, like
//! typingclub.com. Each stage first drills its new keys alone, then mixes them with every key
//! learned before. Lines are random groups of keys, generated anew for every round.
//!
//! Hangul composes as it is typed (ㄹ then ㅓ shows 러), so a group is either a run of lone
//! consonants or of lone vowels (ㄹㄹ, ㅓㅓㅓ), or real syllables (러, 럴러).

use std::collections::HashSet;

use super::jamo::{combine_medial, compose_syllable, decompose_to_jamo, medial_index};
use super::primer::opens_in_order;
use super::targets::Target;
use crate::rng::Rng;

/// Lines per lesson round.
const LINES: usize = 4;
/// Final consonants of everyday syllables (각 간 갇 갈 감 갑 갓 강 갔).
const COMMON_FINALS: &str = "ㄱㄴㄷㄹㅁㅂㅅㅇㅆ";
/// A line ends once it shows this many characters, spaces included. Groups are at most 5 keys, so
/// a line never passes 28 characters (the drill screen's width).
const LINE_CHARS: usize = 22;
/// Attempts at a line containing every new key before accepting one without.
const LINE_TRIES: usize = 100;

struct Stage {
    /// Kebab-case, permanent once released: lesson ids derive from it.
    id: &'static str,
    title: &'static str,
    /// The keys this stage introduces: jamo (doubles are typed with Shift) or `,` `.`.
    keys: &'static str,
}

const fn stage(id: &'static str, title: &'static str, keys: &'static str) -> Stage {
    Stage { id, title, keys }
}

/// Home row first (index fingers on F and J), then the top row, the bottom row and Shift.
const STAGES: [Stage; 18] = [
    stage("fj", "Home row · index fingers", "ㄹㅓ"),
    stage("dk", "Home row · middle fingers", "ㅇㅏ"),
    stage("sl", "Home row · ring fingers", "ㄴㅣ"),
    stage("gh", "Home row · index reach", "ㅎㅗ"),
    stage("a", "Home row · left pinky", "ㅁ"),
    stage("ru", "Top row · index fingers", "ㄱㅕ"),
    stage("ei", "Top row · middle fingers", "ㄷㅑ"),
    stage("ty", "Top row · index reach", "ㅅㅛ"),
    stage("wo", "Top row · ring fingers", "ㅈㅐ"),
    stage("qp", "Top row · pinkies", "ㅂㅔ"),
    stage("vm", "Bottom row · index fingers", "ㅍㅡ"),
    stage("bn", "Bottom row · index reach", "ㅠㅜ"),
    stage("c-comma", "Bottom row · middle fingers", "ㅊ,"),
    stage("x-period", "Bottom row · ring fingers", "ㅌ."),
    stage("z", "Bottom row · left pinky", "ㅋ"),
    stage("shift-index", "Shift · left index", "ㄲㅆ"),
    stage("shift-left", "Shift · left hand", "ㄸㅉㅃ"),
    stage("shift-right", "Shift · right hand", "ㅒㅖ"),
];

fn is_punctuation(c: char) -> bool {
    matches!(c, ',' | '.')
}

fn is_vowel(c: char) -> bool {
    medial_index(c).is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LessonKind {
    /// The new keys alone, as lone jamo.
    Keys,
    /// The new keys in syllables, borrowing learned vowels or consonants when the stage has none.
    Syllables,
    /// The new keys mixed with every key learned before.
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyLesson {
    /// Kebab-case, permanent once released: passes are stored by lesson id.
    pub id: String,
    pub title: String,
    /// The keys of the lesson's stage.
    pub new: Vec<char>,
    /// Every key learned so far, `new` included.
    pub known: Vec<char>,
    pub kind: LessonKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Course {
    pub lessons: Vec<KeyLesson>,
}

impl Default for Course {
    fn default() -> Self {
        Self::new()
    }
}

impl Course {
    /// The lessons of every stage, in order: its new keys alone as lone jamo (not for a single
    /// key), then in syllables (twice for the first stage, which has nothing to mix with), then
    /// twice mixed with every earlier key. Ids are `<stage>-<n>`.
    pub fn new() -> Course {
        use LessonKind::*;
        let mut lessons = Vec::new();
        let mut known: Vec<char> = Vec::new();
        for (index, stage) in STAGES.iter().enumerate() {
            let new: Vec<char> = stage.keys.chars().collect();
            known.extend(&new);
            let kinds: &[LessonKind] = match (index, new.len()) {
                (0, _) => &[Keys, Syllables, Syllables],
                (_, 1) => &[Syllables, Mixed, Mixed],
                _ => &[Keys, Syllables, Mixed, Mixed],
            };
            for (n, &kind) in kinds.iter().enumerate() {
                let label = match kind {
                    Keys => "new keys",
                    Syllables => "syllables",
                    Mixed => "all keys",
                };
                let same = kinds.iter().filter(|&&k| k == kind).count();
                let nth = kinds[..n].iter().filter(|&&k| k == kind).count() + 1;
                lessons.push(KeyLesson {
                    id: format!("{}-{}", stage.id, n + 1),
                    title: if same == 1 {
                        format!("{} · {label}", stage.title)
                    } else {
                        format!("{} · {label} {nth}", stage.title)
                    },
                    new: new.clone(),
                    known: known.clone(),
                    kind,
                });
            }
        }
        Course { lessons }
    }

    /// The first lesson is always open; the next one opens once the previous is passed.
    pub fn is_unlocked(&self, index: usize, passed: &HashSet<String>) -> bool {
        opens_in_order(&self.lessons, |lesson| &lesson.id, index, passed)
    }
}

impl KeyLesson {
    /// A fresh round of random key groups (`keyboard/<lesson>/<n>`), each line typing every new
    /// key.
    pub fn targets(&self, seed: u64) -> Vec<Target> {
        let mut rng = Rng::new(seed);
        let keys = spaced(&self.new);
        let hint = match self.kind {
            LessonKind::Keys => format!("new keys: {keys}"),
            LessonKind::Syllables => format!("new keys in syllables: {keys}"),
            LessonKind::Mixed => format!("{keys} among every key so far"),
        };
        (1..=LINES)
            .map(|n| Target {
                item_id: format!("keyboard/{}/{n}", self.id),
                korean: self.line(&mut rng),
                english: hint.clone(),
            })
            .collect()
    }

    fn line(&self, rng: &mut Rng) -> String {
        let pools = Pools::of(self);
        let mut line = String::new();
        for _ in 0..LINE_TRIES {
            line = pools.line(rng);
            let typed = decompose_to_jamo(&line).unwrap_or_default();
            if self.new.iter().all(|key| typed.contains(key)) {
                break;
            }
        }
        line
    }
}

/// What a lesson's groups are drawn from.
struct Pools {
    kind: LessonKind,
    new_consonants: Vec<char>,
    new_vowels: Vec<char>,
    consonants: Vec<char>,
    vowels: Vec<char>,
    marks: Vec<char>,
}

impl Pools {
    fn of(lesson: &KeyLesson) -> Pools {
        let pick = |keys: &[char], keep: fn(char) -> bool| -> Vec<char> {
            keys.iter().copied().filter(|&c| keep(c)).collect()
        };
        let is_consonant = |c| !is_vowel(c) && !is_punctuation(c);
        let (new_consonants, new_vowels) =
            (pick(&lesson.new, is_consonant), pick(&lesson.new, is_vowel));
        let (mut consonants, mut vowels) = (
            pick(&lesson.known, is_consonant),
            pick(&lesson.known, is_vowel),
        );
        // Alone, the new keys are the only ones, but a syllable needs both halves.
        if lesson.kind != LessonKind::Mixed {
            if !new_consonants.is_empty() {
                consonants.clone_from(&new_consonants);
            }
            if !new_vowels.is_empty() {
                vowels.clone_from(&new_vowels);
            }
        }
        let marks = pick(
            if lesson.kind == LessonKind::Mixed {
                &lesson.known
            } else {
                &lesson.new
            },
            is_punctuation,
        );
        Pools {
            kind: lesson.kind,
            new_consonants,
            new_vowels,
            consonants,
            vowels,
            marks,
        }
    }

    fn line(&self, rng: &mut Rng) -> String {
        // Out of four groups, how many are lone jamo rather than syllables.
        let lone_quarters = match self.kind {
            LessonKind::Keys => 4,
            LessonKind::Syllables | LessonKind::Mixed => 1,
        };
        // Punctuation ends a group: every other one while it is new, now and then later.
        let mark_odds = if self.kind == LessonKind::Mixed { 5 } else { 2 };
        let mut groups: Vec<String> = Vec::new();
        let mut chars = 0;
        while chars < LINE_CHARS {
            let mut group = if rng.below(4) < lone_quarters {
                self.lone(rng)
            } else {
                self.syllables(rng)
            };
            if !self.marks.is_empty() && rng.below(mark_odds) == 0 {
                group.push(self.marks[rng.below(self.marks.len())]);
            }
            chars += group.chars().count() + usize::from(!groups.is_empty());
            groups.push(group);
        }
        groups.join(" ")
    }

    /// A key from `known`; mixed lessons draw half their keys from `new` (when it has any).
    fn key(&self, rng: &mut Rng, new: &[char], known: &[char]) -> char {
        let pool = if self.kind == LessonKind::Mixed && !new.is_empty() && rng.below(2) == 0 {
            new
        } else {
            known
        };
        pool[rng.below(pool.len())]
    }

    fn consonant(&self, rng: &mut Rng) -> char {
        self.key(rng, &self.new_consonants, &self.consonants)
    }

    fn vowel(&self, rng: &mut Rng) -> char {
        self.key(rng, &self.new_vowels, &self.vowels)
    }

    /// One to four lone consonants, or lone vowels that do not combine (ㅗ then ㅏ would show ㅘ).
    fn lone(&self, rng: &mut Rng) -> String {
        let len = 1 + rng.below(4);
        // Until mixed, lone groups show only the new keys, so only their kind of jamo.
        let alone = self.kind != LessonKind::Mixed;
        let vowels_only = alone && self.new_consonants.is_empty();
        let consonants_only = alone && self.new_vowels.is_empty();
        let vowels = !consonants_only && (vowels_only || rng.below(2) == 0);
        let mut group = String::new();
        let mut last: Option<char> = None;
        for _ in 0..len {
            let key = if vowels {
                let mut vowel = self.vowel(rng);
                if last.is_some_and(|prev| combine_medial(prev, vowel).is_some()) {
                    vowel = last.expect("checked");
                }
                vowel
            } else {
                self.consonant(rng)
            };
            last = Some(key);
            group.push(key);
        }
        group
    }

    /// One or two syllables, some with a final consonant (only the common ones: 뇿 is no word).
    fn syllables(&self, rng: &mut Rng) -> String {
        (0..1 + rng.below(2))
            .map(|_| {
                let (initial, medial) = (self.consonant(rng), self.vowel(rng));
                let fin = Some(self.consonant(rng))
                    .filter(|&c| COMMON_FINALS.contains(c) && rng.below(3) == 0);
                compose_syllable(initial, medial, fin).expect("keyboard jamo form a syllable")
            })
            .collect()
    }
}

fn spaced(keys: &[char]) -> String {
    keys.iter()
        .map(char::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typing::jamo::decompose_syllable;
    use crate::typing::{compose, keystroke_for_jamo};

    #[test]
    fn stages_teach_every_key_once_starting_from_f_and_j() {
        let mut seen = HashSet::new();
        for stage in &STAGES {
            for key in stage.keys.chars() {
                assert!(keystroke_for_jamo(key).is_some(), "{key} is not a key");
                assert!(seen.insert(key), "{key} is taught twice");
            }
        }
        // The 33 keyboard jamo plus , and .
        assert_eq!(seen.len(), 35);
        let first = keystroke_for_jamo('ㄹ').zip(keystroke_for_jamo('ㅓ'));
        assert_eq!(first.map(|(a, b)| (a.code, b.code)), Some(("KeyF", "KeyJ")));
    }

    #[test]
    fn each_stage_drills_its_keys_alone_then_mixed() {
        use LessonKind::*;
        let course = Course::new();
        let plan = |stage: &str| -> Vec<(&str, LessonKind)> {
            course
                .lessons
                .iter()
                .filter(|l| l.id.rsplit_once('-').is_some_and(|(s, _)| s == stage))
                .map(|l| (l.id.as_str(), l.kind))
                .collect()
        };
        assert_eq!(
            plan("fj"),
            [("fj-1", Keys), ("fj-2", Syllables), ("fj-3", Syllables)]
        );
        assert_eq!(
            plan("dk"),
            [
                ("dk-1", Keys),
                ("dk-2", Syllables),
                ("dk-3", Mixed),
                ("dk-4", Mixed)
            ]
        );
        // A single key has no pair to alternate with as lone jamo.
        assert_eq!(
            plan("a"),
            [("a-1", Syllables), ("a-2", Mixed), ("a-3", Mixed)]
        );
        assert_eq!(
            course.lessons[1].title,
            "Home row · index fingers · syllables 1"
        );
        assert_eq!(
            course.lessons[3].title,
            "Home row · middle fingers · new keys"
        );

        let ids: HashSet<&str> = course.lessons.iter().map(|l| l.id.as_str()).collect();
        assert_eq!(ids.len(), course.lessons.len());
        assert!(ids.iter().all(|id| crate::content::is_kebab(id)));
        let dk = &course.lessons[3];
        assert_eq!(dk.new, ['ㅇ', 'ㅏ']);
        assert_eq!(dk.known, ['ㄹ', 'ㅓ', 'ㅇ', 'ㅏ']);
    }

    /// The keys a lesson's round makes the learner press, line by line.
    fn pressed(lesson: &KeyLesson, seed: u64) -> Vec<Vec<char>> {
        lesson
            .targets(seed)
            .iter()
            .map(|t| decompose_to_jamo(&t.korean).unwrap())
            .collect()
    }

    fn lessons(kind: LessonKind) -> impl Iterator<Item = KeyLesson> {
        Course::new()
            .lessons
            .into_iter()
            .filter(move |l| l.kind == kind)
    }

    #[test]
    fn every_line_before_mixing_types_every_new_key() {
        for lesson in lessons(LessonKind::Keys).chain(lessons(LessonKind::Syllables)) {
            for seed in 0..20 {
                for keys in pressed(&lesson, seed) {
                    for key in &lesson.new {
                        assert!(keys.contains(key), "{}: no {key}", lesson.id);
                    }
                }
            }
        }
    }

    #[test]
    fn new_keys_lessons_show_only_the_new_keys_as_lone_jamo() {
        for lesson in lessons(LessonKind::Keys) {
            for seed in 0..20 {
                for target in lesson.targets(seed) {
                    for c in target.korean.chars().filter(|&c| c != ' ') {
                        assert!(lesson.new.contains(&c), "{}: {}", lesson.id, target.korean);
                    }
                }
            }
        }
    }

    #[test]
    fn syllable_lessons_borrow_only_the_kind_of_jamo_the_stage_lacks() {
        for lesson in lessons(LessonKind::Syllables) {
            let has_vowel = lesson.new.iter().any(|&c| is_vowel(c));
            let has_consonant = lesson
                .new
                .iter()
                .any(|&c| !is_vowel(c) && !is_punctuation(c));
            let mut syllables = 0;
            for seed in 0..20 {
                for target in lesson.targets(seed) {
                    syllables += target
                        .korean
                        .chars()
                        .filter(|&c| decompose_syllable(c).is_some())
                        .count();
                    for key in decompose_to_jamo(&target.korean).unwrap() {
                        let borrowed = if is_vowel(key) {
                            !has_vowel
                        } else {
                            !has_consonant
                        };
                        assert!(
                            key == ' ' || lesson.new.contains(&key) || borrowed,
                            "{}: {key} in {}",
                            lesson.id,
                            target.korean
                        );
                    }
                }
            }
            assert!(syllables > 0, "{}", lesson.id);
        }
    }

    #[test]
    fn mixed_lessons_draw_from_known_keys_and_favour_the_new_ones() {
        let course = Course::new();
        let lesson = course.lessons.iter().find(|l| l.id == "ru-3").unwrap();
        assert_eq!(lesson.kind, LessonKind::Mixed);
        let (mut new, mut total) = (0, 0);
        let mut used = HashSet::new();
        for seed in 0..50 {
            for key in pressed(lesson, seed)
                .into_iter()
                .flatten()
                .filter(|&k| k != ' ')
            {
                assert!(lesson.known.contains(&key), "{key} is not learned yet");
                used.insert(key);
                total += 1;
                new += usize::from(lesson.new.contains(&key));
            }
        }
        assert_eq!(used.len(), lesson.known.len(), "every learned key comes up");
        // Half the draws are new keys, plus their share of the other half.
        assert!(new * 100 / total > 50, "{new} / {total}");
    }

    #[test]
    fn rounds_fit_the_screen_type_back_and_change_with_the_seed() {
        let course = Course::new();
        for lesson in &course.lessons {
            for seed in 0..20 {
                let round = lesson.targets(seed);
                assert_eq!(round.len(), LINES);
                for (n, target) in round.iter().enumerate() {
                    assert_eq!(target.item_id, format!("keyboard/{}/{}", lesson.id, n + 1));
                    assert!(target.korean.chars().count() <= 28, "{}", target.korean);
                    let jamo = decompose_to_jamo(&target.korean).unwrap();
                    assert_eq!(compose(&jamo), target.korean, "types back as shown");
                    // Punctuation only ends a group.
                    for word in target.korean.split(' ') {
                        let chars: Vec<char> = word.chars().collect();
                        let inner = &chars[..chars.len() - 1];
                        assert!(!inner.iter().copied().any(is_punctuation), "{word}");
                    }
                }
            }
        }
        let first = &course.lessons[0];
        assert_ne!(first.targets(1), first.targets(2));
        assert_eq!(first.targets(3), first.targets(3));
    }

    #[test]
    fn lessons_open_one_after_the_other() {
        let course = Course::new();
        let passed: HashSet<String> = ["fj-1".to_owned()].into();
        let open: Vec<bool> = (0..3).map(|i| course.is_unlocked(i, &passed)).collect();
        assert_eq!(open, [true, true, false]);
        assert!(!course.is_unlocked(course.lessons.len(), &passed));
    }
}
