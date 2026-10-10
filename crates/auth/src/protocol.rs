use std::{
    io::{BufRead, Write},
    time::Duration,
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use veila_common::Secret;
use zeroize::Zeroizing;

pub const MAX_FRAME_BYTES: usize = 4096;
pub const MAX_PROMPT_CHARS: usize = 160;
pub const MAX_SECRET_BYTES: usize = 512;
pub const MAX_CHALLENGES: u32 = 16;
pub const MAX_NOTICES: u32 = 32;

pub const INITIAL_DEADLINE: Duration = Duration::from_secs(30);
pub const INTERACTIVE_DEADLINE: Duration = Duration::from_secs(120);
pub const PROMPT_DEADLINE: Duration = Duration::from_secs(60);
pub const WRITE_DEADLINE: Duration = Duration::from_secs(2);

pub const PARENT_PID_ENV: &str = "VEILA_PAM_PARENT_PID";

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("failed to read PAM helper frame")]
    Read(#[source] std::io::Error),
    #[error("failed to write PAM helper frame")]
    Write(#[source] std::io::Error),
    #[error("PAM helper frame exceeded limit")]
    Oversize,
    #[error("PAM helper frame ended before newline")]
    Truncated,
    // serde_json errors can quote frame contents, which may include secrets
    #[error("invalid PAM helper frame")]
    Decode,
    #[error("failed to encode PAM helper frame")]
    Encode,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HelperRequest {
    Start { username: String, secret: Secret },
    Response { sequence: u32, secret: Secret },
    Cancel { sequence: u32 },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HelperMessage {
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

pub(crate) fn sanitize_text(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_control())
        .take(MAX_PROMPT_CHARS)
        .collect()
}

pub fn valid_text(text: &str) -> bool {
    text.chars().count() <= MAX_PROMPT_CHARS && !text.chars().any(char::is_control)
}

pub fn valid_secret(secret: &Secret) -> bool {
    secret.expose().len() <= MAX_SECRET_BYTES && !secret.expose().chars().any(char::is_control)
}

/// Accumulates one newline-terminated frame in zeroized memory.
#[derive(Default)]
pub struct FrameBuffer {
    frame: Zeroizing<Vec<u8>>,
}

impl FrameBuffer {
    /// Copies `available` up to the next newline. Returns the bytes to consume and whether the frame is complete.
    pub fn push(&mut self, available: &[u8]) -> Result<(usize, bool), ProtocolError> {
        let newline = available.iter().position(|byte| *byte == b'\n');
        let count = newline.unwrap_or(available.len());
        if self.frame.len() + count > MAX_FRAME_BYTES {
            return Err(ProtocolError::Oversize);
        }
        self.frame.extend_from_slice(&available[..count]);
        Ok((count + usize::from(newline.is_some()), newline.is_some()))
    }

    pub fn decode<T: DeserializeOwned>(&self) -> Result<T, ProtocolError> {
        serde_json::from_slice(&self.frame).map_err(|_| ProtocolError::Decode)
    }

    pub fn end_of_input<T>(&self) -> Result<Option<T>, ProtocolError> {
        if self.frame.is_empty() {
            Ok(None)
        } else {
            Err(ProtocolError::Truncated)
        }
    }
}

pub fn encode_frame<T: Serialize>(message: &T) -> Result<Zeroizing<Vec<u8>>, ProtocolError> {
    let mut frame = Zeroizing::new(Vec::with_capacity(MAX_FRAME_BYTES + 1));
    serde_json::to_writer(&mut *frame, message).map_err(|_| ProtocolError::Encode)?;
    if frame.len() > MAX_FRAME_BYTES {
        return Err(ProtocolError::Oversize);
    }
    frame.push(b'\n');
    Ok(frame)
}

pub fn read_frame<R: BufRead + ?Sized, T: DeserializeOwned>(
    reader: &mut R,
) -> Result<Option<T>, ProtocolError> {
    let mut frame = FrameBuffer::default();
    loop {
        let available = reader.fill_buf().map_err(ProtocolError::Read)?;
        if available.is_empty() {
            return frame.end_of_input();
        }
        let (consumed, complete) = frame.push(available)?;
        reader.consume(consumed);
        if complete {
            return frame.decode().map(Some);
        }
    }
}

pub fn write_frame<W: Write + ?Sized, T: Serialize>(
    writer: &mut W,
    message: &T,
) -> Result<(), ProtocolError> {
    let frame = encode_frame(message)?;
    writer.write_all(&frame).map_err(ProtocolError::Write)?;
    writer.flush().map_err(ProtocolError::Write)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn reads_consecutive_frames_without_discarding_tail() {
        let data = b"{\"Notice\":{\"text\":\"one\"}}\n{\"Notice\":{\"text\":\"two\"}}\n";
        let mut reader = Cursor::new(data);
        let first: HelperMessage = read_frame(&mut reader).expect("first").expect("frame");
        let second: HelperMessage = read_frame(&mut reader).expect("second").expect("frame");
        assert!(matches!(first, HelperMessage::Notice { text } if text == "one"));
        assert!(matches!(second, HelperMessage::Notice { text } if text == "two"));
        assert!(
            read_frame::<_, HelperMessage>(&mut reader)
                .expect("eof")
                .is_none()
        );
    }

    #[test]
    fn rejects_oversized_frame() {
        let mut reader = Cursor::new(vec![b'a'; MAX_FRAME_BYTES + 1]);
        assert!(matches!(
            read_frame::<_, HelperMessage>(&mut reader),
            Err(ProtocolError::Oversize)
        ));
    }

    #[test]
    fn rejects_truncated_frame() {
        let mut reader = Cursor::new(b"{}".to_vec());
        assert!(matches!(
            read_frame::<_, HelperMessage>(&mut reader),
            Err(ProtocolError::Truncated)
        ));
    }

    #[test]
    fn decode_errors_do_not_quote_frame_contents() {
        let mut reader =
            Cursor::new(b"{\"Response\":{\"sequence\":\"hunter2\",\"secret\":\"x\"}}\n".to_vec());
        let error = read_frame::<_, HelperRequest>(&mut reader).expect_err("invalid frame");
        assert!(!format!("{error:?} {error}").contains("hunter2"));
    }

    #[test]
    fn refuses_to_encode_oversized_frame() {
        let text = "a".repeat(MAX_FRAME_BYTES);
        assert!(matches!(
            encode_frame(&HelperMessage::Notice { text }),
            Err(ProtocolError::Oversize)
        ));
    }
}
