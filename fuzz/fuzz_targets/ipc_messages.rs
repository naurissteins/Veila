#![no_main]

use libfuzzer_sys::fuzz_target;
use veila_common::ipc::{CurtainControlMessage, CurtainStartupMessage};
use veila_common::session::frame::decode;

fuzz_target!(|data: &[u8]| {
    if data.len() > veila_common::session::MAX_FRAME_BYTES {
        return;
    }
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    let _ = decode::<veila_common::session::SupervisorMessage>(input);
    let _ = decode::<veila_common::session::CurtainMessage>(input);
    let _ = decode::<CurtainControlMessage>(input);
    let _ = decode::<CurtainStartupMessage>(input);
});
