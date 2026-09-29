//! Text-to-speech behind one trait. Nothing outside this module knows which service speaks.

mod edge;
mod google;

use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

pub use edge::EdgeTts;
pub use google::GoogleTranslateTts;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Encoded audio, playable as is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Audio {
    pub bytes: Vec<u8>,
    pub mime: &'static str,
}

#[derive(Debug, thiserror::Error)]
pub enum TtsError {
    #[error("{provider}: network error: {source}")]
    Network {
        provider: &'static str,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("{provider}: unexpected response: {message}")]
    Protocol {
        provider: &'static str,
        message: String,
    },
    #[error("text is empty")]
    EmptyText,
    #[error("no text-to-speech provider configured")]
    NoProvider,
}

pub trait TtsProvider: Send + Sync {
    fn name(&self) -> &'static str;
    /// Korean speech for `text`.
    fn synthesize<'a>(&'a self, text: &'a str) -> BoxFuture<'a, Result<Audio, TtsError>>;
}

/// Tries providers in order and returns the first success.
pub struct Fallback {
    providers: Vec<Box<dyn TtsProvider>>,
}

impl Fallback {
    pub fn new(providers: Vec<Box<dyn TtsProvider>>) -> Self {
        Fallback { providers }
    }
}

impl TtsProvider for Fallback {
    fn name(&self) -> &'static str {
        "fallback"
    }

    fn synthesize<'a>(&'a self, text: &'a str) -> BoxFuture<'a, Result<Audio, TtsError>> {
        Box::pin(async move {
            let mut last = TtsError::NoProvider;
            for provider in &self.providers {
                match provider.synthesize(text).await {
                    Ok(audio) => return Ok(audio),
                    Err(err) => {
                        tracing::warn!(provider = provider.name(), %err, "tts provider failed");
                        last = err;
                    }
                }
            }
            Err(last)
        })
    }
}

/// Keeps the most recent syntheses in memory (never on disk).
pub struct Cached<P> {
    inner: P,
    capacity: usize,
    entries: Mutex<VecDeque<(String, Arc<Audio>)>>,
}

impl<P: TtsProvider> Cached<P> {
    pub fn new(inner: P, capacity: usize) -> Self {
        Cached {
            inner,
            capacity: capacity.max(1),
            entries: Mutex::new(VecDeque::new()),
        }
    }

    pub async fn get(&self, text: &str) -> Result<Arc<Audio>, TtsError> {
        if text.trim().is_empty() {
            return Err(TtsError::EmptyText);
        }
        {
            let mut entries = self.entries.lock().expect("tts cache poisoned");
            if let Some(pos) = entries.iter().position(|(t, _)| t == text) {
                let entry = entries.remove(pos).expect("position is valid");
                let audio = entry.1.clone();
                entries.push_back(entry);
                return Ok(audio);
            }
        }
        let audio = Arc::new(self.inner.synthesize(text).await?);
        let mut entries = self.entries.lock().expect("tts cache poisoned");
        if entries.len() == self.capacity {
            entries.pop_front();
        }
        entries.push_back((text.to_string(), audio.clone()));
        Ok(audio)
    }
}

/// Default chain: Microsoft Edge neural voice, then Google Translate's voice.
pub fn default_provider() -> Cached<Fallback> {
    Cached::new(
        Fallback::new(vec![
            Box::new(EdgeTts::default()),
            Box::new(GoogleTranslateTts::default()),
        ]),
        128,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fake {
        name: &'static str,
        fail: bool,
        calls: Arc<AtomicUsize>,
    }

    impl TtsProvider for Fake {
        fn name(&self) -> &'static str {
            self.name
        }
        fn synthesize<'a>(&'a self, text: &'a str) -> BoxFuture<'a, Result<Audio, TtsError>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let result = if self.fail {
                Err(TtsError::Protocol {
                    provider: self.name,
                    message: "down".into(),
                })
            } else {
                Ok(Audio {
                    bytes: format!("{}:{text}", self.name).into_bytes(),
                    mime: "audio/mpeg",
                })
            };
            Box::pin(async move { result })
        }
    }

    fn fake(name: &'static str, fail: bool) -> (Box<dyn TtsProvider>, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let provider = Fake {
            name,
            fail,
            calls: calls.clone(),
        };
        (Box::new(provider), calls)
    }

    #[tokio::test]
    async fn fallback_uses_the_first_provider_that_works() {
        let (a, a_calls) = fake("a", true);
        let (b, _) = fake("b", false);
        let (c, c_calls) = fake("c", false);
        let chain = Fallback::new(vec![a, b, c]);
        let audio = chain.synthesize("안녕").await.unwrap();
        assert_eq!(audio.bytes, "b:안녕".as_bytes());
        assert_eq!(a_calls.load(Ordering::SeqCst), 1);
        assert_eq!(c_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn fallback_reports_the_last_error_when_all_fail() {
        let (a, _) = fake("a", true);
        let (b, _) = fake("b", true);
        let err = Fallback::new(vec![a, b]).synthesize("x").await.unwrap_err();
        assert!(err.to_string().starts_with("b:"), "{err}");
    }

    #[tokio::test]
    async fn cache_hits_skip_the_provider_and_evict_the_least_recent() {
        let calls = Arc::new(AtomicUsize::new(0));
        let provider = Fake {
            name: "p",
            fail: false,
            calls: calls.clone(),
        };
        let cache = Cached::new(provider, 2);
        cache.get("하나").await.unwrap();
        cache.get("둘").await.unwrap();
        cache.get("하나").await.unwrap(); // hit, refreshes 하나
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        cache.get("셋").await.unwrap(); // evicts 둘
        cache.get("하나").await.unwrap(); // still cached
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        cache.get("둘").await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 4);
        assert!(matches!(cache.get("  ").await, Err(TtsError::EmptyText)));
    }
}
