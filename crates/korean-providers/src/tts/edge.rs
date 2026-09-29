//! Microsoft Edge "Read aloud" neural voices (free, no key), over the same WebSocket protocol the
//! browser uses. Mirrors the reference client `edge-tts` (github.com/rany2/edge-tts).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use sha2::{Digest, Sha256};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

use super::{Audio, BoxFuture, TtsError, TtsProvider};

const NAME: &str = "edge";
const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
const WSS_URL: &str = "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1";
const CHROMIUM_FULL_VERSION: &str = "143.0.3650.75";
const CHROMIUM_MAJOR_VERSION: &str = "143";
/// Seconds between the Windows file-time epoch (1601) and the Unix epoch.
const WIN_EPOCH_S: u64 = 11_644_473_600;
const TIMEOUT: Duration = Duration::from_secs(10);

pub struct EdgeTts {
    /// Short voice name, e.g. `ko-KR-SunHiNeural`.
    pub voice: String,
    /// Relative speaking rate, e.g. `-10%` (slightly slower for learners).
    pub rate: String,
}

impl Default for EdgeTts {
    fn default() -> Self {
        EdgeTts {
            voice: "ko-KR-SunHiNeural".into(),
            rate: "-10%".into(),
        }
    }
}

fn network(err: impl std::error::Error + Send + Sync + 'static) -> TtsError {
    TtsError::Network {
        provider: NAME,
        source: Box::new(err),
    }
}

fn protocol(message: impl Into<String>) -> TtsError {
    TtsError::Protocol {
        provider: NAME,
        message: message.into(),
    }
}

/// `Sec-MS-GEC`: SHA-256 of the Windows file time rounded down to 5 minutes + the client token.
fn sec_ms_gec(unix_seconds: u64) -> String {
    let mut ticks = unix_seconds + WIN_EPOCH_S;
    ticks -= ticks % 300;
    let ticks = u128::from(ticks) * 10_000_000;
    let digest = Sha256::digest(format!("{ticks}{TRUSTED_CLIENT_TOKEN}"));
    digest.iter().map(|b| format!("{b:02X}")).collect()
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
        .replace('"', "&quot;")
}

/// `ko-KR-SunHiNeural` → `Microsoft Server Speech Text to Speech Voice (ko-KR, SunHiNeural)`.
fn long_voice_name(short: &str) -> String {
    match short.rsplit_once('-') {
        Some((locale, name)) => {
            format!("Microsoft Server Speech Text to Speech Voice ({locale}, {name})")
        }
        None => short.to_string(),
    }
}

fn ssml(voice: &str, rate: &str, text: &str) -> String {
    format!(
        "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'>\
         <voice name='{}'><prosody pitch='+0Hz' rate='{rate}' volume='+0%'>{}</prosody></voice>\
         </speak>",
        long_voice_name(voice),
        escape_xml(text)
    )
}

/// JavaScript `Date.toString()` in UTC, as the browser sends it.
fn js_date(unix_seconds: u64) -> String {
    const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let days = (unix_seconds / 86_400) as i64;
    let secs = unix_seconds % 86_400;
    // Civil date from days since 1970-01-01 (H. Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{} {} {:02} {} {:02}:{:02}:{:02} GMT+0000 (Coordinated Universal Time)",
        DAYS[(days % 7) as usize],
        MONTHS[(month - 1) as usize],
        day,
        year,
        secs / 3_600,
        secs / 60 % 60,
        secs % 60
    )
}

/// Audio payload of a binary frame: `[u16 header length][headers][audio]`.
fn audio_payload(frame: &[u8]) -> Result<Option<&[u8]>, TtsError> {
    if frame.len() < 2 {
        return Err(protocol("binary frame without header length"));
    }
    let header_len = usize::from(u16::from_be_bytes([frame[0], frame[1]]));
    let body_start = 2 + header_len;
    if body_start > frame.len() {
        return Err(protocol("binary frame header longer than the frame"));
    }
    let headers = String::from_utf8_lossy(&frame[2..body_start]);
    if !headers.contains("Path:audio") {
        return Err(protocol("binary frame is not audio"));
    }
    let body = &frame[body_start..];
    Ok((!body.is_empty()).then_some(body))
}

fn random_hex() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

impl EdgeTts {
    async fn run(&self, text: &str) -> Result<Audio, TtsError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let url = format!(
            "{WSS_URL}?TrustedClientToken={TRUSTED_CLIENT_TOKEN}&ConnectionId={}\
             &Sec-MS-GEC={}&Sec-MS-GEC-Version=1-{CHROMIUM_FULL_VERSION}",
            random_hex(),
            sec_ms_gec(now)
        );
        let mut request = url.into_client_request().map_err(network)?;
        let user_agent = format!(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/{CHROMIUM_MAJOR_VERSION}.0.0.0 Safari/537.36 Edg/{CHROMIUM_MAJOR_VERSION}.0.0.0"
        );
        let cookie = format!("muid={};", random_hex().to_uppercase());
        let headers = request.headers_mut();
        for (name, value) in [
            ("Pragma", "no-cache".to_string()),
            ("Cache-Control", "no-cache".to_string()),
            (
                "Origin",
                "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold".to_string(),
            ),
            ("User-Agent", user_agent),
            ("Accept-Language", "en-US,en;q=0.9".to_string()),
            ("Cookie", cookie),
        ] {
            headers.insert(name, HeaderValue::from_str(&value).map_err(network)?);
        }

        let (mut ws, _) = tokio_tungstenite::connect_async(request)
            .await
            .map_err(network)?;
        let config = format!(
            "X-Timestamp:{}\r\nContent-Type:application/json; charset=utf-8\r\n\
             Path:speech.config\r\n\r\n\
             {{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\
             \"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"false\"}},\
             \"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"}}}}}}}}\r\n",
            js_date(now)
        );
        ws.send(Message::text(config)).await.map_err(network)?;
        let request_id = random_hex();
        let ssml_message = format!(
            "X-RequestId:{request_id}\r\nContent-Type:application/ssml+xml\r\n\
             X-Timestamp:{}Z\r\nPath:ssml\r\n\r\n{}",
            js_date(now),
            ssml(&self.voice, &self.rate, text)
        );
        ws.send(Message::text(ssml_message))
            .await
            .map_err(network)?;

        let mut audio = Vec::new();
        while let Some(message) = ws.next().await {
            match message.map_err(network)? {
                Message::Binary(frame) => {
                    if let Some(body) = audio_payload(&frame)? {
                        audio.extend_from_slice(body);
                    }
                }
                Message::Text(text) if text.contains("Path:turn.end") => break,
                Message::Close(frame) => {
                    return Err(protocol(format!("connection closed early: {frame:?}")));
                }
                _ => {}
            }
        }
        let _ = ws.close(None).await;
        if audio.is_empty() {
            return Err(protocol("no audio received"));
        }
        Ok(Audio {
            bytes: audio,
            mime: "audio/mpeg",
        })
    }
}

impl TtsProvider for EdgeTts {
    fn name(&self) -> &'static str {
        NAME
    }

    fn synthesize<'a>(&'a self, text: &'a str) -> BoxFuture<'a, Result<Audio, TtsError>> {
        Box::pin(async move {
            tokio::time::timeout(TIMEOUT, self.run(text))
                .await
                .map_err(|_| protocol("timed out"))?
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_matches_the_reference_client() {
        // Value computed with edge-tts' DRM.generate_sec_ms_gec for the same instant.
        assert_eq!(
            sec_ms_gec(1_790_000_000),
            "CA99F0B37F2EAC6F5D719AE4BA7C98978842B3F335D9F42979070A8DB149F0A5"
        );
        // Stable within a five-minute window.
        assert_eq!(sec_ms_gec(1_790_000_000), sec_ms_gec(1_790_000_000 + 50));
    }

    #[test]
    fn dates_are_formatted_like_javascript() {
        assert_eq!(
            js_date(0),
            "Thu Jan 01 1970 00:00:00 GMT+0000 (Coordinated Universal Time)"
        );
        assert_eq!(
            js_date(1_790_000_000),
            "Mon Sep 21 2026 14:13:20 GMT+0000 (Coordinated Universal Time)"
        );
    }

    #[test]
    fn ssml_names_the_voice_and_escapes_text() {
        let s = ssml("ko-KR-SunHiNeural", "-10%", "A&B <네>");
        assert!(s.contains("(ko-KR, SunHiNeural)"));
        assert!(s.contains("rate='-10%'"));
        assert!(s.contains("A&amp;B &lt;네&gt;"));
    }

    #[test]
    fn binary_frames_are_split_into_headers_and_audio() {
        let headers = b"X-RequestId:1\r\nContent-Type:audio/mpeg\r\nPath:audio\r\n";
        let mut frame = (headers.len() as u16).to_be_bytes().to_vec();
        frame.extend_from_slice(headers);
        frame.extend_from_slice(b"MP3");
        assert_eq!(audio_payload(&frame).unwrap(), Some(&b"MP3"[..]));

        let end = {
            let h = b"Path:audio\r\n";
            let mut f = (h.len() as u16).to_be_bytes().to_vec();
            f.extend_from_slice(h);
            f
        };
        assert_eq!(audio_payload(&end).unwrap(), None);
        assert!(audio_payload(&[0, 50, 1]).is_err());
    }

    /// Hits the real service: `cargo test -p korean-providers -- --ignored`.
    #[tokio::test]
    #[ignore = "network"]
    async fn live_synthesis_returns_mp3() {
        let audio = EdgeTts::default().synthesize("안녕하세요").await.unwrap();
        assert!(audio.bytes.len() > 1_000);
    }
}
