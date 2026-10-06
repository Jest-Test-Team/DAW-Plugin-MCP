use anyhow::anyhow;
use daw_contracts::DawHost;

pub fn select_host(name: &str) -> anyhow::Result<Box<dyn DawHost>> {
    match name.to_ascii_lowercase().as_str() {
        "reaper" => daw_bridge_reaper::connect(),
        "ableton" | "live" => daw_bridge_ableton::connect(),
        "bitwig" => daw_bridge_bitwig::connect(),
        "logic" => daw_bridge_logic::connect(),
        "studioone" | "studio_one" | "s1" => daw_bridge_studioone::connect(),
        "protools" | "pro_tools" | "pt" => daw_bridge_protools::connect(),
        "cubase" => daw_bridge_cubase::connect(),
        "flstudio" | "fl" => daw_bridge_flstudio::connect(),
        other => Err(anyhow!("unknown host {other}")),
    }
}
