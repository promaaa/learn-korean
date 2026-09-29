//! Item photos. References are resolved once and cached in the database; bytes are downloaded by
//! Rust and served to the WebView through the `kimg://` protocol from memory (ADR-005).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use korean_db::images::StoredImage;
use korean_providers::images::{self, FirstMatch, Image, ImageError, ImageProvider};
use serde::Serialize;
use tauri::http::{Response, StatusCode};
use tauri::{AppHandle, Manager, Runtime, State};

use crate::state::AppState;

pub const SCHEME: &str = "kimg";
/// Queries that found nothing are retried after a week.
const RETRY_MISSING_MS: i64 = 7 * 24 * 3_600_000;
const BYTES_CACHE: usize = 32;

pub struct Images {
    search: FirstMatch,
    client: reqwest::Client,
    bytes: Mutex<VecDeque<(String, Arc<Image>)>>,
}

impl Images {
    pub fn new() -> Self {
        Images {
            search: images::default_provider(),
            client: images::http_client(),
            bytes: Mutex::new(VecDeque::new()),
        }
    }

    async fn bytes(&self, url: &str) -> Result<Arc<Image>, ImageError> {
        if let Some((_, image)) = self.cached(url) {
            return Ok(image);
        }
        let image = Arc::new(images::fetch(&self.client, url).await?);
        if let Ok(mut cache) = self.bytes.lock() {
            if cache.len() == BYTES_CACHE {
                cache.pop_front();
            }
            cache.push_back((url.to_string(), image.clone()));
        }
        Ok(image)
    }

    fn cached(&self, url: &str) -> Option<(String, Arc<Image>)> {
        self.bytes
            .lock()
            .ok()?
            .iter()
            .find(|(u, _)| u == url)
            .cloned()
    }
}

/// Photo reference for a query, searching the providers the first time.
async fn resolve(
    pool: &sqlx::SqlitePool,
    images: &Images,
    query: &str,
) -> Result<Option<StoredImage>, String> {
    let now = crate::session::now_ms();
    let cached = korean_db::images::get(pool, query)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(lookup) = cached
        && (lookup.found.is_some() || now - lookup.resolved_at < RETRY_MISSING_MS)
    {
        return Ok(lookup.found);
    }
    let found = images
        .search
        .search(query)
        .await
        .map_err(|e| e.to_string())?
        .map(|r| StoredImage {
            provider: r.provider,
            image_url: r.image_url,
            source_url: r.source_url,
            attribution: r.attribution,
        });
    korean_db::images::put(pool, query, found.as_ref(), now)
        .await
        .map_err(|e| e.to_string())?;
    Ok(found)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemImage {
    source_url: String,
    attribution: String,
}

/// An item's photo and its credit, `None` if the item has no photo. A reference whose image was
/// deleted upstream is forgotten and searched again once.
async fn photo(
    app: &AppState,
    images: &Images,
    item_id: &str,
) -> Result<Option<(StoredImage, Arc<Image>)>, String> {
    let pool = app.db()?.pool();
    let item = korean_db::content::item(pool, item_id)
        .await
        .map_err(|e| e.to_string())?;
    let Some(query) = item.and_then(|i| i.image) else {
        return Ok(None);
    };
    for attempt in 0..2 {
        let Some(found) = resolve(pool, images, &query).await? else {
            return Ok(None);
        };
        match images.bytes(&found.image_url).await {
            Ok(image) => return Ok(Some((found, image))),
            Err(err) if err.is_gone() && attempt == 0 => {
                log::info!("image for {query:?} is gone ({err}); searching again");
                korean_db::images::forget(pool, &query)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Err(err) => return Err(err.to_string()),
        }
    }
    Ok(None)
}

/// Credit for an item's photo, or `None` if it has none. The picture itself is loaded from
/// `kimg://localhost/<item id>` (already in the byte cache by then).
#[tauri::command]
pub async fn item_image(
    item_id: String,
    app: State<'_, AppState>,
    images: State<'_, Images>,
) -> Result<Option<ItemImage>, String> {
    Ok(photo(&app, &images, &item_id)
        .await?
        .map(|(f, _)| ItemImage {
            source_url: f.source_url,
            attribution: f.attribution,
        }))
}

fn respond(status: StatusCode, mime: &str, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header("Content-Type", mime)
        .header("Cache-Control", "no-store")
        .body(body)
        .expect("static response parts")
}

/// `kimg://localhost/<percent-encoded item id>` → image bytes.
pub async fn serve<R: Runtime>(app: &AppHandle<R>, path: &str) -> Response<Vec<u8>> {
    let item_id = percent_encoding::percent_decode_str(path.trim_start_matches('/'))
        .decode_utf8_lossy()
        .into_owned();
    let (state, images) = (app.state::<AppState>(), app.state::<Images>());
    match photo(&state, &images, &item_id).await {
        Ok(Some((_, image))) => respond(StatusCode::OK, &image.mime, image.bytes.clone()),
        Ok(None) => respond(StatusCode::NOT_FOUND, "text/plain", b"no image".to_vec()),
        Err(err) => {
            log::warn!("image for {item_id}: {err}");
            respond(StatusCode::NOT_FOUND, "text/plain", err.into_bytes())
        }
    }
}
