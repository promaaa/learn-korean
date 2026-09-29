//! Wikimedia Commons search, used when Openverse has nothing.

use std::collections::HashMap;

use serde::Deserialize;

use super::{ImageError, ImageProvider, ImageRef, fits_card};
use crate::BoxFuture;

const NAME: &str = "wikimedia";
const ENDPOINT: &str = "https://commons.wikimedia.org/w/api.php";

pub struct WikimediaCommons {
    client: reqwest::Client,
}

impl WikimediaCommons {
    pub fn new(client: reqwest::Client) -> Self {
        WikimediaCommons { client }
    }
}

#[derive(Deserialize)]
struct Response {
    query: Option<Query>,
}

#[derive(Deserialize)]
struct Query {
    pages: HashMap<String, Page>,
}

#[derive(Deserialize)]
struct Page {
    index: u32,
    #[serde(default)]
    imageinfo: Vec<Info>,
}

#[derive(Deserialize)]
struct Info {
    thumburl: Option<String>,
    descriptionurl: String,
    mime: String,
    width: u32,
    height: u32,
    #[serde(default)]
    extmetadata: HashMap<String, Meta>,
}

#[derive(Deserialize)]
struct Meta {
    value: String,
}

/// Commons metadata is HTML (`<a href=…>Name</a>`); keep the text.
fn strip_html(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => text.push(c),
            _ => {}
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn pick(response: Response) -> Option<ImageRef> {
    let mut pages: Vec<Page> = response.query?.pages.into_values().collect();
    pages.sort_by_key(|p| p.index);
    pages.into_iter().find_map(|page| {
        let info = page.imageinfo.into_iter().next()?;
        if !matches!(info.mime.as_str(), "image/jpeg" | "image/png")
            || !fits_card(info.width, info.height)
        {
            return None;
        }
        let meta = |key: &str| {
            info.extmetadata
                .get(key)
                .map(|m| strip_html(&m.value))
                .filter(|v| !v.is_empty())
        };
        let mut credit: Vec<String> = [meta("Artist"), meta("LicenseShortName")]
            .into_iter()
            .flatten()
            .collect();
        credit.push("Wikimedia Commons".into());
        Some(ImageRef {
            provider: NAME.into(),
            image_url: info.thumburl?,
            source_url: info.descriptionurl,
            attribution: credit.join(" · "),
        })
    })
}

impl ImageProvider for WikimediaCommons {
    fn name(&self) -> &'static str {
        NAME
    }

    fn search<'a>(&'a self, query: &'a str) -> BoxFuture<'a, Result<Option<ImageRef>, ImageError>> {
        Box::pin(async move {
            let network = |source| ImageError::Network {
                provider: NAME,
                source,
            };
            let search = format!("{query} filetype:bitmap");
            let response: Response = self
                .client
                .get(ENDPOINT)
                .query(&[
                    ("action", "query"),
                    ("format", "json"),
                    ("generator", "search"),
                    ("gsrsearch", search.as_str()),
                    ("gsrnamespace", "6"),
                    ("gsrlimit", "8"),
                    ("prop", "imageinfo"),
                    ("iiprop", "url|extmetadata|mime|size"),
                    ("iiurlwidth", "960"),
                    ("iiextmetadatafilter", "Artist|LicenseShortName"),
                ])
                .send()
                .await
                .map_err(network)?
                .error_for_status()
                .map_err(network)?
                .json()
                .await
                .map_err(network)?;
            Ok(pick(response))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_by_search_rank_and_builds_a_plain_text_credit() {
        let response: Response = serde_json::from_str(
            r#"{"query":{"pages":{
              "9":{"index":3,"imageinfo":[{"thumburl":"https://t/3.jpg","descriptionurl":"https://c/3","mime":"image/jpeg","width":2000,"height":1500}]},
              "7":{"index":1,"imageinfo":[{"thumburl":"https://t/1.svg.png","descriptionurl":"https://c/1","mime":"image/svg+xml","width":2000,"height":1500}]},
              "8":{"index":2,"imageinfo":[{"thumburl":"https://t/2.jpg","descriptionurl":"https://c/2","mime":"image/jpeg","width":5141,"height":3427,
                  "extmetadata":{"Artist":{"value":"<a rel=\"nofollow\" href=\"https://flickr.com/x\">Chloe  Lim</a>"},"LicenseShortName":{"value":"CC BY 2.0"}}}]}
            }}}"#,
        )
        .unwrap();
        let found = pick(response).unwrap();
        assert_eq!(found.image_url, "https://t/2.jpg");
        assert_eq!(found.source_url, "https://c/2");
        assert_eq!(
            found.attribution,
            "Chloe Lim · CC BY 2.0 · Wikimedia Commons"
        );
    }

    #[test]
    fn no_results_means_none() {
        let response: Response = serde_json::from_str(r#"{"batchcomplete":""}"#).unwrap();
        assert!(pick(response).is_none());
    }

    #[tokio::test]
    #[ignore = "network"]
    async fn live_search_finds_a_photo() {
        let found = WikimediaCommons::new(super::super::http_client())
            .search("bibimbap")
            .await
            .unwrap();
        assert!(found.is_some());
    }
}
