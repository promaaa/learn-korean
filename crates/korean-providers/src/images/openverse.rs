//! Openverse (openverse.org): openly licensed photos aggregated from Flickr, Wikimedia and others.

use serde::Deserialize;

use super::{ImageError, ImageProvider, ImageRef, fits_card};
use crate::BoxFuture;

const NAME: &str = "openverse";
const ENDPOINT: &str = "https://api.openverse.org/v1/images/";

pub struct Openverse {
    client: reqwest::Client,
}

impl Openverse {
    pub fn new(client: reqwest::Client) -> Self {
        Openverse { client }
    }
}

#[derive(Deserialize)]
struct Page {
    results: Vec<Hit>,
}

#[derive(Deserialize)]
struct Hit {
    url: Option<String>,
    foreign_landing_url: Option<String>,
    creator: Option<String>,
    license: String,
    license_version: Option<String>,
    source: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
}

/// `by-nc-sa` + `2.0` → `CC BY-NC-SA 2.0`; `cc0` → `CC0`; `pdm` → `Public domain`.
fn license_label(license: &str, version: Option<&str>) -> String {
    match license {
        "cc0" => "CC0".into(),
        "pdm" => "Public domain".into(),
        other => {
            let name = format!("CC {}", other.to_uppercase());
            match version {
                Some(v) if !v.is_empty() => format!("{name} {v}"),
                _ => name,
            }
        }
    }
}

fn pick(page: Page) -> Option<ImageRef> {
    page.results.into_iter().find_map(|hit| {
        let fits = matches!((hit.width, hit.height), (Some(w), Some(h)) if fits_card(w, h));
        let url = hit.url.filter(|u| u.starts_with("https://"))?;
        if !fits {
            return None;
        }
        let mut credit = vec![];
        if let Some(creator) = hit.creator.filter(|c| !c.trim().is_empty()) {
            credit.push(creator);
        }
        credit.push(license_label(&hit.license, hit.license_version.as_deref()));
        credit.push(hit.source.unwrap_or_else(|| NAME.into()));
        Some(ImageRef {
            provider: NAME.into(),
            source_url: hit.foreign_landing_url.unwrap_or_else(|| url.clone()),
            image_url: url,
            attribution: credit.join(" · "),
        })
    })
}

impl ImageProvider for Openverse {
    fn name(&self) -> &'static str {
        NAME
    }

    fn search<'a>(&'a self, query: &'a str) -> BoxFuture<'a, Result<Option<ImageRef>, ImageError>> {
        Box::pin(async move {
            let network = |source| ImageError::Network {
                provider: NAME,
                source,
            };
            let page: Page = self
                .client
                .get(ENDPOINT)
                .query(&[
                    ("q", query),
                    ("page_size", "12"),
                    ("mature", "false"),
                    ("category", "photograph"),
                ])
                .send()
                .await
                .map_err(network)?
                .error_for_status()
                .map_err(network)?
                .json()
                .await
                .map_err(network)?;
            Ok(pick(page))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_first_card_shaped_https_photo_with_credit() {
        let page: Page = serde_json::from_str(
            r#"{"results":[
              {"url":"https://x/pano.jpg","license":"by","width":4000,"height":900},
              {"url":"http://x/insecure.jpg","license":"by","width":1024,"height":768},
              {"url":"https://live.staticflickr.com/1_b.jpg",
               "foreign_landing_url":"https://www.flickr.com/photos/1",
               "creator":"nurpax","license":"by-nc-nd","license_version":"2.0",
               "source":"flickr","width":1024,"height":681}
            ]}"#,
        )
        .unwrap();
        let found = pick(page).unwrap();
        assert_eq!(found.image_url, "https://live.staticflickr.com/1_b.jpg");
        assert_eq!(found.source_url, "https://www.flickr.com/photos/1");
        assert_eq!(found.attribution, "nurpax · CC BY-NC-ND 2.0 · flickr");
    }

    #[test]
    fn license_labels() {
        assert_eq!(license_label("cc0", Some("1.0")), "CC0");
        assert_eq!(license_label("pdm", None), "Public domain");
        assert_eq!(license_label("by-sa", Some("4.0")), "CC BY-SA 4.0");
    }

    #[tokio::test]
    #[ignore = "network"]
    async fn live_search_finds_a_photo() {
        let found = Openverse::new(super::super::http_client())
            .search("bicycle")
            .await
            .unwrap();
        assert!(found.is_some());
    }
}
