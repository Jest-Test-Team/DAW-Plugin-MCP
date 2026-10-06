//! ProTools bridge. Sub-agent replaces mock() with the native protocol while keeping this API.

use daw_contracts::testing::{native_profile, MockHost};
use daw_contracts::{DawHost, HostId};

pub fn mock() -> MockHost {
    MockHost::new(native_profile(HostId::ProTools))
}

pub fn connect() -> anyhow::Result<Box<dyn DawHost>> {
    Ok(Box::new(mock()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use daw_contracts::{PageRequest, Support};

    #[tokio::test]
    async fn mock_lists_tracks() {
        let host = connect().unwrap();
        assert_eq!(host.host_id(), HostId::ProTools);
        let page = host.list_tracks(PageRequest { limit: 10, offset: 0 }).await.unwrap();
        assert!(!page.items.is_empty());
        let cap = host.capabilities();
        assert!(cap.capabilities.iter().any(|c| c.tool == "daw_list_tracks" && c.support == Support::Supported));
    }

    #[test]
    fn ptsl_notes_are_present() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bridges/protools/ptsl_notes.md");
        assert!(p.exists());
    }
}
