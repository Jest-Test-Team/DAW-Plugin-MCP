#![no_main]

use daw_plugin::ipc::decode_frame;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = decode_frame(data);
});
