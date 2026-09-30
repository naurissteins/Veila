#![no_main]

use libfuzzer_sys::fuzz_target;
use veila_common::ipc::{IPC_MAX_LINE_BYTES, LineAccumulator, LineProgress};

fuzz_target!(|data: &[u8]| {
    if data.len() > 2 * IPC_MAX_LINE_BYTES {
        return;
    }
    let Some((&width, stream)) = data.split_first() else {
        return;
    };
    let mut accumulator = LineAccumulator::new();
    let mut expected = Vec::new();

    for chunk in stream.chunks(usize::from(width % 32) + 1) {
        let mut remaining = chunk;
        while !remaining.is_empty() {
            match accumulator.push_chunk(remaining, "fuzz") {
                Ok(LineProgress::Pending) => {
                    expected.extend_from_slice(remaining);
                    break;
                }
                Ok(LineProgress::Complete { line, consumed }) => {
                    assert!(consumed > 0 && consumed <= remaining.len());
                    assert_eq!(remaining[consumed - 1], b'\n');
                    assert!(line.len() <= IPC_MAX_LINE_BYTES);
                    expected.extend_from_slice(&remaining[..consumed - 1]);
                    assert_eq!(line.as_bytes(), expected);
                    expected.clear();
                    remaining = &remaining[consumed..];
                }
                Err(_) => return,
            }
        }
    }

    match accumulator.finish("fuzz") {
        Ok(None) => assert!(expected.is_empty()),
        Ok(Some(_)) => panic!("truncated line accepted"),
        Err(_) => assert!(!expected.is_empty()),
    }
});
