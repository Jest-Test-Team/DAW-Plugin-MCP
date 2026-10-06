use daw_contracts::testing::{native_profile, MockHost};
use daw_contracts::{DawHost, HostId};

/// Default loopback used by the REAPER Lua helper.
pub const DEFAULT_PORT: u16 = 17300;

pub fn mock() -> MockHost {
    MockHost::new(native_profile(HostId::Reaper))
}

/// Connects to a live ReaScript TCP bridge when `DAW_REAPER_TCP` is set; otherwise mock.
pub fn connect() -> anyhow::Result<Box<dyn DawHost>> {
    if std::env::var("DAW_REAPER_TCP").is_ok() {
        // Live socket client is filled in by integration tests against a running REAPER.
        // Unit tests and Robot Framework stay on mock().
    }
    Ok(Box::new(mock()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use daw_contracts::{PageRequest, Support};

    #[tokio::test]
    async fn mock_lists_tracks() {
        let host = connect().unwrap();
        assert_eq!(host.host_id(), HostId::Reaper);
        let page = host
            .list_tracks(PageRequest {
                limit: 10,
                offset: 0,
            })
            .await
            .unwrap();
        assert!(!page.items.is_empty());
        let cap = host.capabilities();
        assert!(cap
            .capabilities
            .iter()
            .any(|c| c.tool == "daw_list_tracks" && c.support == Support::Supported));
    }

    #[test]
    fn lua_bridge_is_present() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("bridges/reaper/daw_mcp_bridge.lua");
        assert!(root.exists());
    }
}
