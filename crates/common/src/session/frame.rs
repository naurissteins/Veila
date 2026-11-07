use super::MAX_FRAME_BYTES;
use crate::ipc::{LineAccumulator, LineProgress};
use serde::{Serialize, de::DeserializeOwned};
use std::io::{self, BufRead, Write};

pub fn encode(value: &impl Serialize) -> io::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(io::Error::other("session frame exceeds limit"));
    }
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn write(writer: &mut impl Write, value: &impl Serialize) -> io::Result<()> {
    writer.write_all(&encode(value)?)
}

pub fn read<T: DeserializeOwned>(reader: &mut impl BufRead) -> io::Result<Option<T>> {
    let mut line = LineAccumulator::with_limit(MAX_FRAME_BYTES);
    loop {
        let bytes = reader.fill_buf()?;
        if bytes.is_empty() {
            line.finish("session frame").map_err(io::Error::other)?;
            return Ok(None);
        }
        match line
            .push_chunk(bytes, "session frame")
            .map_err(io::Error::other)?
        {
            LineProgress::Pending => {
                let len = bytes.len();
                reader.consume(len);
            }
            LineProgress::Complete { line, consumed } => {
                reader.consume(consumed);
                return decode(&line).map(Some);
            }
        }
    }
}

pub fn decode<T: DeserializeOwned>(line: &str) -> io::Result<T> {
    // do not retain serde diagnostics that could quote untrusted frame contents
    serde_json::from_str(line).map_err(|_| io::Error::other("invalid session frame"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AppConfig, session::SupervisorMessage};
    use std::io::Cursor;
    #[test]
    fn resolved_config_larger_than_legacy_frame_roundtrips_without_eating_next_frame() {
        let mut config = AppConfig::default();
        config.weather.location = Some("x".repeat(70_000));
        let message = SupervisorMessage::ReloadConfig {
            config: Box::new(config),
        };
        let mut bytes = encode(&message).expect("encode");
        bytes.extend(encode(&"next").expect("next"));
        let mut reader = Cursor::new(bytes);
        assert_eq!(
            read::<SupervisorMessage>(&mut reader).expect("read"),
            Some(message)
        );
        assert_eq!(
            read::<String>(&mut reader).expect("next"),
            Some("next".into())
        );
    }
    #[test]
    fn rejects_oversized_partial_invalid_and_unknown_frames() {
        assert!(encode(&"x".repeat(MAX_FRAME_BYTES)).is_err());
        assert!(read::<String>(&mut Cursor::new(vec![b'x'; MAX_FRAME_BYTES + 1])).is_err());
        for input in [
            b"\"partial\"".as_slice(),
            b"\xff\n",
            b"{bad}\n",
            b"{\"AuthInFlight\":\"secret-diagnostic\"}\n",
        ] {
            let error = read::<crate::session::CurtainMessage>(&mut Cursor::new(input))
                .expect_err("invalid");
            assert!(!error.to_string().contains("secret-diagnostic"));
        }
        assert!(
            decode::<crate::session::SupervisorMessage>(
                r#"{"ReloadConfig":{"config":{},"extra":1}}"#
            )
            .is_err()
        );
    }
    #[test]
    fn clean_eof_is_not_a_verdict_or_unlock() {
        assert_eq!(
            read::<crate::session::CurtainMessage>(&mut Cursor::new([])).expect("eof"),
            None
        );
    }
}
