#![no_main]

use libfuzzer_sys::fuzz_target;
use veila_common::ipc::{
    ClientMessage, CurtainControlMessage, CurtainControlResponse, CurtainInitialSnapshots,
    CurtainStartupMessage, DaemonControlMessage, DaemonControlResponse, DaemonMessage,
    IPC_MAX_LINE_BYTES, decode_message,
};

fuzz_target!(|data: &[u8]| {
    if data.len() > IPC_MAX_LINE_BYTES {
        return;
    }
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    let _ = decode_message::<ClientMessage>(input);
    let _ = decode_message::<DaemonMessage>(input);
    let _ = decode_message::<CurtainControlMessage>(input);
    let _ = decode_message::<CurtainControlResponse>(input);
    let _ = decode_message::<CurtainStartupMessage>(input);
    let _ = decode_message::<CurtainInitialSnapshots>(input);
    let _ = decode_message::<DaemonControlMessage>(input);
    let _ = decode_message::<DaemonControlResponse>(input);
});
