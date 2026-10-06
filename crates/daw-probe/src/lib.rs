//! Multi-instance Probe aggregator (L1). Plugin instances register over IPC.

use daw_contracts::{ControlLayer, HostId, SpectrumResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeFrame {
    pub plugin_id: String,
    pub track_hint: Option<String>,
    pub sample_rate: f32,
    pub rms: f32,
    pub peak: f32,
    #[serde(default)]
    pub spectrum: Option<SpectrumResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskingHit {
    pub a: String,
    pub b: String,
    pub band_hz: f32,
    pub overlap_db: f32,
}

#[derive(Default)]
pub struct ProbeHub {
    frames: Mutex<HashMap<String, ProbeFrame>>,
}

impl ProbeHub {
    pub fn register(&self, frame: ProbeFrame) {
        self.frames
            .lock()
            .expect("probe mutex")
            .insert(frame.plugin_id.clone(), frame);
    }

    pub fn snapshot(&self) -> Vec<ProbeFrame> {
        self.frames.lock().expect("probe mutex").values().cloned().collect()
    }

    pub fn masking_matrix(&self) -> Vec<MaskingHit> {
        let frames = self.snapshot();
        let mut hits = Vec::new();
        for (i, a) in frames.iter().enumerate() {
            for b in frames.iter().skip(i + 1) {
                if let (Some(sa), Some(sb)) = (&a.spectrum, &b.spectrum) {
                    for (idx, hz) in sa.bands_hz.iter().enumerate() {
                        if let (Some(ma), Some(mb)) =
                            (sa.magnitudes_db.get(idx), sb.magnitudes_db.get(idx))
                        {
                            let overlap = ma.min(*mb);
                            if overlap > -24.0 {
                                hits.push(MaskingHit {
                                    a: a.plugin_id.clone(),
                                    b: b.plugin_id.clone(),
                                    band_hz: *hz,
                                    overlap_db: overlap,
                                });
                            }
                        }
                    }
                }
            }
        }
        hits
    }

    pub fn host_id() -> HostId {
        HostId::Plugin
    }

    pub fn layer() -> ControlLayer {
        ControlLayer::L1Probe
    }

    pub fn new_id() -> String {
        format!("probe-{}", Uuid::new_v4())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masking_flags_overlapping_bands() {
        let hub = ProbeHub::default();
        let spec = SpectrumResult {
            bands_hz: vec![80.0],
            magnitudes_db: vec![-12.0],
        };
        hub.register(ProbeFrame {
            plugin_id: "a".into(),
            track_hint: Some("Kick".into()),
            sample_rate: 48000.0,
            rms: 0.1,
            peak: 0.4,
            spectrum: Some(spec.clone()),
        });
        hub.register(ProbeFrame {
            plugin_id: "b".into(),
            track_hint: Some("Bass".into()),
            sample_rate: 48000.0,
            rms: 0.1,
            peak: 0.4,
            spectrum: Some(spec),
        });
        let hits = hub.masking_matrix();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].band_hz, 80.0);
    }
}
