use super::glossary::Glossary;
use super::model::Pack;

/// Packs shipped inside the binary, in unlock order.
const BUNDLED: &[(&str, &str)] = &[
    (
        "starter",
        include_str!("../../../../content/starter/pack.json"),
    ),
    (
        "legacy",
        include_str!("../../../../content/legacy/pack.json"),
    ),
    (
        "small-talk",
        include_str!("../../../../content/small-talk/pack.json"),
    ),
    (
        "restaurants",
        include_str!("../../../../content/restaurants/pack.json"),
    ),
];

const GLOSSARY: &str = include_str!("../../../../content/glossary.json");

/// Parses the bundled packs. They are validated in CI, so a parse failure is a build defect.
pub fn bundled_packs() -> Vec<Pack> {
    BUNDLED
        .iter()
        .map(|(name, json)| {
            serde_json::from_str(json)
                .unwrap_or_else(|e| panic!("bundled pack {name} is invalid: {e}"))
        })
        .collect()
}

/// Parses the bundled glossary, validated against the packs in CI.
pub fn bundled_glossary() -> Glossary {
    serde_json::from_str(GLOSSARY)
        .unwrap_or_else(|e| panic!("bundled content/glossary.json is invalid: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{validate_glossary, validate_packs};

    #[test]
    fn bundled_content_is_valid() {
        let packs = bundled_packs();
        for (pack, (name, _)) in packs.iter().zip(BUNDLED) {
            assert_eq!(&pack.id, name, "pack id must match its directory");
        }
        let errors: Vec<_> = [
            validate_packs(&packs),
            validate_glossary(&packs, &bundled_glossary()),
        ]
        .into_iter()
        .filter_map(Result::err)
        .flatten()
        .collect();
        if !errors.is_empty() {
            let report: Vec<String> = errors.iter().map(ToString::to_string).collect();
            panic!("invalid content:\n{}", report.join("\n"));
        }
    }
}
