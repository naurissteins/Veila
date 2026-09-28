use std::io::{BufRead, Write};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};
use veila_common::Secret;
use zeroize::Zeroizing;

pub(super) const MAX_FRAME_BYTES: usize = 4096;
pub(super) const MAX_PROMPT_CHARS: usize = 160;
pub(super) const MAX_CHALLENGES: u32 = 16;
pub(super) const MAX_NOTICES: u32 = 32;

#[derive(Debug, Serialize, Deserialize)]
pub(super) enum HelperRequest {
    Start { username: String, secret: Secret },
    Response { sequence: u32, secret: Secret },
    Cancel { sequence: u32 },
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) enum HelperMessage {
    Challenge {
        sequence: u32,
        echo: bool,
        text: String,
    },
    Notice {
        text: String,
    },
    Verdict {
        accepted: bool,
        message: Option<String>,
    },
}

pub(super) fn sanitize_text(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_control())
        .take(MAX_PROMPT_CHARS)
        .collect()
}

pub(super) fn valid_text(text: &str) -> bool {
    text.chars().count() <= MAX_PROMPT_CHARS && !text.chars().any(char::is_control)
}

pub(super) fn valid_secret(secret: &Secret) -> bool {
    secret.expose().len() <= 512 && !secret.expose().chars().any(char::is_control)
}

pub(super) fn read_sync<R: BufRead + ?Sized, T: DeserializeOwned>(
    reader: &mut R,
) -> Result<Option<T>> {
    let mut frame = Zeroizing::new(Vec::with_capacity(MAX_FRAME_BYTES + 1));
    loop {
        let available = reader
            .fill_buf()
            .context("failed to read PAM helper frame")?;
        if available.is_empty() {
            if frame.is_empty() {
                return Ok(None);
            }
            bail!("PAM helper frame ended before newline");
        }
        let length = available.iter().position(|byte| *byte == b'\n');
        let count = length.unwrap_or(available.len());
        if frame.len() + count > MAX_FRAME_BYTES {
            bail!("PAM helper frame exceeded limit");
        }
        frame.extend_from_slice(&available[..count]);
        reader.consume(count + usize::from(length.is_some()));
        if length.is_some() {
            return serde_json::from_slice(&frame)
                .context("invalid PAM helper frame")
                .map(Some);
        }
    }
}

pub(super) async fn read_async<R: AsyncBufRead + Unpin, T: DeserializeOwned>(
    reader: &mut R,
) -> Result<Option<T>> {
    let mut frame = Zeroizing::new(Vec::with_capacity(MAX_FRAME_BYTES + 1));
    loop {
        let available = reader
            .fill_buf()
            .await
            .context("failed to read PAM helper frame")?;
        if available.is_empty() {
            if frame.is_empty() {
                return Ok(None);
            }
            bail!("PAM helper frame ended before newline");
        }
        let length = available.iter().position(|byte| *byte == b'\n');
        let count = length.unwrap_or(available.len());
        if frame.len() + count > MAX_FRAME_BYTES {
            bail!("PAM helper frame exceeded limit");
        }
        frame.extend_from_slice(&available[..count]);
        reader.consume(count + usize::from(length.is_some()));
        if length.is_some() {
            return serde_json::from_slice(&frame)
                .context("invalid PAM helper frame")
                .map(Some);
        }
    }
}

pub(super) fn write_sync<W: Write + ?Sized, T: Serialize>(
    writer: &mut W,
    message: &T,
) -> Result<()> {
    let mut frame = Zeroizing::new(Vec::with_capacity(MAX_FRAME_BYTES + 1));
    serde_json::to_writer(&mut *frame, message).context("failed to encode PAM helper frame")?;
    if frame.len() > MAX_FRAME_BYTES {
        bail!("PAM helper frame exceeded limit");
    }
    frame.push(b'\n');
    writer
        .write_all(&frame)
        .context("failed to write PAM helper frame")?;
    writer.flush().context("failed to flush PAM helper frame")
}

pub(super) async fn write_async<W: AsyncWrite + Unpin, T: Serialize>(
    writer: &mut W,
    message: &T,
) -> Result<()> {
    let mut frame = Zeroizing::new(Vec::with_capacity(MAX_FRAME_BYTES + 1));
    serde_json::to_writer(&mut *frame, message).context("failed to encode PAM helper frame")?;
    if frame.len() > MAX_FRAME_BYTES {
        bail!("PAM helper frame exceeded limit");
    }
    frame.push(b'\n');
    writer
        .write_all(&frame)
        .await
        .context("failed to write PAM helper frame")?;
    writer
        .flush()
        .await
        .context("failed to flush PAM helper frame")
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn reads_consecutive_frames_without_discarding_tail() {
        let data = b"{\"Notice\":{\"text\":\"one\"}}\n{\"Notice\":{\"text\":\"two\"}}\n";
        let mut reader = Cursor::new(data);
        let first: HelperMessage = read_sync(&mut reader).expect("first").expect("frame");
        let second: HelperMessage = read_sync(&mut reader).expect("second").expect("frame");
        assert!(matches!(first, HelperMessage::Notice { text } if text == "one"));
        assert!(matches!(second, HelperMessage::Notice { text } if text == "two"));
    }

    #[test]
    fn rejects_oversized_frame() {
        let mut reader = Cursor::new(vec![b'a'; MAX_FRAME_BYTES + 1]);
        assert!(read_sync::<_, HelperMessage>(&mut reader).is_err());
    }

    #[test]
    fn rejects_truncated_frame() {
        let mut reader = Cursor::new(b"{}".to_vec());
        assert!(read_sync::<_, HelperMessage>(&mut reader).is_err());
    }
}
