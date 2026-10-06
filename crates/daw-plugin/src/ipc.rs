//! Message-thread IPC client. Never call from `process()`.

use std::path::PathBuf;

pub fn default_socket_path() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"\\.\pipe\daw-mcp")
    } else {
        std::env::temp_dir().join("daw-mcp.sock")
    }
}

pub fn encode_frame(body: &[u8]) -> Vec<u8> {
    let len = body.len() as u32;
    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(body);
    out
}

pub fn decode_frame(buf: &[u8]) -> Option<&[u8]> {
    if buf.len() < 4 {
        return None;
    }
    let len = u32::from_le_bytes(buf[0..4].try_into().ok()?) as usize;
    if buf.len() < 4 + len {
        return None;
    }
    Some(&buf[4..4 + len])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip() {
        let payload = b"{\"method\":\"plugin/hello\"}";
        let framed = encode_frame(payload);
        assert_eq!(decode_frame(&framed), Some(payload.as_slice()));
    }
}
