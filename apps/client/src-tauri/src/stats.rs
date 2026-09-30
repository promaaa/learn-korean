//! Statistics screen. Rules live in `korean_core::stats`; this module only loads the data.

use korean_core::stats::{self, Stats};
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub async fn stats(app: State<'_, AppState>) -> Result<Stats, String> {
    let pool = app.db()?.pool();
    let err = |e: sqlx::Error| e.to_string();
    let answers = korean_db::stats::answers(pool).await.map_err(err)?;
    let states = korean_db::reviews::states(pool).await.map_err(err)?;
    // Every bundled pack, locked ones included: vocabulary totals describe the whole course.
    let items = korean_db::content::unlocked_items(pool, u32::MAX)
        .await
        .map_err(err)?;
    Ok(stats::compute(
        &answers,
        &states,
        &items,
        crate::session::now_ms(),
        crate::progress::utc_offset_seconds(),
    ))
}
