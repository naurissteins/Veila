#![no_main]

use libfuzzer_sys::fuzz_target;
use veila_common::ipc::{
    CurtainControlMessage, CurtainInitialSnapshots, CurtainStartupMessage, decode_message,
};

fuzz_target!(|data: &[u8]| {
    if data.len() > veila_common::session::MAX_FRAME_BYTES {
        return;
    }
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    let _ = decode_message::<veila_common::session::SupervisorMessage>(input);
    let _ = decode_message::<veila_common::session::CurtainMessage>(input);
    let _ = decode_message::<CurtainControlMessage>(input);
    let _ = decode_message::<CurtainStartupMessage>(input);
    let _ = decode_message::<CurtainInitialSnapshots>(input);
});
