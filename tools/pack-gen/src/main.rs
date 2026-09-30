//! `pack-gen check [--draft <file>] <pack-id>`: validates the packs on disk, a draft merged in.
//! `pack-gen draft <pack-id> --brief <scenario> [--count N] [--title T --description D --level L]`:
//! asks an LLM (`PACK_GEN_LLM`, default `claude -p`) for new items into `content/<id>/draft.json`.
//! `pack-gen review <pack-id>`: accept, skip or edit each draft item into the pack.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use korean_core::content::{Item, ValidationError, bundled_pack_ids, item_words};
use pack_gen::draft::{self, Draft, PackMeta, parse_generated};
use pack_gen::library::{
    draft_path, glossary_path, pack_path, read_glossary, read_json_opt, shown, to_json, write_json,
};
use pack_gen::prompt::{Request, format_rules, prompt};
use pack_gen::{Error, Library};

const USAGE: &str = "usage:
  pack-gen check [--draft <file>] <pack-id>
  pack-gen draft <pack-id> --brief <scenario> [--count N] [--title T --description D --level L]
  pack-gen review <pack-id>";

/// Items asked for when `--count` is not given.
const DEFAULT_COUNT: usize = 10;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(err) => {
            eprintln!("pack-gen: {err}");
            ExitCode::from(2)
        }
    }
}

/// Runs a command: `Ok(false)` when the content it checked has errors.
fn run(args: &[String]) -> Result<bool, Error> {
    let root = repo_root()?;
    let content = root.join("content");
    let Some((command, rest)) = args.split_first() else {
        return Err(USAGE.to_string().into());
    };
    match command.as_str() {
        "check" => {
            let args = Args::parse(rest, &["draft"])?;
            check(&content, args.pack_id()?, args.get("draft").map(Path::new))
        }
        "draft" => {
            let args = Args::parse(rest, &["brief", "count", "title", "description", "level"])?;
            draft(&root, &content, args.pack_id()?, &args)
        }
        "review" => review(&content, Args::parse(rest, &[])?.pack_id()?),
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            Ok(true)
        }
        other => Err(format!("unknown command {other:?}\n{USAGE}").into()),
    }
}

/// The repository this binary was built from: `content/` and `docs/` are read from there.
fn repo_root() -> Result<PathBuf, Error> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::canonicalize(&root).map_err(|e| format!("{}: {e}", root.display()).into())
}

#[derive(Default)]
struct Args {
    positional: Vec<String>,
    options: HashMap<String, String>,
}

impl Args {
    /// `--name value` options among `allowed`, and positional arguments.
    fn parse(args: &[String], allowed: &[&str]) -> Result<Args, Error> {
        let mut parsed = Args::default();
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            let Some(name) = arg.strip_prefix("--") else {
                parsed.positional.push(arg.clone());
                continue;
            };
            if !allowed.contains(&name) {
                return Err(format!("unknown option --{name}\n{USAGE}").into());
            }
            let value = args
                .next()
                .ok_or_else(|| format!("--{name} needs a value"))?;
            if parsed.options.insert(name.into(), value.clone()).is_some() {
                return Err(format!("--{name} is given twice").into());
            }
        }
        Ok(parsed)
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.options.get(name).map(String::as_str)
    }

    fn pack_id(&self) -> Result<&str, Error> {
        match &self.positional[..] {
            [id] => Ok(id),
            _ => Err(format!("expected one pack id\n{USAGE}").into()),
        }
    }
}

/// The bundled packs, read from disk, with the pack being authored.
struct Opened {
    library: Library,
    index: usize,
    /// Whether the pack is listed in `BUNDLED`; otherwise it was appended as the last pack.
    bundled: bool,
}

/// Loads the bundled packs from `content` and appends pack `id` when it is not bundled: from its
/// `pack.json`, else created from `meta`.
fn open(content: &Path, id: &str, meta: Option<&PackMeta>) -> Result<Opened, Error> {
    let ids = bundled_pack_ids();
    let mut library = Library::load(content, &ids)?;
    if let Some(index) = library.index(id) {
        return Ok(Opened {
            library,
            index,
            bundled: true,
        });
    }
    let path = pack_path(content, id);
    let pack = match (read_json_opt(&path)?, meta) {
        (Some(pack), _) => pack,
        (None, Some(meta)) => meta.pack(id),
        (None, None) => {
            return Err(format!(
                "no pack {id:?}: {} does not exist and no draft gives its title, description and \
                 level (bundled packs: {})",
                shown(&path),
                ids.join(", ")
            )
            .into());
        }
    };
    let glossary = read_glossary(content, id)?;
    Ok(Opened {
        index: library.push(pack, glossary),
        library,
        bundled: false,
    })
}

fn check(content: &Path, id: &str, draft_file: Option<&Path>) -> Result<bool, Error> {
    let draft: Option<Draft> = match draft_file {
        Some(path) => Some(
            read_json_opt(path)?.ok_or_else(|| format!("{}: no such draft file", shown(path)))?,
        ),
        None => None,
    };
    let opened = open(content, id, draft.as_ref().and_then(|d| d.pack.as_ref()))?;
    let errors = match &draft {
        Some(draft) => draft::check_draft(opened.library.clone(), opened.index, draft),
        None => opened.library.check(),
    };
    Ok(report(&opened, draft.as_ref().zip(draft_file), &errors))
}

/// Prints every error, then a summary line. Returns whether the content is valid.
fn report(opened: &Opened, draft: Option<(&Draft, &Path)>, errors: &[ValidationError]) -> bool {
    let pack = &opened.library.packs[opened.index];
    let glossary = &opened.library.glossaries[opened.index];
    let mut summary = format!(
        "{}: {}, {}",
        pack.id,
        plural(pack.items.len(), "item"),
        plural(glossary.len(), "gloss")
    );
    if let Some((draft, path)) = draft {
        let _ = write!(
            summary,
            " + draft {}: {}, {}",
            shown(path),
            plural(draft.items.len(), "item"),
            plural(draft.glossary.len(), "gloss")
        );
    }
    if !opened.bundled {
        println!(
            "note: {id} is not bundled yet, so it is checked as the last pack. To ship it, add \
             \"{id}\", to BUNDLED in crates/korean-core/src/content/bundled.rs",
            id = pack.id
        );
    }
    for error in errors {
        println!("{error}");
    }
    let packs = opened.library.packs.len();
    if errors.is_empty() {
        println!("OK {summary} ({packs} packs checked)");
        true
    } else {
        println!("{} ({summary})", plural(errors.len(), "error"));
        false
    }
}

/// `1 item`, `2 items`, `2 glosses`.
fn plural(n: usize, noun: &str) -> String {
    match (n, noun.ends_with('s')) {
        (1, _) => format!("1 {noun}"),
        (_, true) => format!("{n} {noun}es"),
        (_, false) => format!("{n} {noun}s"),
    }
}

fn draft(root: &Path, content: &Path, id: &str, args: &Args) -> Result<bool, Error> {
    let scenario = args.get("brief").ok_or("--brief <scenario> is required")?;
    let count = match args.get("count") {
        None => DEFAULT_COUNT,
        Some(count) => count
            .parse()
            .ok()
            .filter(|n| *n > 0)
            .ok_or_else(|| format!("--count {count:?} is not a positive number"))?,
    };
    let path = draft_path(content, id);
    let mut draft: Draft = read_json_opt(&path)?.unwrap_or_default();
    let exists = bundled_pack_ids().contains(&id) || pack_path(content, id).exists();
    let meta = [
        args.get("title"),
        args.get("description"),
        args.get("level"),
    ];
    match meta {
        _ if exists && meta.iter().any(Option::is_some) => {
            return Err(format!(
                "pack {id} exists: --title, --description and --level are for new packs"
            )
            .into());
        }
        [Some(title), Some(description), Some(level)] if !exists => {
            let unlock_level = level
                .parse()
                .ok()
                .filter(|l| *l >= 1)
                .ok_or_else(|| format!("--level {level:?} is not a level >= 1"))?;
            draft.pack = Some(PackMeta {
                title: title.into(),
                description: description.into(),
                unlock_level,
            });
        }
        _ if exists || (draft.pack.is_some() && meta.iter().all(Option::is_none)) => {}
        _ => {
            return Err(
                format!("{id} is a new pack: give --title, --description and --level").into(),
            );
        }
    }
    let opened = open(content, id, draft.pack.as_ref())?;

    let docs = root.join("docs").join("content.md");
    let rules = std::fs::read_to_string(&docs).map_err(|e| format!("{}: {e}", shown(&docs)))?;
    let request = Request {
        pack: &opened.library.packs[opened.index],
        scenario,
        count,
    };
    let text = prompt(&format_rules(&rules), &opened.library, &draft, &request);
    let command = std::env::var("PACK_GEN_LLM").unwrap_or_else(|_| "claude -p".into());
    eprintln!("asking `{command}` for {count} items for {id}...");
    let answer = run_llm(&command, &text)?;
    let generated = parse_generated(&answer).map_err(|e| {
        let saved = std::env::temp_dir().join(format!("pack-gen-{id}-answer.txt"));
        let saved = match std::fs::write(&saved, &answer) {
            Ok(()) => format!("answer saved to {}", saved.display()),
            Err(write) => format!("answer not saved: {write}"),
        };
        format!("the LLM answer is not {{\"items\": [...], \"glossary\": {{...}}}}: {e} ({saved})")
    })?;
    if generated.items.is_empty() {
        return Err("the LLM drafted no items".to_string().into());
    }
    let added = generated.items.len();
    draft.extend(generated);
    write_json(&path, &draft)?;
    println!(
        "drafted {added} items into {} ({} waiting for review)",
        shown(&path),
        draft.items.len()
    );
    let errors = draft::check_draft(opened.library.clone(), opened.index, &draft);
    let ok = report(&opened, Some((&draft, &path)), &errors);
    if ok {
        println!("next: `pack-gen review {id}`");
    } else {
        println!(
            "next: fix the errors in {} (or with e in `pack-gen review {id}`)",
            shown(&path)
        );
    }
    Ok(ok)
}

/// Runs `command` through `sh -c` with `prompt` on stdin and returns its stdout.
fn run_llm(command: &str, prompt: &str) -> Result<String, Error> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("cannot run `{command}`: {e}"))?;
    let mut stdin = child.stdin.take().expect("stdin is piped");
    let prompt = prompt.to_string();
    // Written from another thread so a command answering while it reads cannot deadlock.
    let writer = std::thread::spawn(move || stdin.write_all(prompt.as_bytes()));
    let output = child
        .wait_with_output()
        .map_err(|e| format!("`{command}`: {e}"))?;
    // A command that exits without reading the whole prompt is judged by its status alone.
    let _ = writer.join();
    if !output.status.success() {
        return Err(format!("`{command}` failed ({})", output.status).into());
    }
    String::from_utf8(output.stdout)
        .map_err(|_| format!("`{command}` printed invalid UTF-8").into())
}

fn review(content: &Path, id: &str) -> Result<bool, Error> {
    if !std::io::stdin().is_terminal() {
        return Err("review is interactive: run it in a terminal"
            .to_string()
            .into());
    }
    let path = draft_path(content, id);
    let mut draft: Draft = read_json_opt(&path)?.ok_or_else(|| {
        format!(
            "{} does not exist: run `pack-gen draft {id}` first",
            shown(&path)
        )
    })?;
    let mut opened = open(content, id, draft.pack.as_ref())?;
    let total = draft.items.len();
    let (mut accepted, mut skipped) = (0, 0);
    // An edit that did not parse, reopened by the next edit.
    let mut pending: Option<String> = None;
    let mut input = std::io::stdin().lock();
    while !draft.items.is_empty() {
        let position = total - draft.items.len() + 1;
        println!(
            "\n[{position}/{total}] {}",
            describe(&draft.items[0], &opened.library, &draft)
        );
        print!("[a]ccept [s]kip [e]dit [q]uit > ");
        std::io::stdout().flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        if input.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            break;
        }
        match line.trim() {
            "a" => {
                let index = opened.index;
                let changed = draft::accept(&mut opened.library, index, &mut draft, 0);
                let library = &opened.library;
                write_json(&pack_path(content, id), &library.packs[index])?;
                for i in changed.into_iter().chain([index]).collect::<BTreeSet<_>>() {
                    let pack = &library.packs[i].id;
                    write_json(&glossary_path(content, pack), &library.glossaries[i])?;
                }
                save_draft(&path, &draft)?;
                pending = None;
                accepted += 1;
            }
            "s" => {
                draft.discard(0);
                save_draft(&path, &draft)?;
                pending = None;
                skipped += 1;
            }
            "e" => match edit(&draft.items[0], &mut pending)? {
                Ok(item) => {
                    draft.items[0] = item;
                    save_draft(&path, &draft)?;
                }
                Err(message) => println!("{message}"),
            },
            "q" => break,
            _ => println!("type a, s, e or q, then Enter"),
        }
    }
    println!(
        "\n{accepted} accepted, {skipped} skipped, {} left in {}",
        draft.items.len(),
        shown(&path)
    );
    let errors = opened.library.check();
    Ok(report(&opened, None, &errors))
}

/// Writes the draft, or deletes it once every item is handled.
fn save_draft(path: &Path, draft: &Draft) -> Result<(), Error> {
    if draft.items.is_empty() {
        return match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                Err(format!("{}: {e}", shown(path)).into())
            }
            _ => Ok(()),
        };
    }
    write_json(path, draft)
}

/// Everything a reviewer needs to judge an item, the glosses of its words included.
fn describe(item: &Item, library: &Library, draft: &Draft) -> String {
    let lower = |debug: String| debug.to_lowercase();
    let mut s = format!(
        "{} · {} · {} · {}\n  {}\n  {}\n",
        item.id,
        lower(format!("{:?}", item.kind)),
        lower(format!("{:?}", item.register)),
        item.context,
        item.korean,
        item.english
    );
    if !item.distractors.is_empty() {
        let _ = writeln!(s, "  distractors: {}", item.distractors.join(" | "));
    }
    if let Some(replies) = &item.replies {
        for (label, lines) in [("good", &replies.good), ("bad", &replies.bad)] {
            for line in lines {
                let _ = writeln!(s, "  reply {label:<4} {} = {}", line.korean, line.english);
            }
        }
    }
    if let Some(chunks) = &item.chunks {
        let _ = writeln!(s, "  chunks: {}", chunks.join(" | "));
    }
    let _ = writeln!(s, "  lexemes: {}", item.lexemes.join(", "));
    if let Some(image) = &item.image {
        let _ = writeln!(s, "  image: {image}");
    }
    if let Some(note) = &item.note {
        let _ = writeln!(s, "  note: {note}");
    }
    s.push_str("  glosses:");
    let mut seen = BTreeSet::new();
    for word in item_words(item).filter(|w| seen.insert(*w)) {
        let (gloss, source) = match (library.gloss(word), draft.glossary.get(word)) {
            (Some((pack, gloss)), _) => (gloss, library.packs[pack].id.as_str()),
            (None, Some(gloss)) => (gloss, "draft"),
            (None, None) => ("-", "NO GLOSS"),
        };
        let _ = write!(s, "\n    {word} = {gloss}  [{source}]");
    }
    s
}

/// Opens the item's JSON (or the previous edit that did not parse) in `$VISUAL`, `$EDITOR` or
/// `vi`, and parses the result.
fn edit(item: &Item, pending: &mut Option<String>) -> Result<Result<Item, String>, Error> {
    let file = std::env::temp_dir().join(format!("pack-gen-{}-item.json", std::process::id()));
    let io = |e: std::io::Error| Error(format!("{}: {e}", file.display()));
    std::fs::write(&file, pending.take().unwrap_or_else(|| to_json(item))).map_err(io)?;
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".into());
    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("{editor} \"$1\""))
        .arg("pack-gen")
        .arg(&file)
        .status()
        .map_err(|e| format!("cannot run `{editor}`: {e}"))?;
    let edited = std::fs::read_to_string(&file).map_err(io)?;
    let _ = std::fs::remove_file(&file);
    if !status.success() {
        return Ok(Err(format!("`{editor}` failed ({status}); item unchanged")));
    }
    match serde_json::from_str(&edited) {
        Ok(item) => Ok(Ok(item)),
        Err(e) => {
            *pending = Some(edited);
            Ok(Err(format!("not a valid item: {e}; press e to fix it")))
        }
    }
}
