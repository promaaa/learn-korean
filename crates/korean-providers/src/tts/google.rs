//! Google Translate's speech endpoint (free, no key). Lower quality than Edge; used as fallback.

use std::time::Duration;

use super::{Audio, BoxFuture, TtsError, TtsProvider};

const NAME: &str = "google";
const ENDPOINT: &str = "https://translate.google.com/translate_tts";
/// The endpoint rejects longer inputs.
const MAX_CHARS: usize = 200;

pub struct GoogleTranslateTts {
    client: reqwest::Client,
}

impl Default for GoogleTranslateTts {
    fn default() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) learn-korean")
            .build()
            .expect("static client configuration");
        GoogleTranslateTts { client }
    }
}

fn truncated(text: &str) -> &str {
    match text.char_indices().nth(MAX_CHARS) {
        Some((end, _)) => &text[..end],
        None => text,
    }
}

impl TtsProvider for GoogleTranslateTts {
    fn name(&self) -> &'static str {
        NAME
    }

    fn synthesize<'a>(&'a self, text: &'a str) -> BoxFuture<'a, Result<Audio, TtsError>> {
        Box::pin(async move {
            let network = |e: reqwest::Error| TtsError::Network {
                provider: NAME,
                source: Box::new(e),
            };
            let response = self
                .client
                .get(ENDPOINT)
                .query(&[
                    ("ie", "UTF-8"),
                    ("tl", "ko"),
                    ("client", "tw-ob"),
                    ("q", truncated(text)),
                ])
                .send()
                .await
                .map_err(network)?
                .error_for_status()
                .map_err(network)?;
            let bytes = response.bytes().await.map_err(network)?;
            if bytes.is_empty() {
                return Err(TtsError::Protocol {
                    provider: NAME,
                    message: "empty body".into(),
                });
            }
            Ok(Audio {
                bytes: bytes.to_vec(),
                mime: "audio/mpeg",
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_text_is_cut_on_a_char_boundary() {
        let long = "가".repeat(250);
        assert_eq!(truncated(&long).chars().count(), MAX_CHARS);
        assert_eq!(truncated("안녕"), "안녕");
    }

    #[tokio::test]
    #[ignore = "network"]
    async fn live_synthesis_returns_mp3() {
        let audio = GoogleTranslateTts::default()
            .synthesize("안녕하세요")
            .await
            .unwrap();
        assert!(audio.bytes.len() > 1_000);
    }
}
