//! Photos for learning items, found on open-licensed image services. Only references
//! (URLs + attribution) are meant to be stored; bytes are fetched on demand and kept in memory.

mod openverse;
mod wikimedia;

use std::time::Duration;

use serde::Serialize;

pub use openverse::Openverse;
pub use wikimedia::WikimediaCommons;

use crate::BoxFuture;

/// Where a photo lives and who made it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageRef {
    pub provider: String,
    pub image_url: String,
    pub source_url: String,
    /// Short credit line, e.g. `Chloe Lim · CC BY 2.0 · Wikimedia Commons`.
    pub attribution: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub bytes: Vec<u8>,
    pub mime: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("{provider}: network error: {source}")]
    Network {
        provider: &'static str,
        source: reqwest::Error,
    },
    #[error("{provider}: unexpected response: {message}")]
    Protocol {
        provider: &'static str,
        message: String,
    },
}

impl ImageError {
    /// The server says the image no longer exists (404, 410) or is refused (403): the stored
    /// reference is dead and a new search is needed.
    pub fn is_gone(&self) -> bool {
        match self {
            ImageError::Network { source, .. } => source
                .status()
                .is_some_and(|s| matches!(s.as_u16(), 403 | 404 | 410)),
            ImageError::Protocol { .. } => false,
        }
    }
}

pub trait ImageProvider: Send + Sync {
    fn name(&self) -> &'static str;
    /// Best photo for an English query, `None` when nothing suitable exists.
    fn search<'a>(&'a self, query: &'a str) -> BoxFuture<'a, Result<Option<ImageRef>, ImageError>>;
}

/// Tries providers in order. A provider that errors is skipped; one that finds nothing too.
pub struct FirstMatch {
    providers: Vec<Box<dyn ImageProvider>>,
}

impl FirstMatch {
    pub fn new(providers: Vec<Box<dyn ImageProvider>>) -> Self {
        FirstMatch { providers }
    }
}

impl ImageProvider for FirstMatch {
    fn name(&self) -> &'static str {
        "first-match"
    }

    fn search<'a>(&'a self, query: &'a str) -> BoxFuture<'a, Result<Option<ImageRef>, ImageError>> {
        Box::pin(async move {
            let mut last_error = None;
            for provider in &self.providers {
                match provider.search(query).await {
                    Ok(Some(found)) => return Ok(Some(found)),
                    Ok(None) => {}
                    Err(err) => {
                        tracing::warn!(provider = provider.name(), %err, "image search failed");
                        last_error = Some(err);
                    }
                }
            }
            // Report "nothing found" only when every provider actually answered.
            match last_error {
                Some(err) => Err(err),
                None => Ok(None),
            }
        })
    }
}

pub fn default_provider() -> FirstMatch {
    let client = http_client();
    FirstMatch::new(vec![
        Box::new(Openverse::new(client.clone())),
        Box::new(WikimediaCommons::new(client)),
    ])
}

/// Wikimedia requires an identifying user agent.
pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent("learn-korean/0.x (https://github.com/promaaa/learn-korean)")
        .build()
        .expect("static client configuration")
}

/// Larger downloads are refused.
pub const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;

/// Downloads an image into memory.
pub async fn fetch(client: &reqwest::Client, url: &str) -> Result<Image, ImageError> {
    const NAME: &str = "fetch";
    let network = |source| ImageError::Network {
        provider: NAME,
        source,
    };
    let response = client
        .get(url)
        .send()
        .await
        .map_err(network)?
        .error_for_status()
        .map_err(network)?;
    let mime = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    if !mime.starts_with("image/") {
        return Err(ImageError::Protocol {
            provider: NAME,
            message: format!("not an image: {mime:?}"),
        });
    }
    if response
        .content_length()
        .is_some_and(|len| len as usize > MAX_IMAGE_BYTES)
    {
        return Err(ImageError::Protocol {
            provider: NAME,
            message: "image too large".into(),
        });
    }
    let bytes = response.bytes().await.map_err(network)?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(ImageError::Protocol {
            provider: NAME,
            message: "image too large".into(),
        });
    }
    Ok(Image {
        bytes: bytes.to_vec(),
        mime,
    })
}

/// Photos that fit a card: big enough, not a panorama or a tall strip.
fn fits_card(width: u32, height: u32) -> bool {
    width >= 400 && height >= 250 && {
        let ratio = f64::from(width) / f64::from(height);
        (0.8..=2.2).contains(&ratio)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(&'static str, Result<Option<&'static str>, ()>);

    impl ImageProvider for Fake {
        fn name(&self) -> &'static str {
            self.0
        }
        fn search<'a>(
            &'a self,
            _query: &'a str,
        ) -> BoxFuture<'a, Result<Option<ImageRef>, ImageError>> {
            let result = match self.1 {
                Ok(found) => Ok(found.map(|url| ImageRef {
                    provider: self.0.into(),
                    image_url: url.into(),
                    source_url: url.into(),
                    attribution: String::new(),
                })),
                Err(()) => Err(ImageError::Protocol {
                    provider: self.0,
                    message: "down".into(),
                }),
            };
            Box::pin(async move { result })
        }
    }

    #[tokio::test]
    async fn first_match_skips_errors_and_empty_results() {
        let chain = FirstMatch::new(vec![
            Box::new(Fake("a", Err(()))),
            Box::new(Fake("b", Ok(None))),
            Box::new(Fake("c", Ok(Some("https://c/1.jpg")))),
        ]);
        let found = chain.search("cat").await.unwrap().unwrap();
        assert_eq!(found.provider, "c");
    }

    #[tokio::test]
    async fn nothing_found_is_only_reported_when_every_provider_answered() {
        let none = FirstMatch::new(vec![Box::new(Fake("a", Ok(None)))]);
        assert_eq!(none.search("x").await.unwrap(), None);
        let broken = FirstMatch::new(vec![
            Box::new(Fake("a", Ok(None))),
            Box::new(Fake("b", Err(()))),
        ]);
        assert!(broken.search("x").await.is_err());
    }

    #[test]
    fn card_shaped_photos_only() {
        assert!(fits_card(1024, 681));
        assert!(fits_card(800, 900));
        assert!(!fits_card(300, 200), "too small");
        assert!(!fits_card(3000, 800), "panorama");
        assert!(!fits_card(600, 1400), "tall strip");
    }
}
