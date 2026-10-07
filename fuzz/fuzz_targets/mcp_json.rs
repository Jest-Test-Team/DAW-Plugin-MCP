#![no_main]

use daw_mcp::parse_jsonrpc;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_jsonrpc(s);
    }
});
