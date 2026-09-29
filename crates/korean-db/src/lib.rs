//! SQLite persistence for learn-korean.
//!
//! [`Database::open`] applies the embedded migrations. When an existing database has pending
//! migrations, a consistent copy is written first to
//! `backups/v<previous-version>-before-schema-<n>.db` so that reinstalling the previous release and
//! restoring that file is always possible.

mod open;

pub use open::{Database, OpenError};
pub use sqlx::SqlitePool;
