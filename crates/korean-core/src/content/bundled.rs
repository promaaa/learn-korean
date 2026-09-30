use super::glossary::Glossary;
use super::model::Pack;

/// `(id, pack.json, glossary.json)` of each listed pack directory `content/<id>/`.
macro_rules! bundle {
    ($($id:literal),+ $(,)?) => {
        &[$((
            $id,
            include_str!(concat!("../../../../content/", $id, "/pack.json")),
            include_str!(concat!("../../../../content/", $id, "/glossary.json")),
        )),+]
    };
}

/// Packs shipped inside the binary, in unlock order. A word's gloss lives in the glossary of the
/// first of these packs whose lines use it.
const BUNDLED: &[(&str, &str, &str)] = bundle! {
    "starter",
    "legacy",
    "everyday-words",
    "small-talk",
    "restaurants",
    "numbers-time",
    "transport",
    "shopping",
    "phone-calls",
    "doctor",
    "workplace",
    "k-drama",
};

/// Ids of the bundled packs, in bundle (unlock) order.
pub fn bundled_pack_ids() -> Vec<&'static str> {
    BUNDLED.iter().map(|(id, ..)| *id).collect()
}

/// Parses the bundled packs. They are validated in CI, so a parse failure is a build defect.
pub fn bundled_packs() -> Vec<Pack> {
    BUNDLED
        .iter()
        .map(|(id, json, _)| {
            serde_json::from_str(json)
                .unwrap_or_else(|e| panic!("bundled content/{id}/pack.json is invalid: {e}"))
        })
        .collect()
}

/// The glossary of each bundled pack, in bundle order.
fn bundled_glossaries() -> Vec<Glossary> {
    BUNDLED
        .iter()
        .map(|(id, _, json)| {
            serde_json::from_str(json)
                .unwrap_or_else(|e| panic!("bundled content/{id}/glossary.json is invalid: {e}"))
        })
        .collect()
}

/// The glossaries of all bundled packs merged, validated against the packs in CI.
pub fn bundled_glossary() -> Glossary {
    bundled_glossaries().into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{validate_glossary, validate_packs, validate_word_cards};

    #[test]
    fn bundled_content_is_valid() {
        let packs = bundled_packs();
        for (pack, (id, ..)) in packs.iter().zip(BUNDLED) {
            assert_eq!(&pack.id, id, "pack id must match its directory");
        }
        let errors: Vec<_> = [
            validate_packs(&packs),
            validate_glossary(&packs, &bundled_glossaries()),
            validate_word_cards(&packs),
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
