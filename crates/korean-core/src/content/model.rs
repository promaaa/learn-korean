use serde::{Deserialize, Serialize};

/// A row exactly as imported from an external source. Never edited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawEntry {
    pub source: String,
    pub row: u32,
    pub front_raw: String,
    pub back_raw: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PartOfSpeech {
    Noun,
    Pronoun,
    Verb,
    Adjective,
    Adverb,
    Conjunction,
    Interjection,
    Expression,
    Particle,
    Determiner,
    Grammar,
    Number,
}

/// Normalized dictionary entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lexeme {
    pub id: String,
    pub lemma: String,
    pub surface: String,
    pub english: String,
    pub pos: PartOfSpeech,
    pub register: Register,
    /// Raw row this lexeme was normalized from, if imported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_row: Option<u32>,
    /// Explanation when the source data was wrong and has been corrected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Register {
    /// 해요체
    Polite,
    /// 합니다체
    Formal,
    /// 반말
    Casual,
    /// No verb ending (nouns, adverbs, set phrases).
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Sentence,
    Word,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    pub korean: String,
    pub english: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replies {
    pub good: Vec<Line>,
    pub bad: Vec<Line>,
}

/// What a game trains. Each (item, skill) pair has its own memory state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Skill {
    /// Understand Korean: pick the meaning of what you hear/read.
    Listening,
    /// Pick a natural answer to a Korean line.
    Response,
    /// Assemble the Korean sentence from chunks.
    Build,
}

impl Skill {
    pub const ALL: [Skill; 3] = [Skill::Listening, Skill::Response, Skill::Build];

    pub fn as_str(self) -> &'static str {
        match self {
            Skill::Listening => "listening",
            Skill::Response => "response",
            Skill::Build => "build",
        }
    }

    pub fn parse(s: &str) -> Option<Skill> {
        Skill::ALL.into_iter().find(|skill| skill.as_str() == s)
    }
}

/// Chunk counts playable in the build game (chunks are picked with the digit keys 1-9).
pub const BUILD_CHUNKS: std::ops::RangeInclusive<usize> = 3..=9;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub id: String,
    pub kind: ItemKind,
    pub korean: String,
    pub english: String,
    pub context: String,
    pub register: Register,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lexemes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub distractors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replies: Option<Replies>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunks: Option<Vec<String>>,
    /// English photo-search query.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

impl Item {
    /// Build-game chunks: explicit `chunks`, else the sentence split on spaces.
    pub fn build_chunks(&self) -> Vec<&str> {
        match &self.chunks {
            Some(chunks) => chunks.iter().map(String::as_str).collect(),
            None => self.korean.split_whitespace().collect(),
        }
    }

    /// Skills this item can train, derived from the data it carries.
    pub fn skills(&self) -> Vec<Skill> {
        let mut skills = vec![Skill::Listening];
        if self.replies.is_some() {
            skills.push(Skill::Response);
        }
        if self.kind == ItemKind::Sentence && BUILD_CHUNKS.contains(&self.build_chunks().len()) {
            skills.push(Skill::Build);
        }
        skills
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pack {
    pub id: String,
    pub version: u32,
    pub title: String,
    pub description: String,
    pub unlock_level: u32,
    pub items: Vec<Item>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sentence(korean: &str) -> Item {
        Item {
            id: "t/x".into(),
            kind: ItemKind::Sentence,
            korean: korean.into(),
            english: "x".into(),
            context: "test".into(),
            register: Register::Polite,
            note: None,
            lexemes: vec![],
            distractors: vec![],
            replies: None,
            chunks: None,
            image: None,
        }
    }

    #[test]
    fn skills_follow_the_data() {
        assert_eq!(sentence("뭐 했어요?").skills(), vec![Skill::Listening]);
        assert_eq!(
            sentence("주말에 뭐 했어요?").skills(),
            vec![Skill::Listening, Skill::Build]
        );
        let mut with_replies = sentence("주말에 뭐 했어요?");
        with_replies.replies = Some(Replies {
            good: vec![],
            bad: vec![],
        });
        assert_eq!(
            with_replies.skills(),
            vec![Skill::Listening, Skill::Response, Skill::Build]
        );
        let ten = sentence("가 나 다 라 마 바 사 아 자 차.");
        assert_eq!(
            ten.skills(),
            vec![Skill::Listening],
            "too many chunks for digit keys"
        );
        let mut word = sentence("자전거 타요 매일");
        word.kind = ItemKind::Word;
        assert_eq!(word.skills(), vec![Skill::Listening]);
    }

    #[test]
    fn explicit_chunks_override_spaces() {
        let mut item = sentence("주말에 뭐 했어요?");
        item.chunks = Some(vec!["주말에 뭐".into(), "했어요?".into()]);
        assert_eq!(item.build_chunks(), vec!["주말에 뭐", "했어요?"]);
        assert_eq!(item.skills(), vec![Skill::Listening]);
    }

    #[test]
    fn skill_names_round_trip() {
        for skill in Skill::ALL {
            assert_eq!(Skill::parse(skill.as_str()), Some(skill));
        }
        assert_eq!(Skill::parse("typing"), None);
    }
}
