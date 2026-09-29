//! Application state shared by commands.

use korean_db::Database;
use serde::Serialize;
use tauri::{AppHandle, Manager};

pub struct AppState {
    /// `Err` keeps the app running so the UI can explain the failure (e.g. a schema from a newer
    /// release) instead of crashing.
    pub db: Result<Database, String>,
}

impl AppState {
    pub fn db(&self) -> Result<&Database, String> {
        self.db.as_ref().map_err(Clone::clone)
    }
}

pub fn init(app: &AppHandle) -> AppState {
    let db = open_database(app);
    if let Err(err) = &db {
        log::error!("database unavailable: {err}");
    }
    AppState { db }
}

fn open_database(app: &AppHandle) -> Result<Database, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let path = dir.join("korean.db");
    let version = app.package_info().version.to_string();
    let db = tauri::async_runtime::block_on(async {
        let db = Database::open(&path, &version).await?;
        let packs = korean_core::content::bundled_packs();
        let report = korean_db::content::seed(db.pool(), &packs).await?;
        log::info!(
            "content: {} packs updated, {} unchanged, {} removed",
            report.packs_updated,
            report.packs_unchanged,
            report.packs_removed
        );
        Ok::<_, Box<dyn std::error::Error>>(db)
    })
    .map_err(|e| e.to_string())?;
    log::info!(
        "database {} at schema {}",
        db.path().display(),
        db.schema_version()
    );
    if let Some(backup) = db.backup() {
        log::info!("pre-migration backup: {}", backup.display());
    }
    Ok(db)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    version: String,
    database: Option<String>,
    schema_version: Option<i64>,
    error: Option<String>,
}

#[tauri::command]
pub fn app_status(app: AppHandle, state: tauri::State<'_, AppState>) -> AppStatus {
    let version = app.package_info().version.to_string();
    match &state.db {
        Ok(db) => AppStatus {
            version,
            database: Some(db.path().display().to_string()),
            schema_version: Some(db.schema_version()),
            error: None,
        },
        Err(err) => AppStatus {
            version,
            database: None,
            schema_version: None,
            error: Some(err.clone()),
        },
    }
}
