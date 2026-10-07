//! Shared DAW Agent contracts: capability profiles, session types, risk levels, host trait.

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const DEFAULT_PAGE_LIMIT: u32 = 32;
pub const MAX_PAGE_LIMIT: u32 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HostId {
    Unknown,
    Plugin,
    Reaper,
    Ableton,
    Bitwig,
    Logic,
    StudioOne,
    ProTools,
    Cubase,
    FlStudio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ControlLayer {
    L0Plugin,
    L1Probe,
    L2Surface,
    L3Native,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Read,
    Reversible,
    Structural,
    Destructive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Support {
    Supported,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Capability {
    pub tool: String,
    pub support: Support,
    pub layer: ControlLayer,
    pub risk: RiskLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_step: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CapabilityProfile {
    pub host: HostId,
    pub layers: Vec<ControlLayer>,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PageRequest {
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

fn default_limit() -> u32 {
    DEFAULT_PAGE_LIMIT
}

impl PageRequest {
    pub fn clamped(self) -> Self {
        Self {
            limit: self.limit.clamp(1, MAX_PAGE_LIMIT),
            offset: self.offset,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub limit: u32,
    pub offset: u32,
    pub total_count: u32,
    pub has_more: bool,
    pub next_offset: Option<u32>,
}

impl<T> Page<T> {
    pub fn slice(all: Vec<T>, req: PageRequest) -> Self {
        let req = req.clamped();
        let total_count = all.len() as u32;
        let start = req.offset.min(total_count) as usize;
        let end = (start + req.limit as usize).min(all.len());
        let items: Vec<T> = all
            .into_iter()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect();
        let next = req.offset + req.limit;
        let has_more = next < total_count;
        Self {
            items,
            limit: req.limit,
            offset: req.offset,
            total_count,
            has_more,
            next_offset: has_more.then_some(next),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub index: u32,
    pub kind: TrackKind,
    pub mute: bool,
    pub solo: bool,
    pub armed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Audio,
    Midi,
    Bus,
    Master,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PluginInstance {
    pub id: String,
    pub track_id: String,
    pub name: String,
    pub index: u32,
    pub bypassed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MidiNote {
    pub pitch: u8,
    pub velocity: u8,
    pub start_beats: f64,
    pub duration_beats: f64,
    pub channel: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SessionSummary {
    pub host: HostId,
    pub name: Option<String>,
    pub sample_rate: Option<u32>,
    pub tempo_bpm: Option<f64>,
    pub track_count: u32,
    pub marker_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HealthIssue {
    pub code: String,
    pub severity: HealthSeverity,
    pub message: String,
    pub confidence: f32,
    pub layer: ControlLayer,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HealthSeverity {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpectrumResult {
    pub bands_hz: Vec<f32>,
    pub magnitudes_db: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LoudnessResult {
    pub lufs_integrated: f32,
    pub true_peak_dbtp: f32,
    pub lra: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PlanAction {
    pub id: String,
    pub tool: String,
    pub risk: RiskLevel,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TransactionPlan {
    #[schemars(with = "String")]
    pub id: Uuid,
    pub host: HostId,
    pub max_risk: RiskLevel,
    pub actions: Vec<PlanAction>,
    pub requires_confirmation: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum DawError {
    #[error("unsupported on {host:?}: {tool} ({reason})")]
    Unsupported {
        host: HostId,
        tool: String,
        reason: String,
    },
    #[error("not found: {0}")]
    NotFound(String),
    #[error("confirmation required for risk {0:?}")]
    ConfirmationRequired(RiskLevel),
    #[error("bridge disconnected")]
    Disconnected,
    #[error("{0}")]
    Message(String),
}

#[async_trait]
pub trait DawHost: Send + Sync {
    fn host_id(&self) -> HostId;
    fn capabilities(&self) -> CapabilityProfile;

    async fn session_summary(&self) -> Result<SessionSummary, DawError>;
    async fn list_tracks(&self, page: PageRequest) -> Result<Page<Track>, DawError>;
    async fn get_track(&self, id: &str) -> Result<Track, DawError>;
    async fn list_plugins(&self, track_id: &str) -> Result<Vec<PluginInstance>, DawError>;
    async fn midi_selection(&self) -> Result<Vec<MidiNote>, DawError>;
    async fn health_check(&self) -> Result<Vec<HealthIssue>, DawError>;

    async fn write_midi(&self, notes: Vec<MidiNote>) -> Result<(), DawError>;
    async fn set_parameter(&self, plugin_id: &str, name: &str, value: f64) -> Result<(), DawError>;
    async fn set_track_mute(&self, track_id: &str, mute: bool) -> Result<(), DawError>;
    async fn rename_track(&self, track_id: &str, name: &str) -> Result<(), DawError>;
    async fn set_playhead_beats(&self, beats: f64) -> Result<(), DawError>;
}

pub fn require_supported<'a>(
    profile: &'a CapabilityProfile,
    tool: &str,
) -> Result<&'a Capability, DawError> {
    let cap = profile
        .capabilities
        .iter()
        .find(|c| c.tool == tool)
        .ok_or_else(|| DawError::Unsupported {
            host: profile.host,
            tool: tool.to_string(),
            reason: "tool not listed in capability profile".into(),
        })?;
    match cap.support {
        Support::Unavailable => Err(DawError::Unsupported {
            host: profile.host,
            tool: tool.to_string(),
            reason: cap.reason.clone().unwrap_or_else(|| "unavailable".into()),
        }),
        Support::Supported | Support::Degraded => Ok(cap),
    }
}

pub mod testing;

pub fn cap(
    tool: &str,
    support: Support,
    layer: ControlLayer,
    risk: RiskLevel,
    reason: Option<&str>,
    next_step: Option<&str>,
) -> Capability {
    Capability {
        tool: tool.into(),
        support,
        layer,
        risk,
        reason: reason.map(str::to_string),
        next_step: next_step.map(str::to_string),
    }
}

pub fn root_tools() -> &'static [&'static str] {
    &[
        "daw_get_capabilities",
        "daw_get_session_summary",
        "daw_list_tracks",
        "daw_get_track",
        "daw_fetch_track_detail",
        "daw_list_plugins",
        "daw_get_midi_selection",
        "daw_analyze_spectrum",
        "daw_analyze_loudness",
        "daw_health_check",
        "daw_create_plan",
        "daw_preview_plan",
        "daw_commit_plan",
        "daw_rollback",
        "daw_write_midi",
        "daw_set_parameter",
        "daw_set_track_mute",
        "daw_rename_track",
        "daw_set_playhead",
        "daw_locate_marker",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_clamps_and_paginates() {
        let items: Vec<u32> = (0..10).collect();
        let page = Page::slice(
            items,
            PageRequest {
                limit: 3,
                offset: 3,
            },
        );
        assert_eq!(page.items, vec![3, 4, 5]);
        assert!(page.has_more);
        assert_eq!(page.next_offset, Some(6));
        assert_eq!(page.total_count, 10);
    }

    #[test]
    fn require_supported_rejects_unavailable() {
        let profile = CapabilityProfile {
            host: HostId::Logic,
            layers: vec![ControlLayer::L2Surface],
            capabilities: vec![Capability {
                tool: "daw_set_parameter".into(),
                support: Support::Unavailable,
                layer: ControlLayer::L3Native,
                risk: RiskLevel::Reversible,
                reason: Some("Logic has no project plugin API".into()),
                next_step: Some("Insert Probe or use REAPER/Ableton".into()),
            }],
        };
        let err = require_supported(&profile, "daw_set_parameter").unwrap_err();
        assert!(matches!(err, DawError::Unsupported { .. }));
    }
}
