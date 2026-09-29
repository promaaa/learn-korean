//! Progress sync between devices through a shared folder (ADR-006). This device's snapshot is
//! written when the window is hidden or loses focus; the other devices' snapshots are merged at
//! start and every time the window is summoned.
//!
//! Enabled by `<app config dir>/sync.json`: `{ "dir": "~/MegaSync/learn-korean" }`, optionally with
//! `"device"` to override the host name used for this device's snapshot file.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use korean_db::Database;
use korean_db::sync::{Import, Revision};
use serde::Deserialize;
use tauri::async_runtime::Mutex;
use tauri::{AppHandle, Manager};

use crate::session::now_ms;
use crate::state::AppState;

const CONFIG: &str = "sync.json";

#[derive(Deserialize)]
struct Config {
    dir: PathBuf,
    device: Option<String>,
}

struct Target {
    dir: PathBuf,
    device: String,
}

pub struct ProgressSync {
    target: Option<Target>,
    /// Serializes pulls and pushes; holds the revision of the last export.
    exported: Mutex<Option<Revision>>,
}

impl ProgressSync {
    pub fn load(app: &AppHandle) -> Self {
        let target = load_target(app).unwrap_or_else(|err| {
            log::error!("sync disabled: {err}");
            None
        });
        match &target {
            Some(t) => log::info!("sync: {} as {:?}", t.dir.display(), t.device),
            None => log::info!("sync off (no {CONFIG})"),
        }
        Self {
            target,
            exported: Mutex::new(None),
        }
    }

    /// Merges the other devices' snapshots. True when local progress changed.
    pub async fn pull(&self, db: &Database) -> bool {
        let Some(target) = &self.target else {
            return false;
        };
        let _serialized = self.exported.lock().await;
        let entries = match std::fs::read_dir(&target.dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == ErrorKind::NotFound => return false,
            Err(err) => {
                log::warn!("sync: cannot list {}: {err}", target.dir.display());
                return false;
            }
        };
        let mut changed = false;
        for path in entries.filter_map(|e| e.ok()).map(|e| e.path()) {
            if !is_peer_snapshot(&path, &target.device) {
                continue;
            }
            match korean_db::sync::import(db.pool(), db.schema_version(), &path).await {
                Ok(Import::Merged { device, changes }) => {
                    log::info!("sync: merged {device}, {changes} changes");
                    changed |= changes > 0;
                }
                Ok(Import::Unchanged { .. }) => {}
                Ok(Import::SchemaMismatch { device, found }) => log::warn!(
                    "sync: {device} is at schema {found}, this release at {}; \
                     install the same release on both devices",
                    db.schema_version()
                ),
                Err(err) => log::warn!("sync: cannot merge {}: {err}", path.display()),
            }
        }
        changed
    }

    /// Writes this device's snapshot if progress changed since the last export.
    pub async fn push(&self, db: &Database) {
        let Some(target) = &self.target else {
            return;
        };
        let mut exported = self.exported.lock().await;
        let revision = match korean_db::sync::revision(db.pool()).await {
            Ok(revision) => revision,
            Err(err) => {
                log::warn!("sync: {err}");
                return;
            }
        };
        if *exported == Some(revision) {
            return;
        }
        let file = target.dir.join(format!("{}.db", target.device));
        match korean_db::sync::export(
            db.pool(),
            db.schema_version(),
            &target.device,
            &file,
            now_ms(),
        )
        .await
        {
            Ok(()) => {
                *exported = Some(revision);
                log::info!("sync: wrote {}", file.display());
            }
            Err(err) => log::warn!("sync: cannot write {}: {err}", file.display()),
        }
    }
}

/// Pushes in the background, e.g. once the window is hidden or unfocused.
pub fn push_later(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let (Some(sync), Some(state)) =
            (app.try_state::<ProgressSync>(), app.try_state::<AppState>())
        else {
            return;
        };
        if let Ok(db) = state.db() {
            sync.push(db).await;
        }
    });
}

/// Pulls, then reports whether progress changed. False before setup finished or without a DB.
pub async fn pull_now(app: &AppHandle) -> bool {
    let (Some(sync), Some(state)) = (app.try_state::<ProgressSync>(), app.try_state::<AppState>())
    else {
        return false;
    };
    match state.db() {
        Ok(db) => sync.pull(db).await,
        Err(_) => false,
    }
}

fn load_target(app: &AppHandle) -> Result<Option<Target>, String> {
    let path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join(CONFIG);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("{}: {err}", path.display())),
    };
    let config: Config =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let dir = match config.dir.strip_prefix("~") {
        Ok(rest) => app.path().home_dir().map_err(|e| e.to_string())?.join(rest),
        Err(_) => config.dir,
    };
    let device = config
        .device
        .unwrap_or_else(|| gethostname::gethostname().to_string_lossy().into_owned());
    Ok(Some(Target {
        dir,
        device: file_safe(&device),
    }))
}

/// A device name usable as a file name on every platform.
fn file_safe(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if safe.is_empty() {
        "device".into()
    } else {
        safe
    }
}

/// `<other device>.db`; dot files are partial writes or sync-tool metadata.
fn is_peer_snapshot(path: &Path, device: &str) -> bool {
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    path.extension().is_some_and(|ext| ext == "db") && !stem.starts_with('.') && stem != device
}
