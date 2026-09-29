//! Legacy Anki deck → learn-korean content, in two explicit steps:
//!
//! 1. [`import_csv`]: CSV → [`RawEntry`] rows, byte-for-byte what the deck contains (immutable).
//! 2. [`build`]: raw rows + hand-written [`Curation`] → normalized [`Lexeme`]s and the `legacy`
//!    [`Pack`]. Every raw row must be curated; rows with detected problems must carry a `fix` or
//!    a `skip`, so bad source data can never reach the app silently.

use std::collections::{BTreeMap, HashSet};

use korean_core::content::{
    Item, ItemKind, Lexeme, Pack, PartOfSpeech, RawEntry, Register, Replies, is_hangul_text,
    validate_packs,
};
use serde::{Deserialize, Serialize};

pub const SOURCE: &str = "anki-important-words";

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("csv: {0}")]
    Csv(#[from] csv::Error),
    #[error("{0} problem(s):\n{1}")]
    Invalid(usize, String),
}

fn invalid(problems: Vec<String>) -> ImportError {
    ImportError::Invalid(problems.len(), problems.join("\n"))
}

/// Parses the two-column export (`front,back`). Row numbers are 1-based record numbers (equal to
/// line numbers in the export, which has no multi-line fields).
pub fn import_csv(csv_text: &str) -> Result<Vec<RawEntry>, ImportError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(csv_text.as_bytes());
    let mut rows = Vec::new();
    for (index, record) in reader.records().enumerate() {
        let record = record?;
        rows.push(RawEntry {
            source: SOURCE.into(),
            row: index as u32 + 1,
            front_raw: record.get(0).unwrap_or_default().to_string(),
            back_raw: record.get(1).unwrap_or_default().to_string(),
        });
    }
    Ok(rows)
}

/// Problems detected in a raw row. Rows with a blocking issue need a curated `fix` or `skip`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issue {
    Empty,
    /// The Korean side contains no Hangul at all (e.g. a repeated header row).
    NoHangul,
    /// Letters from another script where Korean is expected (e.g. Arabic).
    ForeignScript,
    /// `...` / `X` placeholders instead of a real word.
    Template,
    /// Several alternatives separated by `/`.
    Variants,
}

impl Issue {
    pub fn is_blocking(self) -> bool {
        !matches!(self, Issue::Variants)
    }
}

/// The Korean part of the back field, without the trailing `(romanization)`.
fn korean_part(back: &str) -> &str {
    back.split(" (").next().unwrap_or(back).trim()
}

fn is_latin(c: char) -> bool {
    c.is_ascii() || ('\u{00C0}'..='\u{024F}').contains(&c)
}

pub fn issues(raw: &RawEntry) -> Vec<Issue> {
    if raw.front_raw.trim().is_empty() && raw.back_raw.trim().is_empty() {
        return vec![Issue::Empty];
    }
    let korean = korean_part(&raw.back_raw);
    let is_hangul =
        |c: char| ('\u{AC00}'..='\u{D7A3}').contains(&c) || ('\u{3131}'..='\u{318E}').contains(&c);
    let mut found = Vec::new();
    if !korean.chars().any(is_hangul) {
        found.push(Issue::NoHangul);
    }
    if korean
        .chars()
        .any(|c| c.is_alphabetic() && !is_hangul(c) && !is_latin(c))
    {
        found.push(Issue::ForeignScript);
    }
    if korean.contains("...") || korean.split_whitespace().any(|w| w == "X") {
        found.push(Issue::Template);
    }
    if raw.back_raw.contains(" / ") {
        found.push(Issue::Variants);
    }
    found
}

#[derive(Debug, Clone, Deserialize)]
pub struct Curation {
    pub source: String,
    pub entries: Vec<CurationEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurationEntry {
    pub row: u32,
    #[serde(default)]
    pub skip: Option<String>,
    #[serde(default)]
    pub lemma: Option<String>,
    #[serde(default)]
    pub surface: Option<String>,
    #[serde(default)]
    pub english: Option<String>,
    #[serde(default)]
    pub pos: Option<PartOfSpeech>,
    #[serde(default)]
    pub register: Option<Register>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub fix: Option<String>,
    #[serde(default)]
    pub usage: Vec<Usage>,
    #[serde(default)]
    pub extra: Vec<ExtraLexeme>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtraLexeme {
    pub lemma: String,
    pub surface: String,
    pub english: String,
    pub pos: PartOfSpeech,
    pub register: Register,
    #[serde(default)]
    pub image: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Usage {
    pub id: String,
    pub korean: String,
    pub english: String,
    pub context: String,
    pub register: Register,
    #[serde(default)]
    pub note: Option<String>,
    pub distractors: Vec<String>,
    #[serde(default)]
    pub replies: Option<Replies>,
    #[serde(default)]
    pub chunks: Option<Vec<String>>,
    #[serde(default)]
    pub image: Option<String>,
    /// The other content words of the sentence (dictionary forms), after the head lemma.
    #[serde(default)]
    pub lexemes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LexemeFile {
    pub source: String,
    pub lexemes: Vec<Lexeme>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Built {
    pub lexemes: LexemeFile,
    pub pack: Pack,
}

/// Parts of speech taught through usage sentences only: a bare ending or particle is not a
/// meaningful vocabulary card.
fn has_word_card(pos: PartOfSpeech) -> bool {
    !matches!(pos, PartOfSpeech::Grammar | PartOfSpeech::Particle)
}

/// Item `lexemes` only hold Hangul lemmas; patterns like `-고 싶다` are left out.
fn lemma_list(lemma: &str) -> Vec<String> {
    if is_hangul_text(lemma) {
        vec![lemma.to_string()]
    } else {
        Vec::new()
    }
}

/// Sentence lexemes: the head lemma (when Hangul) followed by the curated extras, deduplicated.
fn sentence_lexemes(head: &str, extras: &[String]) -> Vec<String> {
    let mut lexemes = lemma_list(head);
    for extra in extras {
        if !lexemes.contains(extra) {
            lexemes.push(extra.clone());
        }
    }
    lexemes
}

struct Normalized {
    id: String,
    item_id: String,
    lemma: String,
    surface: String,
    english: String,
    pos: PartOfSpeech,
    register: Register,
    image: Option<String>,
    fix: Option<String>,
    row: u32,
}

pub fn build(raw: &[RawEntry], curation: &Curation) -> Result<Built, ImportError> {
    let mut problems = Vec::new();
    if curation.source != SOURCE {
        problems.push(format!(
            "curation source is {:?}, expected {SOURCE:?}",
            curation.source
        ));
    }
    let mut curated: BTreeMap<u32, &CurationEntry> = BTreeMap::new();
    for entry in &curation.entries {
        if curated.insert(entry.row, entry).is_some() {
            problems.push(format!("row {}: curated twice", entry.row));
        }
    }
    let raw_rows: HashSet<u32> = raw.iter().map(|r| r.row).collect();
    for row in curated.keys() {
        if !raw_rows.contains(row) {
            problems.push(format!("row {row}: curated but absent from the raw import"));
        }
    }

    let mut normalized = Vec::new();
    let mut usages = Vec::new();
    for raw_entry in raw {
        let row = raw_entry.row;
        let Some(entry) = curated.get(&row) else {
            problems.push(format!("row {row}: not curated ({:?})", raw_entry.back_raw));
            continue;
        };
        let blocking: Vec<Issue> = issues(raw_entry)
            .into_iter()
            .filter(|i| i.is_blocking())
            .collect();
        if entry.skip.is_some() {
            continue;
        }
        if !blocking.is_empty() && entry.fix.is_none() {
            problems.push(format!(
                "row {row}: {blocking:?} in {:?} needs a `fix` or `skip`",
                raw_entry.back_raw
            ));
        }
        let (Some(lemma), Some(surface), Some(english), Some(pos), Some(register)) = (
            &entry.lemma,
            &entry.surface,
            &entry.english,
            entry.pos,
            entry.register,
        ) else {
            problems.push(format!(
                "row {row}: lemma, surface, english, pos and register are required"
            ));
            continue;
        };
        normalized.push(Normalized {
            id: format!("{SOURCE}/{row}"),
            item_id: format!("legacy/{row}"),
            lemma: lemma.clone(),
            surface: surface.clone(),
            english: english.clone(),
            pos,
            register,
            image: entry.image.clone(),
            fix: entry.fix.clone(),
            row,
        });
        for (n, extra) in entry.extra.iter().enumerate() {
            normalized.push(Normalized {
                id: format!("{SOURCE}/{row}-{}", n + 2),
                item_id: format!("legacy/{row}-{}", n + 2),
                lemma: extra.lemma.clone(),
                surface: extra.surface.clone(),
                english: extra.english.clone(),
                pos: extra.pos,
                register: extra.register,
                image: extra.image.clone(),
                fix: None,
                row,
            });
        }
        usages.extend(entry.usage.iter().map(|u| (row, u, lemma.clone())));
    }

    for n in &normalized {
        if !is_hangul_text(&n.surface) {
            problems.push(format!(
                "row {}: surface {:?} is not Hangul text",
                n.row, n.surface
            ));
        }
    }

    let lexemes = normalized
        .iter()
        .map(|n| Lexeme {
            id: n.id.clone(),
            lemma: n.lemma.clone(),
            surface: n.surface.clone(),
            english: n.english.clone(),
            pos: n.pos,
            register: n.register,
            raw_row: Some(n.row),
            fix: n.fix.clone(),
        })
        .collect();

    let words = normalized.iter().filter(|n| has_word_card(n.pos)).map(|n| {
        let item = Item {
            id: n.item_id.clone(),
            kind: ItemKind::Word,
            korean: n.surface.clone(),
            english: n.english.clone(),
            context: "vocabulary".into(),
            register: n.register,
            note: None,
            lexemes: lemma_list(&n.lemma),
            distractors: vec![],
            replies: None,
            chunks: None,
            image: n.image.clone(),
        };
        (n.row, item)
    });
    let sentences = usages.into_iter().map(|(row, u, lemma)| {
        let item = Item {
            id: u.id.clone(),
            kind: ItemKind::Sentence,
            korean: u.korean.clone(),
            english: u.english.clone(),
            context: u.context.clone(),
            register: u.register,
            note: u.note.clone(),
            lexemes: sentence_lexemes(&lemma, &u.lexemes),
            distractors: u.distractors.clone(),
            replies: u.replies.clone(),
            chunks: u.chunks.clone(),
            image: u.image.clone(),
        };
        (row, item)
    });
    // Deck order, each word followed by the sentences that use it (stable sort keeps words first).
    let mut ordered: Vec<(u32, Item)> = words.chain(sentences).collect();
    ordered.sort_by_key(|(row, _)| *row);
    let items: Vec<Item> = ordered.into_iter().map(|(_, item)| item).collect();

    let pack = Pack {
        id: "legacy".into(),
        version: 1,
        title: "Legacy lexicon".into(),
        description: "The 284 words of the original Anki deck, normalized, with usage sentences."
            .into(),
        unlock_level: 1,
        items,
    };
    if let Err(errors) = validate_packs(std::slice::from_ref(&pack)) {
        problems.extend(errors.iter().map(ToString::to_string));
    }
    if !problems.is_empty() {
        return Err(invalid(problems));
    }
    Ok(Built {
        lexemes: LexemeFile {
            source: SOURCE.into(),
            lexemes,
        },
        pack,
    })
}

/// Pretty JSON with a trailing newline, the format of every generated file.
pub fn to_json<T: Serialize>(value: &T) -> String {
    let mut json = serde_json::to_string_pretty(value).expect("serializable");
    json.push('\n');
    json
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(row: u32, front: &str, back: &str) -> RawEntry {
        RawEntry {
            source: SOURCE.into(),
            row,
            front_raw: front.into(),
            back_raw: back.into(),
        }
    }

    #[test]
    fn csv_rows_keep_their_line_numbers_and_quoted_commas() {
        let rows =
            import_csv("Personne,사람 (sa ram)\r\n\"Tu (Rare, souvent évité)\",너 (neo)\r\n,\r\n")
                .unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1].row, 2);
        assert_eq!(rows[1].front_raw, "Tu (Rare, souvent évité)");
        assert_eq!(rows[2].back_raw, "");
    }

    #[test]
    fn detects_the_known_bad_rows() {
        assert_eq!(
            issues(&raw(3, "Traduction Française", "Coréen (Phonétique)")),
            [Issue::NoHangul]
        );
        assert_eq!(issues(&raw(269, "", "")), [Issue::Empty]);
        assert!(issues(&raw(164, "Seulement", "فقط (bak-kke)")).contains(&Issue::ForeignScript));
        assert_eq!(
            issues(&raw(14, "S'il vous plaît", "...주세요 (...ju se yo)")),
            [Issue::Template]
        );
        assert_eq!(
            issues(&raw(
                146,
                "Comment dit-on",
                "X 한국말로 뭐예요? (X han guk)"
            )),
            [Issue::Template]
        );
        assert_eq!(
            issues(&raw(134, "Très", "아주 (a ju) / 너무 (neo mu)")),
            [Issue::Variants]
        );
        assert!(!Issue::Variants.is_blocking());
        assert_eq!(
            issues(&raw(126, "Être fatigué", "피곤해요 (pi gon hae yo)")),
            []
        );
    }

    fn curation(json: &str) -> Curation {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn every_row_must_be_curated_and_bad_rows_fixed() {
        let rows = vec![
            raw(1, "Personne", "사람 (sa ram)"),
            raw(2, "Seulement", "فقط (bak-kke)"),
            raw(3, "Chat", "고양이 (go yang i)"),
        ];
        let c = curation(
            r#"{"source":"anki-important-words","entries":[
              {"row":1,"lemma":"사람","surface":"사람","english":"person","pos":"noun","register":"neutral"},
              {"row":2,"lemma":"만","surface":"만","english":"only","pos":"particle","register":"neutral"}
            ]}"#,
        );
        let err = build(&rows, &c).unwrap_err().to_string();
        assert!(err.contains("row 2: [NoHangul, ForeignScript]"), "{err}");
        assert!(err.contains("row 3: not curated"), "{err}");
    }

    #[test]
    fn builds_lexemes_word_cards_and_usage_sentences() {
        let rows = vec![
            raw(1, "Traduction", "Coréen (Phonétique)"),
            raw(2, "Vélo", "자전거 (ja-jeon-geo)"),
            raw(3, "Et", "...하고 (ha go)"),
        ];
        let c = curation(
            r#"{"source":"anki-important-words","entries":[
              {"row":1,"skip":"header"},
              {"row":2,"lemma":"자전거","surface":"자전거","english":"bicycle","pos":"noun","register":"neutral","image":"bicycle",
               "usage":[{"id":"legacy/bike","korean":"자전거 있어요?","english":"Do you have a bicycle?","context":"transport","register":"polite",
                         "lexemes":["있다","자전거"],
                         "distractors":["Do you have a car?","Is this a bicycle?","Do you like bicycles?"]}]},
              {"row":3,"lemma":"하고","surface":"하고","english":"and (with nouns)","pos":"particle","register":"neutral","fix":"template removed"}
            ]}"#,
        );
        let built = build(&rows, &c).unwrap();
        let lexeme_ids: Vec<_> = built
            .lexemes
            .lexemes
            .iter()
            .map(|l| l.id.as_str())
            .collect();
        assert_eq!(
            lexeme_ids,
            ["anki-important-words/2", "anki-important-words/3"]
        );
        let item_ids: Vec<_> = built.pack.items.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(
            item_ids,
            ["legacy/2", "legacy/bike"],
            "particles get no word card"
        );
        assert_eq!(built.pack.items[0].image.as_deref(), Some("bicycle"));
        assert_eq!(
            built.pack.items[1].lexemes,
            ["자전거", "있다"],
            "head lemma first, deduplicated"
        );
    }

    /// The committed generated files must be exactly what the importer produces.
    #[test]
    fn committed_legacy_content_is_reproducible() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/legacy");
        let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
        let rows = import_csv(&read("source/anki-important-words.csv")).unwrap();
        assert_eq!(to_json(&rows), read("raw.json"), "raw.json is stale");
        let curation: Curation = serde_json::from_str(&read("curation.json")).unwrap();
        let built = build(&rows, &curation).unwrap();
        assert_eq!(
            to_json(&built.lexemes),
            read("lexemes.json"),
            "lexemes.json is stale"
        );
        assert_eq!(
            to_json(&built.pack),
            read("pack.json"),
            "pack.json is stale"
        );
    }
}
