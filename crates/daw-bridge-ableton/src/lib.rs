use daw_contracts::testing::{native_profile, MockHost};
use daw_contracts::{DawHost, HostId};

pub const DEFAULT_PORT: u16 = 17301;

pub fn mock() -> MockHost {
    MockHost::new(native_profile(HostId::Ableton))
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
        assert_eq!(host.host_id(), HostId::Ableton);
        let page = host
            .list_tracks(PageRequest {
                limit: 10,
                offset: 0,
            })
            .await
            .unwrap();
        assert!(!page.items.is_empty());
        assert!(host
            .capabilities()
            .capabilities
            .iter()
            .any(|c| c.tool == "daw_list_tracks" && c.support == Support::Supported));
    }

    #[test]
    fn remote_script_is_present() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("bridges/ableton/DawMcp/__init__.py");
        assert!(root.exists());
    }
}
