use super::model::Pack;

/// Packs shipped inside the binary, in unlock order.
const BUNDLED: &[(&str, &str)] = &[(
    "starter",
    include_str!("../../../../content/starter/pack.json"),
)];

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::validate_packs;

    #[test]
    fn bundled_content_is_valid() {
        let packs = bundled_packs();
        for (pack, (name, _)) in packs.iter().zip(BUNDLED) {
            assert_eq!(&pack.id, name, "pack id must match its directory");
        }
        if let Err(errors) = validate_packs(&packs) {
            let report: Vec<String> = errors.iter().map(ToString::to_string).collect();
            panic!("invalid content:\n{}", report.join("\n"));
        }
    }
}
