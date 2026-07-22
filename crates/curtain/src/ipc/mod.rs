pub(crate) mod auth;
pub(crate) mod control;

use std::io::BufRead;

use anyhow::{Context, Result};
use veila_common::ipc::{LineAccumulator, LineProgress};

/// Reads one newline-delimited IPC message, leaving any bytes after the newline in the reader
pub(crate) fn read_bounded_line<R: BufRead>(reader: &mut R, label: &str) -> Result<Option<String>> {
    let mut accumulator = LineAccumulator::new();

    loop {
        let buffer = reader
            .fill_buf()
            .with_context(|| format!("failed to read {label}"))?;
        if buffer.is_empty() {
            return accumulator.finish(label).map_err(Into::into);
        }

        match accumulator.push_chunk(buffer, label)? {
            LineProgress::Complete { line, consumed } => {
                reader.consume(consumed);
                return Ok(Some(line));
            }
            LineProgress::Pending => {
                let consumed = buffer.len();
                reader.consume(consumed);
            }
        }
    }
}
