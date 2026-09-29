//! Bundled content tables: seeding from packs and loading items.

use std::hash::{DefaultHasher, Hash, Hasher};

use korean_core::content::{Item, Pack};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SeedReport {
    pub packs_updated: usize,
    pub packs_unchanged: usize,
    pub packs_removed: usize,
}

/// Makes the content tables mirror `packs`. Packs whose content hash is unchanged are skipped;
/// packs no longer bundled are removed. User data is untouched.
pub async fn seed(pool: &SqlitePool, packs: &[Pack]) -> sqlx::Result<SeedReport> {
    let mut report = SeedReport::default();
    let mut tx = pool.begin().await?;

    let existing: Vec<(String, String)> = sqlx::query_as("SELECT id, hash FROM content_packs")
        .fetch_all(&mut *tx)
        .await?;
    for (id, _) in &existing {
        if !packs.iter().any(|p| &p.id == id) {
            sqlx::query("DELETE FROM content_packs WHERE id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            report.packs_removed += 1;
        }
    }

    for (position, pack) in packs.iter().enumerate() {
        let json = serde_json::to_string(pack).expect("packs serialize");
        let hash = content_hash(&json);
        let position = position as i64;
        if existing.iter().any(|(id, h)| id == &pack.id && h == &hash) {
            sqlx::query("UPDATE content_packs SET position = ? WHERE id = ?")
                .bind(position)
                .bind(&pack.id)
                .execute(&mut *tx)
                .await?;
            report.packs_unchanged += 1;
            continue;
        }
        sqlx::query("DELETE FROM content_packs WHERE id = ?")
            .bind(&pack.id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO content_packs \
             (id, version, title, description, unlock_level, position, hash) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&pack.id)
        .bind(pack.version)
        .bind(&pack.title)
        .bind(&pack.description)
        .bind(pack.unlock_level)
        .bind(position)
        .bind(&hash)
        .execute(&mut *tx)
        .await?;
        for (item_position, item) in pack.items.iter().enumerate() {
            sqlx::query(
                "INSERT INTO content_items (id, pack_id, position, kind, korean, english, data) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&item.id)
            .bind(&pack.id)
            .bind(item_position as i64)
            .bind(kind_str(item))
            .bind(&item.korean)
            .bind(&item.english)
            .bind(serde_json::to_string(item).expect("items serialize"))
            .execute(&mut *tx)
            .await?;
        }
        report.packs_updated += 1;
    }
    tx.commit().await?;
    Ok(report)
}

fn kind_str(item: &Item) -> &'static str {
    match item.kind {
        korean_core::content::ItemKind::Sentence => "sentence",
        korean_core::content::ItemKind::Word => "word",
    }
}

/// Stable within a build: only used to detect that a bundled pack changed since the last seed.
fn content_hash(json: &str) -> String {
    let mut hasher = DefaultHasher::new();
    json.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// A pack's metadata, without its items.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct PackInfo {
    pub id: String,
    pub title: String,
    pub unlock_level: i64,
    pub items: i64,
}

pub async fn packs(pool: &SqlitePool) -> sqlx::Result<Vec<PackInfo>> {
    sqlx::query_as(
        "SELECT p.id, p.title, p.unlock_level, COUNT(i.id) AS items \
         FROM content_packs p LEFT JOIN content_items i ON i.pack_id = p.id \
         GROUP BY p.id ORDER BY p.position",
    )
    .fetch_all(pool)
    .await
}

/// Items of packs unlocked at `level`, in pack then item order.
pub async fn unlocked_items(pool: &SqlitePool, level: u32) -> sqlx::Result<Vec<Item>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT i.data FROM content_items i JOIN content_packs p ON p.id = i.pack_id \
         WHERE p.unlock_level <= ? ORDER BY p.position, i.position",
    )
    .bind(level)
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|(data,)| serde_json::from_str(&data).map_err(|e| sqlx::Error::Decode(Box::new(e))))
        .collect()
}

/// One bundled item by id.
pub async fn item(pool: &SqlitePool, id: &str) -> sqlx::Result<Option<Item>> {
    let data: Option<String> = sqlx::query_scalar("SELECT data FROM content_items WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    data.map(|d| serde_json::from_str(&d).map_err(|e| sqlx::Error::Decode(Box::new(e))))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use korean_core::content::{ItemKind, Register};

    fn item(id: &str, korean: &str) -> Item {
        Item {
            id: id.into(),
            kind: ItemKind::Sentence,
            korean: korean.into(),
            english: id.into(),
            context: "test".into(),
            register: Register::Polite,
            note: None,
            lexemes: vec![],
            distractors: vec!["a".into(), "b".into(), "c".into()],
            replies: None,
            chunks: None,
            image: None,
        }
    }

    fn pack(id: &str, level: u32, items: Vec<Item>) -> Pack {
        Pack {
            id: id.into(),
            version: 1,
            title: id.to_uppercase(),
            description: String::new(),
            unlock_level: level,
            items,
        }
    }

    #[tokio::test]
    async fn seeding_is_idempotent_and_follows_changes() {
        let db = Database::in_memory().await.unwrap();
        let pool = db.pool();
        let v1 = vec![
            pack(
                "a",
                1,
                vec![item("a/one", "하나예요."), item("a/two", "둘이에요.")],
            ),
            pack("b", 3, vec![item("b/one", "셋이에요.")]),
        ];
        let r = seed(pool, &v1).await.unwrap();
        assert_eq!((r.packs_updated, r.packs_unchanged), (2, 0));
        let r = seed(pool, &v1).await.unwrap();
        assert_eq!((r.packs_updated, r.packs_unchanged), (0, 2));

        // Edit pack a, drop pack b.
        let v2 = vec![pack("a", 1, vec![item("a/one", "하나예요!")])];
        let r = seed(pool, &v2).await.unwrap();
        assert_eq!((r.packs_updated, r.packs_removed), (1, 1));
        let items = unlocked_items(pool, 10).await.unwrap();
        assert_eq!(items, v2[0].items);
        assert_eq!(
            super::item(pool, "a/one").await.unwrap().as_ref(),
            Some(&v2[0].items[0])
        );
        assert_eq!(super::item(pool, "b/one").await.unwrap(), None);
    }

    #[tokio::test]
    async fn locked_packs_are_excluded_and_order_is_preserved() {
        let db = Database::in_memory().await.unwrap();
        let pool = db.pool();
        seed(
            pool,
            &[
                pack("a", 1, vec![item("a/2", "둘."), item("a/1", "하나.")]),
                pack("b", 3, vec![item("b/1", "셋.")]),
            ],
        )
        .await
        .unwrap();
        let ids = |items: Vec<Item>| items.into_iter().map(|i| i.id).collect::<Vec<_>>();
        assert_eq!(ids(unlocked_items(pool, 1).await.unwrap()), ["a/2", "a/1"]);
        assert_eq!(
            ids(unlocked_items(pool, 3).await.unwrap()),
            ["a/2", "a/1", "b/1"]
        );
        let info = packs(pool).await.unwrap();
        assert_eq!(info[1].items, 1);
        assert_eq!(info[1].unlock_level, 3);
    }

    #[tokio::test]
    async fn reseeding_never_touches_user_history() {
        let db = Database::in_memory().await.unwrap();
        let pool = db.pool();
        seed(pool, &[pack("a", 1, vec![item("a/one", "하나.")])])
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO review_log (item_id, skill, correct, rating, elapsed_ms, answered_at) \
             VALUES ('a/one', 'listening', 1, 3, 1200, 0)",
        )
        .execute(pool)
        .await
        .unwrap();
        seed(pool, &[]).await.unwrap();
        let kept: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM review_log")
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(kept, 1);
    }
}
