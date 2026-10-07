//! StudioOne degraded surface bridge. Sub-agent implements MCU/MIDI/PTSL details.

use daw_contracts::testing::{degraded_surface_profile, MockHost};
use daw_contracts::{DawHost, HostId};

pub fn mock() -> MockHost {
    MockHost::new(degraded_surface_profile(
        HostId::StudioOne,
        "Studio One has no public project API",
    ))
}

pub fn connect() -> anyhow::Result<Box<dyn DawHost>> {
    Ok(Box::new(mock()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use daw_contracts::require_supported;

    #[tokio::test]
    async fn parameter_writes_are_unavailable() {
        let host = connect().unwrap();
        let err = require_supported(&host.capabilities(), "daw_set_parameter").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unsupported") || msg.contains("Studio One has no pu"),
            "{msg}"
        );
        assert_eq!(host.host_id(), HostId::StudioOne);
    }

    #[test]
    fn surface_xml_is_present() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bridges/studioone/StudioOneMCP.surface.xml");
        assert!(p.exists());
    }
}
