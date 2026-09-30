//! The prompt asking an LLM to draft items for a pack.

use std::collections::BTreeSet;
use std::fmt::Write;

use korean_core::content::{GRAMMAR, ItemKind, Pack, carded_lemmas};

use crate::Library;
use crate::draft::Draft;

/// The pack format rules of `docs/content.md`: every section but those about tools.
pub fn format_rules(content_md: &str) -> String {
    const TOOL_SECTIONS: [&str; 2] = ["## Legacy Anki deck", "## Authoring with pack-gen"];
    let mut keep = true;
    let mut rules = String::new();
    for line in content_md.lines() {
        if line.starts_with("## ") {
            keep = !TOOL_SECTIONS.contains(&line.trim_end());
        }
        if keep {
            rules.push_str(line);
            rules.push('\n');
        }
    }
    rules
}

/// What to draft: `count` items for `pack` (possibly new, not in the library) about `scenario`.
pub struct Request<'a> {
    pub pack: &'a Pack,
    pub scenario: &'a str,
    pub count: usize,
}

pub fn prompt(rules: &str, library: &Library, draft: &Draft, request: &Request) -> String {
    let Request {
        pack,
        scenario,
        count,
    } = request;
    let id = &pack.id;
    let mut carded: BTreeSet<&str> = carded_lemmas(&library.packs).into_iter().collect();
    carded.extend(
        draft
            .items
            .iter()
            .filter(|i| i.kind == ItemKind::Word)
            .flat_map(|i| i.lexemes.iter().map(String::as_str)),
    );
    let glossed: BTreeSet<&str> = library
        .glossaries
        .iter()
        .chain([&draft.glossary])
        .flat_map(|g| g.iter().map(|(word, _)| word))
        .collect();
    let join = |words: &BTreeSet<&str>| words.iter().copied().collect::<Vec<_>>().join(", ");

    let mut p = String::new();
    // `write!` to a String cannot fail.
    let _ = writeln!(
        p,
        "You write content for a Korean learning app for English speakers. Draft {count} new \
         items for the content pack \"{id}\" (\"{}\": {}; unlocked at level {}).\n\n\
         Scenario: {scenario}\n",
        pack.title, pack.description, pack.unlock_level
    );
    let _ = writeln!(p, "# Content format and rules\n\n{rules}");
    let _ = writeln!(
        p,
        "# Lines already in this pack\n\nDo not repeat them; a Korean line may appear only once \
         across all packs.\n"
    );
    for item in pack.items.iter().chain(&draft.items) {
        let _ = writeln!(p, "- {} = {}", item.korean, item.english);
    }
    if pack.items.is_empty() && draft.items.is_empty() {
        let _ = writeln!(p, "(none yet: this is a new pack)");
    }
    let _ = writeln!(
        p,
        "\n# Lemmas that already have a word card\n\nReuse them freely; never add another card for \
         them: {}\n\n\
         Every other lemma you list in a sentence's `lexemes` needs a new word card in this pack \
         (kind \"word\", `korean` and `lexemes` its dictionary form), placed before the first \
         sentence using it. Grammar lemmas get no card: {}.\n",
        join(&carded),
        GRAMMAR.join(", ")
    );
    let _ = writeln!(
        p,
        "# Words already glossed\n\nNever put these in \"glossary\": {}\n",
        join(&glossed)
    );
    let _ = writeln!(
        p,
        "# Answer\n\n\
         Answer with one JSON object and nothing else: {{\"items\": [...], \"glossary\": {{...}}}}.\n\
         - `items`: the new items in teaching order (word cards before the sentences using them), \
         each following the item format above; ids are \"{id}/<kebab-slug>\". Mostly sentences \
         with `replies`, natural everyday Korean, polite register unless the scenario needs \
         another. Never romanization.\n\
         - `glossary`: every word form of your new lines (item `korean` and every reply, split on \
         spaces, `? . , ! ~` stripped) that is not listed under \"Words already glossed\", mapped \
         to its English (at most 40 characters, following the glossary rules above). Each form \
         once."
    );
    p
}
