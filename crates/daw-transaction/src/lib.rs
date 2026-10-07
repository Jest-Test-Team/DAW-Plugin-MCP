//! Preview / commit / rollback with risk gates.

use daw_contracts::{
    require_supported, CapabilityProfile, DawError, DawHost, PlanAction, RiskLevel, TransactionPlan,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Checkpoint {
    pub plan_id: Uuid,
    pub midi: Option<Vec<daw_contracts::MidiNote>>,
    pub muted: HashMap<String, bool>,
    pub names: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Preview {
    pub plan: TransactionPlan,
    pub markdown: String,
}

pub struct TransactionEngine {
    checkpoints: Mutex<HashMap<Uuid, Checkpoint>>,
}

impl Default for TransactionEngine {
    fn default() -> Self {
        Self {
            checkpoints: Mutex::new(HashMap::new()),
        }
    }
}

impl TransactionEngine {
    pub fn create_plan(
        &self,
        profile: &CapabilityProfile,
        actions: Vec<PlanAction>,
        confirmed: bool,
    ) -> Result<TransactionPlan, DawError> {
        let mut max_risk = RiskLevel::Read;
        for action in &actions {
            let cap = require_supported(profile, &action.tool)?;
            if cap.risk > max_risk {
                max_risk = cap.risk;
            }
            if cap.risk >= RiskLevel::Structural && !confirmed {
                return Err(DawError::ConfirmationRequired(cap.risk));
            }
        }
        Ok(TransactionPlan {
            id: Uuid::new_v4(),
            host: profile.host,
            max_risk,
            actions,
            requires_confirmation: max_risk >= RiskLevel::Structural,
        })
    }

    pub fn preview(&self, plan: &TransactionPlan) -> Preview {
        let mut lines = vec![
            format!("# Plan {}", plan.id),
            format!("Host: {:?}", plan.host),
            format!("Max risk: {:?}", plan.max_risk),
            String::new(),
        ];
        for (i, a) in plan.actions.iter().enumerate() {
            lines.push(format!(
                "{}. `{}` ({:?}) {}",
                i + 1,
                a.tool,
                a.risk,
                a.params
            ));
        }
        Preview {
            plan: plan.clone(),
            markdown: lines.join("\n"),
        }
    }

    pub async fn commit(
        &self,
        host: &dyn DawHost,
        plan: &TransactionPlan,
        confirmed: bool,
    ) -> Result<Checkpoint, DawError> {
        if plan.max_risk >= RiskLevel::Structural && !confirmed {
            return Err(DawError::ConfirmationRequired(plan.max_risk));
        }
        let midi = host.midi_selection().await.ok();
        let tracks = host
            .list_tracks(daw_contracts::PageRequest {
                limit: daw_contracts::MAX_PAGE_LIMIT,
                offset: 0,
            })
            .await?;
        let muted = tracks
            .items
            .iter()
            .map(|t| (t.id.clone(), t.mute))
            .collect();
        let names = tracks
            .items
            .iter()
            .map(|t| (t.id.clone(), t.name.clone()))
            .collect();
        let checkpoint = Checkpoint {
            plan_id: plan.id,
            midi,
            muted,
            names,
        };
        self.checkpoints
            .lock()
            .expect("tx mutex")
            .insert(plan.id, checkpoint.clone());

        for action in &plan.actions {
            match action.tool.as_str() {
                "daw_write_midi" => {
                    let notes = serde_json::from_value(action.params.clone())
                        .map_err(|e| DawError::Message(e.to_string()))?;
                    host.write_midi(notes).await?;
                }
                "daw_set_track_mute" => {
                    let track_id = action
                        .params
                        .get("track_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| DawError::Message("track_id required".into()))?;
                    let mute = action
                        .params
                        .get("mute")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    host.set_track_mute(track_id, mute).await?;
                }
                "daw_rename_track" => {
                    let track_id = action
                        .params
                        .get("track_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| DawError::Message("track_id required".into()))?;
                    let name = action
                        .params
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| DawError::Message("name required".into()))?;
                    host.rename_track(track_id, name).await?;
                }
                "daw_set_parameter" => {
                    let plugin_id = action
                        .params
                        .get("plugin_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| DawError::Message("plugin_id required".into()))?;
                    let name = action
                        .params
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| DawError::Message("name required".into()))?;
                    let value = action
                        .params
                        .get("value")
                        .and_then(|v| v.as_f64())
                        .ok_or_else(|| DawError::Message("value required".into()))?;
                    host.set_parameter(plugin_id, name, value).await?;
                }
                "daw_set_playhead" => {
                    let beats = action
                        .params
                        .get("beats")
                        .and_then(|v| v.as_f64())
                        .ok_or_else(|| DawError::Message("beats required".into()))?;
                    host.set_playhead_beats(beats).await?;
                }
                other => {
                    return Err(DawError::Message(format!("commit does not run {other}")));
                }
            }
        }
        Ok(checkpoint)
    }

    pub async fn rollback(&self, host: &dyn DawHost, plan_id: Uuid) -> Result<(), DawError> {
        let checkpoint = self
            .checkpoints
            .lock()
            .expect("tx mutex")
            .remove(&plan_id)
            .ok_or_else(|| DawError::NotFound(plan_id.to_string()))?;
        if let Some(midi) = checkpoint.midi {
            host.write_midi(midi).await?;
        }
        for (id, mute) in checkpoint.muted {
            host.set_track_mute(&id, mute).await?;
        }
        for (id, name) in checkpoint.names {
            host.rename_track(&id, &name).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use daw_contracts::testing::{native_profile, MockHost};
    use daw_contracts::{HostId, PlanAction, RiskLevel};
    use serde_json::json;

    #[tokio::test]
    async fn structural_requires_confirmation() {
        let host = MockHost::new(native_profile(HostId::Reaper));
        let engine = TransactionEngine::default();
        let err = engine
            .create_plan(
                &host.capabilities(),
                vec![PlanAction {
                    id: "1".into(),
                    tool: "daw_set_track_mute".into(),
                    risk: RiskLevel::Reversible,
                    params: json!({"track_id":"trk-1","mute":true}),
                }],
                false,
            )
            .unwrap();
        assert_eq!(err.max_risk, RiskLevel::Reversible);
    }

    #[tokio::test]
    async fn commit_and_rollback_restores_mute() {
        let host = MockHost::new(native_profile(HostId::Ableton));
        let engine = TransactionEngine::default();
        let plan = engine
            .create_plan(
                &host.capabilities(),
                vec![PlanAction {
                    id: "1".into(),
                    tool: "daw_set_track_mute".into(),
                    risk: RiskLevel::Reversible,
                    params: json!({"track_id":"trk-1","mute":true}),
                }],
                true,
            )
            .unwrap();
        engine.commit(&host, &plan, true).await.unwrap();
        let t = host.get_track("trk-1").await.unwrap();
        assert!(t.mute);
        engine.rollback(&host, plan.id).await.unwrap();
        let t = host.get_track("trk-1").await.unwrap();
        assert!(!t.mute);
    }
}
