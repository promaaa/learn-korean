//! Speaking Korean: synthesis through the provider chain, playback on a dedicated audio thread.
//! Audio stays in memory; nothing is written to disk.

use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};

use korean_providers::tts::{self, Audio, Cached, Fallback};

pub struct Speech {
    tts: Cached<Fallback>,
    player: Sender<Arc<Audio>>,
    /// Latest request: a slower, older synthesis must not cut off the newer line.
    latest: AtomicU64,
}

impl Speech {
    pub fn new() -> Self {
        Speech {
            tts: tts::default_provider(),
            player: spawn_player(),
            latest: AtomicU64::new(0),
        }
    }
}

/// The output stream is not `Send` on every platform, so one thread owns it and plays whatever
/// it receives, cutting off the previous sound.
fn spawn_player() -> Sender<Arc<Audio>> {
    let (tx, rx) = mpsc::channel::<Arc<Audio>>();
    std::thread::Builder::new()
        .name("audio".into())
        .spawn(move || {
            let mut sink = match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(sink) => sink,
                Err(err) => {
                    log::error!("no audio output device: {err}");
                    // Keep draining so senders never block or error.
                    for _ in rx {}
                    return;
                }
            };
            sink.log_on_drop(false);
            let player = rodio::Player::connect_new(sink.mixer());
            for audio in rx {
                player.clear();
                match rodio::Decoder::try_from(Cursor::new(audio.bytes.clone())) {
                    Ok(source) => {
                        player.append(source);
                        player.play();
                    }
                    Err(err) => log::warn!("cannot decode {} audio: {err}", audio.mime),
                }
            }
        })
        .expect("spawn audio thread");
    tx
}

/// Synthesizes (or reuses) speech for `text` and plays it, unless a newer request came in while
/// it was being synthesized.
#[tauri::command]
pub async fn speak(text: String, speech: tauri::State<'_, Speech>) -> Result<(), String> {
    let request = speech.latest.fetch_add(1, Ordering::SeqCst) + 1;
    let audio = speech.tts.get(&text).await.map_err(|e| e.to_string())?;
    if speech.latest.load(Ordering::SeqCst) != request {
        return Ok(());
    }
    speech
        .player
        .send(audio)
        .map_err(|_| "audio thread stopped".to_string())
}
