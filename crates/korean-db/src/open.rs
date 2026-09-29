use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use sqlx::migrate::{Migrate, Migrator};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Connection, SqliteConnection, SqlitePool};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("i/o error on {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(
        "database schema {found} is newer than this app supports ({supported}); \
         install a newer release or restore a backup from {backups}"
    )]
    SchemaTooNew {
        found: i64,
        supported: i64,
        backups: PathBuf,
    },
}

#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
    path: PathBuf,
    schema_version: i64,
    backup: Option<PathBuf>,
}

impl Database {
    /// Opens (creating if needed) the database at `path` and migrates it to the latest schema.
    pub async fn open(path: &Path, app_version: &str) -> Result<Self, OpenError> {
        open_with(path, app_version, &MIGRATOR).await
    }

    /// In-memory database with every migration applied (tests, previews).
    pub async fn in_memory() -> Result<Self, OpenError> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        // A single connection: every pooled connection would otherwise get its own empty DB.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        MIGRATOR.run(&pool).await?;
        Ok(Self {
            pool,
            path: PathBuf::from(":memory:"),
            schema_version: latest_version(&MIGRATOR),
            backup: None,
        })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Highest applied migration version.
    pub fn schema_version(&self) -> i64 {
        self.schema_version
    }

    /// Backup written by this open, if migrations were pending on an existing database.
    pub fn backup(&self) -> Option<&Path> {
        self.backup.as_deref()
    }
}

fn latest_version(migrator: &Migrator) -> i64 {
    migrator.iter().map(|m| m.version).max().unwrap_or(0)
}

pub(crate) async fn open_with(
    path: &Path,
    app_version: &str,
    migrator: &Migrator,
) -> Result<Database, OpenError> {
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(|source| OpenError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    let backups_dir = dir.join("backups");
    let existed = path.exists();

    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    let supported = latest_version(migrator);
    let mut backup = None;
    if existed {
        let mut conn = SqliteConnection::connect_with(&options).await?;
        conn.ensure_migrations_table().await?;
        let applied: HashSet<i64> = conn
            .list_applied_migrations()
            .await?
            .into_iter()
            .map(|m| m.version)
            .collect();
        let found = applied.iter().copied().max().unwrap_or(0);
        if found > supported {
            return Err(OpenError::SchemaTooNew {
                found,
                supported,
                backups: backups_dir,
            });
        }
        let pending = migrator
            .iter()
            .any(|m| m.migration_type.is_up_migration() && !applied.contains(&m.version));
        if pending {
            std::fs::create_dir_all(&backups_dir).map_err(|source| OpenError::Io {
                path: backups_dir.clone(),
                source,
            })?;
            let user_version: i64 = sqlx::query_scalar("PRAGMA user_version")
                .fetch_one(&mut conn)
                .await?;
            let previous = decode_version(user_version);
            let target = backups_dir.join(format!("v{previous}-before-schema-{supported}.db"));
            if target.exists() {
                std::fs::remove_file(&target).map_err(|source| OpenError::Io {
                    path: target.clone(),
                    source,
                })?;
            }
            sqlx::query("VACUUM INTO ?")
                .bind(target.to_string_lossy().into_owned())
                .execute(&mut conn)
                .await?;
            tracing::info!(backup = %target.display(), "database backed up before migration");
            backup = Some(target);
        }
        conn.close().await?;
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;
    migrator.run(&pool).await?;
    // Remember which release last wrote the database, to name the next pre-migration backup.
    sqlx::query(&format!(
        "PRAGMA user_version = {}",
        encode_version(app_version)
    ))
    .execute(&pool)
    .await?;

    Ok(Database {
        pool,
        path: path.to_path_buf(),
        schema_version: supported,
        backup,
    })
}

/// `major.minor.patch` packed into SQLite's 32-bit `user_version`; 0 when unparsable.
fn encode_version(version: &str) -> i64 {
    let core = version.split(['-', '+']).next().unwrap_or_default();
    let parts: Vec<i64> = core.split('.').filter_map(|p| p.parse().ok()).collect();
    match parts[..] {
        [major, minor, patch] if major < 2_000 && minor < 1_000 && patch < 1_000 => {
            major * 1_000_000 + minor * 1_000 + patch
        }
        _ => 0,
    }
}

fn decode_version(encoded: i64) -> String {
    if encoded <= 0 {
        return "unknown".into();
    }
    format!(
        "{}.{}.{}",
        encoded / 1_000_000,
        encoded / 1_000 % 1_000,
        encoded % 1_000
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn migrator_with(dir: &Path, migrations: &[(&str, &str)]) -> Migrator {
        std::fs::create_dir_all(dir).unwrap();
        for (name, sql) in migrations {
            std::fs::write(dir.join(name), sql).unwrap();
        }
        Migrator::new(dir.to_path_buf()).await.unwrap()
    }

    const V1: (&str, &str) = ("0001_notes.sql", "CREATE TABLE notes (body TEXT NOT NULL);");
    const V2: (&str, &str) = ("0002_tags.sql", "ALTER TABLE notes ADD COLUMN tag TEXT;");

    #[tokio::test]
    async fn fresh_database_gets_every_migration_without_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Database::open(&tmp.path().join("korean.db"), "0.1.0")
            .await
            .unwrap();
        assert_eq!(db.schema_version(), latest_version(&MIGRATOR));
        assert!(db.backup().is_none());
        let tables: Vec<String> =
            sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table'")
                .fetch_all(db.pool())
                .await
                .unwrap();
        assert!(tables.contains(&"review_log".to_string()));
        assert!(!tmp.path().join("backups").exists());
    }

    #[tokio::test]
    async fn pending_migration_backs_up_the_previous_schema_with_its_data() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("korean.db");
        let old = migrator_with(&tmp.path().join("m1"), &[V1]).await;
        let db = open_with(&path, "0.5.0", &old).await.unwrap();
        sqlx::query("INSERT INTO notes (body) VALUES ('안녕')")
            .execute(db.pool())
            .await
            .unwrap();
        db.pool().close().await;

        let new = migrator_with(&tmp.path().join("m2"), &[V1, V2]).await;
        let db = open_with(&path, "0.6.0", &new).await.unwrap();
        let backup = db.backup().expect("backup written").to_path_buf();
        // Named after the release that last wrote the database, i.e. the one to roll back to.
        assert_eq!(backup, tmp.path().join("backups/v0.5.0-before-schema-2.db"));
        assert_eq!(db.schema_version(), 2);

        // The backup is the old schema, untouched, with the user's data.
        let old_db = open_with(&backup, "0.5.0", &old).await.unwrap();
        assert!(old_db.backup().is_none());
        let body: String = sqlx::query_scalar("SELECT body FROM notes")
            .fetch_one(old_db.pool())
            .await
            .unwrap();
        assert_eq!(body, "안녕");
        let has_tag: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('notes') WHERE name = 'tag'",
        )
        .fetch_one(old_db.pool())
        .await
        .unwrap();
        assert_eq!(has_tag, 0);
    }

    #[tokio::test]
    async fn up_to_date_database_is_not_backed_up_again() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("korean.db");
        let m = migrator_with(&tmp.path().join("m"), &[V1, V2]).await;
        open_with(&path, "0.6.0", &m)
            .await
            .unwrap()
            .pool()
            .close()
            .await;
        let db = open_with(&path, "0.6.0", &m).await.unwrap();
        assert!(db.backup().is_none());
    }

    #[tokio::test]
    async fn older_app_refuses_a_newer_schema() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("korean.db");
        let new = migrator_with(&tmp.path().join("m2"), &[V1, V2]).await;
        open_with(&path, "0.6.0", &new)
            .await
            .unwrap()
            .pool()
            .close()
            .await;

        let old = migrator_with(&tmp.path().join("m1"), &[V1]).await;
        match open_with(&path, "0.5.0", &old).await {
            Err(OpenError::SchemaTooNew {
                found, supported, ..
            }) => assert_eq!((found, supported), (2, 1)),
            other => panic!("expected SchemaTooNew, got {other:?}"),
        }
    }

    #[test]
    fn app_versions_round_trip_through_user_version() {
        assert_eq!(decode_version(encode_version("0.5.0")), "0.5.0");
        assert_eq!(
            decode_version(encode_version("12.34.567-beta.1")),
            "12.34.567"
        );
        assert_eq!(decode_version(encode_version("dev")), "unknown");
        assert_eq!(decode_version(0), "unknown");
    }

    #[test]
    fn released_migrations_are_immutable() {
        use sha2::{Digest, Sha256};
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let lock = std::fs::read_to_string(root.join("migrations.lock")).unwrap();
        let locked: Vec<(&str, &str)> = lock
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .map(|l| l.split_once(' ').expect("`<version> <sha256>`"))
            .collect();
        let mut files: Vec<_> = std::fs::read_dir(root.join("migrations"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        files.sort();
        assert_eq!(
            files.len(),
            locked.len(),
            "every migration must be listed in migrations.lock"
        );
        for (file, (version, hash)) in files.iter().zip(&locked) {
            let name = file.file_name().unwrap().to_string_lossy();
            assert!(
                name.starts_with(version),
                "{name} does not match lock entry {version}"
            );
            let digest = format!("{:x}", Sha256::digest(std::fs::read(file).unwrap()));
            assert_eq!(
                &digest, hash,
                "{name} changed after release; add a new migration instead"
            );
        }
    }
}
