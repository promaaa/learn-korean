//! `anki-import import <deck.csv> <raw.json>`: CSV → immutable raw rows.
//! `anki-import build <legacy-dir>`: `raw.json` + `curation.json` → `lexemes.json` + `pack.json`.

use std::path::Path;
use std::process::ExitCode;

use anki_import::{Curation, build, import_csv, issues, to_json};
use korean_core::content::RawEntry;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["import", csv, out] => import(Path::new(csv), Path::new(out)),
        ["build", dir] => build_dir(Path::new(dir)),
        _ => Err("usage: anki-import import <deck.csv> <raw.json> | build <legacy-dir>".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn import(csv: &Path, out: &Path) -> Result<()> {
    let rows = import_csv(&std::fs::read_to_string(csv)?)?;
    for row in &rows {
        let found = issues(row);
        if !found.is_empty() {
            println!(
                "row {:>3} {:?}: {} -> {}",
                row.row, found, row.front_raw, row.back_raw
            );
        }
    }
    std::fs::write(out, to_json(&rows))?;
    println!("{} raw rows written to {}", rows.len(), out.display());
    Ok(())
}

fn build_dir(dir: &Path) -> Result<()> {
    let rows: Vec<RawEntry> =
        serde_json::from_str(&std::fs::read_to_string(dir.join("raw.json"))?)?;
    let curation: Curation =
        serde_json::from_str(&std::fs::read_to_string(dir.join("curation.json"))?)?;
    let built = build(&rows, &curation)?;
    std::fs::write(dir.join("lexemes.json"), to_json(&built.lexemes))?;
    std::fs::write(dir.join("pack.json"), to_json(&built.pack))?;
    let fixes = built
        .lexemes
        .lexemes
        .iter()
        .filter(|l| l.fix.is_some())
        .count();
    println!(
        "{} lexemes ({fixes} fixed), {} items in the legacy pack",
        built.lexemes.lexemes.len(),
        built.pack.items.len()
    );
    Ok(())
}
