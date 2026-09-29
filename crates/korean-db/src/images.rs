//! Cached photo references (metadata only).

use sqlx::SqlitePool;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct StoredImage {
    pub provider: String,
    pub image_url: String,
    pub source_url: String,
    pub attribution: String,
}

/// A previous lookup for a query: `found` is `None` when no photo was suitable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lookup {
    pub found: Option<StoredImage>,
    pub resolved_at: i64,
}

type Row = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    i64,
);

pub async fn get(pool: &SqlitePool, query: &str) -> sqlx::Result<Option<Lookup>> {
    let row: Option<Row> = sqlx::query_as(
        "SELECT provider, image_url, source_url, attribution, resolved_at \
         FROM image_refs WHERE query = ?",
    )
    .bind(query)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(
        |(provider, image_url, source_url, attribution, resolved_at)| {
            let found = match (provider, image_url, source_url, attribution) {
                (Some(provider), Some(image_url), Some(source_url), Some(attribution)) => {
                    Some(StoredImage {
                        provider,
                        image_url,
                        source_url,
                        attribution,
                    })
                }
                _ => None,
            };
            Lookup { found, resolved_at }
        },
    ))
}

pub async fn put(
    pool: &SqlitePool,
    query: &str,
    found: Option<&StoredImage>,
    now_ms: i64,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO image_refs (query, provider, image_url, source_url, attribution, resolved_at) \
         VALUES (?, ?, ?, ?, ?, ?) \
         ON CONFLICT (query) DO UPDATE SET provider = excluded.provider, \
         image_url = excluded.image_url, source_url = excluded.source_url, \
         attribution = excluded.attribution, resolved_at = excluded.resolved_at",
    )
    .bind(query)
    .bind(found.map(|f| &f.provider))
    .bind(found.map(|f| &f.image_url))
    .bind(found.map(|f| &f.source_url))
    .bind(found.map(|f| &f.attribution))
    .bind(now_ms)
    .execute(pool)
    .await?;
    Ok(())
}

/// Drops a lookup so the next display searches again (e.g. the photo was deleted upstream).
pub async fn forget(pool: &SqlitePool, query: &str) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM image_refs WHERE query = ?")
        .bind(query)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    #[tokio::test]
    async fn found_and_not_found_lookups_round_trip() {
        let db = Database::in_memory().await.unwrap();
        let pool = db.pool();
        assert_eq!(get(pool, "cat").await.unwrap(), None);

        put(pool, "cat", None, 10).await.unwrap();
        assert_eq!(
            get(pool, "cat").await.unwrap(),
            Some(Lookup {
                found: None,
                resolved_at: 10
            })
        );

        let image = StoredImage {
            provider: "openverse".into(),
            image_url: "https://x/cat.jpg".into(),
            source_url: "https://x/cat".into(),
            attribution: "someone · CC BY 2.0 · flickr".into(),
        };
        put(pool, "cat", Some(&image), 20).await.unwrap();
        assert_eq!(get(pool, "cat").await.unwrap().unwrap().found, Some(image));

        forget(pool, "cat").await.unwrap();
        assert_eq!(get(pool, "cat").await.unwrap(), None);
    }
}
