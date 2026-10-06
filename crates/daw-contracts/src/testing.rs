//! In-memory host used by unit tests and Robot Framework fixtures.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::{
    CapabilityProfile, DawError, DawHost, HealthIssue, HostId, MidiNote, Page, PageRequest,
    PluginInstance, SessionSummary, Track, TrackKind,
};

#[derive(Clone)]
pub struct MockHost {
    inner: Arc<Mutex<MockState>>,
}

struct MockState {
    profile: CapabilityProfile,
    summary: SessionSummary,
    tracks: Vec<Track>,
    plugins: Vec<PluginInstance>,
    midi: Vec<MidiNote>,
    health: Vec<HealthIssue>,
}

impl MockHost {
    pub fn new(profile: CapabilityProfile) -> Self {
        let host = profile.host;
        let tracks = vec![
            Track {
                id: "trk-1".into(),
                name: "Kick".into(),
                index: 0,
                kind: TrackKind::Audio,
                mute: false,
                solo: false,
                armed: false,
                color: None,
            },
            Track {
                id: "trk-2".into(),
                name: "Bass".into(),
                index: 1,
                kind: TrackKind::Midi,
                mute: false,
                solo: false,
                armed: false,
                color: None,
            },
        ];
        Self {
            inner: Arc::new(Mutex::new(MockState {
                summary: SessionSummary {
                    host,
                    name: Some("mock-session".into()),
                    sample_rate: Some(48_000),
                    tempo_bpm: Some(120.0),
                    track_count: 2,
                    marker_count: 0,
                },
                profile,
                tracks,
                plugins: vec![PluginInstance {
                    id: "fx-1".into(),
                    track_id: "trk-1".into(),
                    name: "EQ".into(),
                    index: 0,
                    bypassed: false,
                }],
                midi: vec![MidiNote {
                    pitch: 60,
                    velocity: 100,
                    start_beats: 0.0,
                    duration_beats: 1.0,
                    channel: 0,
                }],
                health: vec![],
            })),
        }
    }

    pub fn with_health(self, health: Vec<HealthIssue>) -> Self {
        self.inner.lock().expect("mock mutex").health = health;
        self
    }
}

#[async_trait]
impl DawHost for MockHost {
    fn host_id(&self) -> HostId {
        self.inner.lock().expect("mock mutex").profile.host
    }

    fn capabilities(&self) -> CapabilityProfile {
        self.inner.lock().expect("mock mutex").profile.clone()
    }

    async fn session_summary(&self) -> Result<SessionSummary, DawError> {
        Ok(self.inner.lock().expect("mock mutex").summary.clone())
    }

    async fn list_tracks(&self, page: PageRequest) -> Result<Page<Track>, DawError> {
        Ok(Page::slice(self.inner.lock().expect("mock mutex").tracks.clone(), page))
    }

    async fn get_track(&self, id: &str) -> Result<Track, DawError> {
        self.inner
            .lock()
            .expect("mock mutex")
            .tracks
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or_else(|| DawError::NotFound(id.into()))
    }

    async fn list_plugins(&self, track_id: &str) -> Result<Vec<PluginInstance>, DawError> {
        Ok(self
            .inner
            .lock()
            .expect("mock mutex")
            .plugins
            .iter()
            .filter(|p| p.track_id == track_id)
            .cloned()
            .collect())
    }

    async fn midi_selection(&self) -> Result<Vec<MidiNote>, DawError> {
        Ok(self.inner.lock().expect("mock mutex").midi.clone())
    }

    async fn health_check(&self) -> Result<Vec<HealthIssue>, DawError> {
        Ok(self.inner.lock().expect("mock mutex").health.clone())
    }

    async fn write_midi(&self, notes: Vec<MidiNote>) -> Result<(), DawError> {
        self.inner.lock().expect("mock mutex").midi = notes;
        Ok(())
    }

    async fn set_parameter(&self, _plugin_id: &str, _name: &str, _value: f64) -> Result<(), DawError> {
        Ok(())
    }

    async fn set_track_mute(&self, track_id: &str, mute: bool) -> Result<(), DawError> {
        let mut g = self.inner.lock().expect("mock mutex");
        let t = g
            .tracks
            .iter_mut()
            .find(|t| t.id == track_id)
            .ok_or_else(|| DawError::NotFound(track_id.into()))?;
        t.mute = mute;
        Ok(())
    }

    async fn rename_track(&self, track_id: &str, name: &str) -> Result<(), DawError> {
        let mut g = self.inner.lock().expect("mock mutex");
        let t = g
            .tracks
            .iter_mut()
            .find(|t| t.id == track_id)
            .ok_or_else(|| DawError::NotFound(track_id.into()))?;
        t.name = name.into();
        Ok(())
    }

    async fn set_playhead_beats(&self, _beats: f64) -> Result<(), DawError> {
        Ok(())
    }
}

pub fn native_profile(host: HostId) -> CapabilityProfile {
    use crate::{cap, ControlLayer, RiskLevel, Support};
    let tools = [
        ("daw_get_capabilities", Support::Supported, ControlLayer::L3Native, RiskLevel::Read),
        ("daw_get_session_summary", Support::Supported, ControlLayer::L3Native, RiskLevel::Read),
        ("daw_list_tracks", Support::Supported, ControlLayer::L3Native, RiskLevel::Read),
        ("daw_get_track", Support::Supported, ControlLayer::L3Native, RiskLevel::Read),
        ("daw_list_plugins", Support::Supported, ControlLayer::L3Native, RiskLevel::Read),
        ("daw_get_midi_selection", Support::Supported, ControlLayer::L3Native, RiskLevel::Read),
        ("daw_health_check", Support::Supported, ControlLayer::L3Native, RiskLevel::Read),
        ("daw_write_midi", Support::Supported, ControlLayer::L3Native, RiskLevel::Reversible),
        ("daw_set_parameter", Support::Supported, ControlLayer::L3Native, RiskLevel::Reversible),
        ("daw_set_track_mute", Support::Supported, ControlLayer::L3Native, RiskLevel::Reversible),
        ("daw_rename_track", Support::Supported, ControlLayer::L3Native, RiskLevel::Reversible),
        ("daw_set_playhead", Support::Supported, ControlLayer::L3Native, RiskLevel::Reversible),
    ];
    CapabilityProfile {
        host,
        layers: vec![ControlLayer::L3Native, ControlLayer::L0Plugin],
        capabilities: tools
            .into_iter()
            .map(|(t, s, l, r)| cap(t, s, l, r, None, None))
            .collect(),
    }
}

pub fn degraded_surface_profile(host: HostId, unavailable_reason: &str) -> CapabilityProfile {
    use crate::{cap, ControlLayer, RiskLevel, Support};
    CapabilityProfile {
        host,
        layers: vec![ControlLayer::L2Surface, ControlLayer::L0Plugin, ControlLayer::L1Probe],
        capabilities: vec![
            cap("daw_get_capabilities", Support::Supported, ControlLayer::L2Surface, RiskLevel::Read, None, None),
            cap("daw_get_session_summary", Support::Degraded, ControlLayer::L2Surface, RiskLevel::Read, Some("limited host metadata"), None),
            cap("daw_list_tracks", Support::Degraded, ControlLayer::L2Surface, RiskLevel::Read, Some("mixer bank only"), None),
            cap("daw_set_track_mute", Support::Supported, ControlLayer::L2Surface, RiskLevel::Reversible, None, None),
            cap("daw_set_playhead", Support::Supported, ControlLayer::L2Surface, RiskLevel::Reversible, None, None),
            cap(
                "daw_set_parameter",
                Support::Unavailable,
                ControlLayer::L3Native,
                RiskLevel::Reversible,
                Some(unavailable_reason),
                Some("Insert Probe plugin or use a native-API DAW"),
            ),
            cap(
                "daw_list_plugins",
                Support::Unavailable,
                ControlLayer::L3Native,
                RiskLevel::Read,
                Some(unavailable_reason),
                Some("Insert Probe plugin for per-track DSP"),
            ),
        ],
    }
}
