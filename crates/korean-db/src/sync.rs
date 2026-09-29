//! Progress sync through a shared folder (ADR-006).
//!
//! Every device writes only its own snapshot: a small SQLite file holding the user tables. Other
//! devices merge it into their local database. Merging is idempotent and order-independent, so
//! snapshots may arrive late, twice or in any order: logs are unioned, a card keeps its most
//! recent review and a setting its latest change.

use std::path::{Path, PathBuf};

use sqlx::{Connection, SqliteConnection, SqlitePool};

/// User tables copied into a snapshot. `image_refs` is a cache and `sync_peers` is local.
const EXPORTED: [&str; 4] = ["review_log", "review_states", "xp_events", "settings"];

const MERGE: [&str; 4] = [
    // Logs are append-only: add the rows not seen yet. An answer is identified by card and time.
    "INSERT INTO main.review_log (item_id, skill, correct, rating, elapsed_ms, answered_at) \
     SELECT item_id, skill, correct, rating, elapsed_ms, answered_at FROM peer.review_log p \
     WHERE NOT EXISTS (SELECT 1 FROM main.review_log l WHERE l.item_id = p.item_id \
     AND l.skill = p.skill AND l.answered_at = p.answered_at)",
    "INSERT INTO main.xp_events (earned_at, amount, item_id, skill) \
     SELECT earned_at, amount, item_id, skill FROM peer.xp_events p \
     WHERE NOT EXISTS (SELECT 1 FROM main.xp_events l WHERE l.item_id = p.item_id \
     AND l.skill = p.skill AND l.earned_at = p.earned_at)",
    // A card keeps its most recent review (`WHERE true` disambiguates the upsert).
    "INSERT INTO main.review_states (item_id, skill, step, phase, stability, difficulty, due_at, \
     last_review_at, scheduled_days, reps, lapses) \
     SELECT item_id, skill, step, phase, stability, difficulty, due_at, last_review_at, \
     scheduled_days, reps, lapses FROM peer.review_states WHERE true \
     ON CONFLICT (item_id, skill) DO UPDATE SET step = excluded.step, phase = excluded.phase, \
     stability = excluded.stability, difficulty = excluded.difficulty, due_at = excluded.due_at, \
     last_review_at = excluded.last_review_at, scheduled_days = excluded.scheduled_days, \
     reps = excluded.reps, lapses = excluded.lapses \
     WHERE excluded.last_review_at > review_states.last_review_at",
    "INSERT INTO main.settings (key, value, updated_at) \
     SELECT key, value, updated_at FROM peer.settings WHERE true \
     ON CONFLICT (key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at \
     WHERE excluded.updated_at > settings.updated_at",
];

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("i/o error on {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Outcome of merging one snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Import {
    /// `changes` local rows were added or replaced.
    Merged { device: String, changes: u64 },
    /// This snapshot was already merged.
    Unchanged { device: String },
    /// Written by a release with another schema; retried once both devices run the same one.
    SchemaMismatch { device: String, found: i64 },
}

/// Changes whenever local progress does; an export is needed only when it differs from the last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Revision {
    reviews: i64,
    xp_events: i64,
    settings_at: i64,
}

pub async fn revision(pool: &SqlitePool) -> sqlx::Result<Revision> {
    let (reviews, xp_events, settings_at) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM review_log), (SELECT COUNT(*) FROM xp_events), \
         (SELECT COALESCE(MAX(updated_at), 0) FROM settings)",
    )
    .fetch_one(pool)
    .await?;
    Ok(Revision {
        reviews,
        xp_events,
        settings_at,
    })
}

/// Writes this device's snapshot to `target`, replacing the previous one atomically.
pub async fn export(
    pool: &SqlitePool,
    schema_version: i64,
    device: &str,
    target: &Path,
    now_ms: i64,
) -> Result<(), SyncError> {
    let dir = target.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(io_error(dir))?;
    let name = target.file_name().unwrap_or_default().to_string_lossy();
    // Dot files are skipped by importers and by sync tools' default ignore rules (MEGA), so a
    // half-written snapshot never travels.
    let partial = dir.join(format!(".{name}.partial"));
    if partial.exists() {
        std::fs::remove_file(&partial).map_err(io_error(&partial))?;
    }
    let mut conn = pool.acquire().await?;
    sqlx::query("ATTACH DATABASE ? AS snap")
        .bind(partial.to_string_lossy().into_owned())
        .execute(&mut *conn)
        .await?;
    let written = write_snapshot(&mut conn, schema_version, device, now_ms).await;
    let detached = sqlx::query("DETACH DATABASE snap")
        .execute(&mut *conn)
        .await;
    written?;
    detached?;
    std::fs::rename(&partial, target).map_err(io_error(target))?;
    Ok(())
}

async fn write_snapshot(
    conn: &mut SqliteConnection,
    schema_version: i64,
    device: &str,
    now_ms: i64,
) -> sqlx::Result<()> {
    let mut tx = conn.begin().await?;
    sqlx::query(
        "CREATE TABLE snap.snapshot (device TEXT NOT NULL, schema_version INTEGER NOT NULL, \
         exported_at INTEGER NOT NULL)",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO snap.snapshot VALUES (?, ?, ?)")
        .bind(device)
        .bind(schema_version)
        .bind(now_ms)
        .execute(&mut *tx)
        .await?;
    for table in EXPORTED {
        sqlx::query(&format!(
            "CREATE TABLE snap.{table} AS SELECT * FROM main.{table}"
        ))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

/// Merges another device's snapshot into the local database. The file is opened read-only.
pub async fn import(
    pool: &SqlitePool,
    schema_version: i64,
    source: &Path,
) -> Result<Import, SyncError> {
    let mut conn = pool.acquire().await?;
    sqlx::query("ATTACH DATABASE ? AS peer")
        .bind(read_only_uri(source))
        .execute(&mut *conn)
        .await?;
    let merged = merge(&mut conn, schema_version).await;
    let detached = sqlx::query("DETACH DATABASE peer")
        .execute(&mut *conn)
        .await;
    let merged = merged?;
    detached?;
    Ok(merged)
}

async fn merge(conn: &mut SqliteConnection, schema_version: i64) -> sqlx::Result<Import> {
    let (device, found, exported_at): (String, i64, i64) =
        sqlx::query_as("SELECT device, schema_version, exported_at FROM peer.snapshot")
            .fetch_one(&mut *conn)
            .await?;
    if found != schema_version {
        return Ok(Import::SchemaMismatch { device, found });
    }
    let seen: Option<i64> =
        sqlx::query_scalar("SELECT exported_at FROM main.sync_peers WHERE device = ?")
            .bind(&device)
            .fetch_optional(&mut *conn)
            .await?;
    if seen.is_some_and(|seen| seen >= exported_at) {
        return Ok(Import::Unchanged { device });
    }
    let mut tx = conn.begin().await?;
    let mut changes = 0;
    for sql in MERGE {
        changes += sqlx::query(sql).execute(&mut *tx).await?.rows_affected();
    }
    sqlx::query(
        "INSERT INTO main.sync_peers (device, exported_at) VALUES (?, ?) \
         ON CONFLICT (device) DO UPDATE SET exported_at = excluded.exported_at",
    )
    .bind(&device)
    .bind(exported_at)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Import::Merged { device, changes })
}

/// `file:` URI opening `path` read-only, so a missing file is an error rather than created.
fn read_only_uri(path: &Path) -> String {
    let path = path
        .to_string_lossy()
        .replace('%', "%25")
        .replace('?', "%3f")
        .replace('#', "%23");
    format!("file:{path}?mode=ro")
}

fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> SyncError + '_ {
    move |source| SyncError::Io {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    async fn device(dir: &Path, name: &str) -> Database {
        Database::open(&dir.join(name).join("korean.db"), "0.7.0")
            .await
            .unwrap()
    }

    async fn answer(db: &Database, item: &str, at: i64, reps: i64) {
        let pool = db.pool();
        sqlx::query(
            "INSERT INTO review_log (item_id, skill, correct, rating, elapsed_ms, answered_at) \
             VALUES (?, 'listening', 1, 3, 900, ?)",
        )
        .bind(item)
        .bind(at)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO xp_events (earned_at, amount, item_id, skill) VALUES (?, 10, ?, 'listening')",
        )
        .bind(at)
        .bind(item)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO review_states (item_id, skill, step, phase, stability, difficulty, \
             due_at, last_review_at, scheduled_days, reps, lapses) \
             VALUES (?, 'listening', 0, 2, 1, 5, ?, ?, 1, ?, 0) \
             ON CONFLICT (item_id, skill) DO UPDATE SET last_review_at = excluded.last_review_at, \
             due_at = excluded.due_at, reps = excluded.reps",
        )
        .bind(item)
        .bind(at + 86_400_000)
        .bind(at)
        .bind(reps)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn summary(db: &Database) -> (i64, i64, Vec<(String, i64, i64)>, String) {
        let pool = db.pool();
        let reviews = sqlx::query_scalar("SELECT COUNT(*) FROM review_log")
            .fetch_one(pool)
            .await
            .unwrap();
        let xp = sqlx::query_scalar("SELECT SUM(amount) FROM xp_events")
            .fetch_one(pool)
            .await
            .unwrap();
        let states = sqlx::query_as(
            "SELECT item_id, last_review_at, reps FROM review_states ORDER BY item_id",
        )
        .fetch_all(pool)
        .await
        .unwrap();
        let focus = crate::settings::focus(pool)
            .await
            .unwrap()
            .as_str()
            .to_owned();
        (reviews, xp, states, focus)
    }

    #[tokio::test]
    async fn snapshots_merge_both_ways_to_the_same_progress() {
        use korean_core::learning::Focus;
        let tmp = tempfile::tempdir().unwrap();
        let shared = tmp.path().join("shared");
        let (laptop, desktop) = (
            device(tmp.path(), "laptop").await,
            device(tmp.path(), "desktop").await,
        );
        let schema = laptop.schema_version();

        // Laptop reviews card a, then desktop reviews a again later and b; each picks a focus.
        answer(&laptop, "p/a", 1_000, 1).await;
        crate::settings::set_focus(laptop.pool(), Focus::Words, 1_500)
            .await
            .unwrap();
        answer(&desktop, "p/a", 2_000, 2).await;
        answer(&desktop, "p/b", 3_000, 1).await;
        crate::settings::set_focus(desktop.pool(), Focus::All, 4_000)
            .await
            .unwrap();

        let laptop_file = shared.join("laptop.db");
        let desktop_file = shared.join("desktop.db");
        export(laptop.pool(), schema, "laptop", &laptop_file, 5_000)
            .await
            .unwrap();
        export(desktop.pool(), schema, "desktop", &desktop_file, 5_000)
            .await
            .unwrap();

        assert_eq!(
            import(laptop.pool(), schema, &desktop_file).await.unwrap(),
            // 2 log rows, 2 xp events, a replaced, b added, focus replaced.
            Import::Merged {
                device: "desktop".into(),
                changes: 7
            }
        );
        assert!(matches!(
            import(desktop.pool(), schema, &laptop_file).await.unwrap(),
            Import::Merged { .. }
        ));
        let expected = (
            3,
            30,
            vec![("p/a".into(), 2_000, 2), ("p/b".into(), 3_000, 1)],
            "all".to_owned(),
        );
        assert_eq!(summary(&laptop).await, expected);
        assert_eq!(summary(&desktop).await, expected);

        // The same snapshot is not merged twice; a newer one with nothing new changes nothing.
        assert_eq!(
            import(laptop.pool(), schema, &desktop_file).await.unwrap(),
            Import::Unchanged {
                device: "desktop".into()
            }
        );
        export(desktop.pool(), schema, "desktop", &desktop_file, 6_000)
            .await
            .unwrap();
        assert_eq!(
            import(laptop.pool(), schema, &desktop_file).await.unwrap(),
            Import::Merged {
                device: "desktop".into(),
                changes: 0
            }
        );
        assert_eq!(summary(&laptop).await, expected);
        assert!(!shared.join(".desktop.db.partial").exists());
    }

    #[tokio::test]
    async fn a_snapshot_from_another_schema_is_left_alone() {
        let tmp = tempfile::tempdir().unwrap();
        let (laptop, desktop) = (
            device(tmp.path(), "laptop").await,
            device(tmp.path(), "desktop").await,
        );
        let schema = laptop.schema_version();
        answer(&desktop, "p/a", 1_000, 1).await;
        let file = tmp.path().join("shared").join("desktop.db");
        export(desktop.pool(), schema + 1, "desktop", &file, 5_000)
            .await
            .unwrap();
        assert_eq!(
            import(laptop.pool(), schema, &file).await.unwrap(),
            Import::SchemaMismatch {
                device: "desktop".into(),
                found: schema + 1
            }
        );
        assert_eq!(summary(&laptop).await.0, 0);
    }

    #[tokio::test]
    async fn a_missing_snapshot_is_an_error_and_is_not_created() {
        let tmp = tempfile::tempdir().unwrap();
        let laptop = device(tmp.path(), "laptop").await;
        let file = tmp.path().join("gone.db");
        assert!(
            import(laptop.pool(), laptop.schema_version(), &file)
                .await
                .is_err()
        );
        assert!(!file.exists());
    }

    #[tokio::test]
    async fn an_answer_moves_the_revision() {
        let tmp = tempfile::tempdir().unwrap();
        let laptop = device(tmp.path(), "laptop").await;
        let before = revision(laptop.pool()).await.unwrap();
        answer(&laptop, "p/a", 1_000, 1).await;
        assert_ne!(revision(laptop.pool()).await.unwrap(), before);
    }
}
