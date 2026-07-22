use zeroize::Zeroize;

use crate::error::{Result, VeilaError};

/// Maximum bytes accepted for a single newline-delimited IPC message
pub const IPC_MAX_LINE_BYTES: usize = 64 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum LineProgress {
    Complete { line: String, consumed: usize },
    Pending,
}

/// Accumulates newline-delimited IPC input while enforcing a byte ceiling
#[derive(Debug)]
pub struct LineAccumulator {
    buffer: Vec<u8>,
    max_bytes: usize,
}

impl LineAccumulator {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Pre-sizes the buffer so an expected message never reallocates
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            max_bytes: IPC_MAX_LINE_BYTES,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn push_chunk(&mut self, chunk: &[u8], label: &str) -> Result<LineProgress> {
        let line_end = chunk.iter().position(|byte| *byte == b'\n');
        let take = line_end.unwrap_or(chunk.len());

        if self.buffer.len() + take > self.max_bytes {
            self.buffer.zeroize();
            return Err(invalid_data(format!(
                "{label} exceeds {} bytes",
                self.max_bytes
            )));
        }

        self.buffer.extend_from_slice(&chunk[..take]);

        let Some(_) = line_end else {
            return Ok(LineProgress::Pending);
        };

        let consumed = take + 1;
        let bytes = std::mem::take(&mut self.buffer);
        match String::from_utf8(bytes) {
            Ok(line) => Ok(LineProgress::Complete { line, consumed }),
            Err(error) => {
                let mut bytes = error.into_bytes();
                bytes.zeroize();
                Err(invalid_data(format!("{label} is not UTF-8")))
            }
        }
    }

    /// Reports an unexpected end of input, distinguishing a clean close from a truncated message
    pub fn finish(&mut self, label: &str) -> Result<Option<String>> {
        if self.buffer.is_empty() {
            return Ok(None);
        }

        self.buffer.zeroize();
        Err(invalid_data(format!("{label} ended before newline")))
    }
}

impl Default for LineAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LineAccumulator {
    fn drop(&mut self) {
        self.buffer.zeroize();
    }
}

fn invalid_data(message: String) -> VeilaError {
    VeilaError::IpcFraming(message)
}

#[cfg(test)]
mod tests {
    use super::{IPC_MAX_LINE_BYTES, LineAccumulator, LineProgress};

    #[test]
    fn reports_bytes_consumed_so_buffered_readers_keep_the_remainder() {
        let mut accumulator = LineAccumulator::new();

        let progress = accumulator
            .push_chunk(b"first\nsecond\n", "test")
            .expect("first line");

        // Six bytes: "first" plus the newline
        assert_eq!(
            progress,
            LineProgress::Complete {
                line: String::from("first"),
                consumed: 6,
            }
        );
    }

    #[test]
    fn accumulates_across_chunk_boundaries() {
        let mut accumulator = LineAccumulator::new();

        assert_eq!(
            accumulator.push_chunk(b"par", "test").expect("pending"),
            LineProgress::Pending
        );
        let progress = accumulator.push_chunk(b"tial\n", "test").expect("complete");

        assert_eq!(
            progress,
            LineProgress::Complete {
                line: String::from("partial"),
                consumed: 5,
            }
        );
    }

    #[test]
    fn rejects_lines_past_the_ceiling_before_allocating_them() {
        let mut accumulator = LineAccumulator::new();
        let oversized = vec![b'a'; IPC_MAX_LINE_BYTES + 1];

        let error = accumulator
            .push_chunk(&oversized, "test")
            .expect_err("oversized line should be rejected");

        assert!(error.to_string().contains("exceeds"));
    }

    #[test]
    fn rejects_non_utf8_input() {
        let mut accumulator = LineAccumulator::new();

        let error = accumulator
            .push_chunk(&[0xff, 0xfe, b'\n'], "test")
            .expect_err("invalid utf-8 should be rejected");

        assert!(error.to_string().contains("not UTF-8"));
    }

    #[test]
    fn distinguishes_a_clean_close_from_a_truncated_message() {
        let mut clean = LineAccumulator::new();
        assert_eq!(clean.finish("test").expect("clean close"), None);

        let mut truncated = LineAccumulator::new();
        truncated.push_chunk(b"partial", "test").expect("pending");
        let error = truncated
            .finish("test")
            .expect_err("truncated message should be rejected");

        assert!(error.to_string().contains("ended before newline"));
    }
}
