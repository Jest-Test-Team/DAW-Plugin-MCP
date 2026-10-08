//! nih-plug wrapper. `process()` only taps preallocated rings.

use crate::{AudioRing, MidiEvent, MidiRing};
use daw_contracts::HostId;
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::Arc;

/// Identifies this insert as the L0 plugin sensor, not a DAW native bridge.
pub const PLUGIN_HOST_ID: HostId = HostId::Plugin;

pub struct DawAgentPlugin {
    params: Arc<DawAgentParams>,
    /// Shared with the message thread; audio callback only `push_*`.
    audio: Arc<AudioRing>,
    midi: Arc<MidiRing>,
}

#[derive(Params)]
pub struct DawAgentParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    #[id = "out"]
    pub output_gain: FloatParam,
}

impl Default for DawAgentPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(DawAgentParams::default()),
            audio: Arc::new(AudioRing::with_capacity(1 << 16)),
            midi: Arc::new(MidiRing::with_capacity(1024)),
        }
    }
}

impl Default for DawAgentParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(720, 520),
            output_gain: FloatParam::new("Output", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
        }
    }
}

impl Plugin for DawAgentPlugin {
    const NAME: &'static str = "DAW Agent";
    const VENDOR: &'static str = "DAW Plugin MCP";
    const URL: &'static str = "https://github.com/Jest-Test-Team/DAW-Plugin-MCP";
    const EMAIL: &'static str = "noreply@example.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        crate::editor::create_editor(self.params.editor_state.clone(), self.params.clone())
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        while let Some(event) = context.next_event() {
            match event {
                NoteEvent::NoteOn {
                    timing,
                    channel,
                    note,
                    velocity,
                    ..
                } => {
                    let _ = self.midi.push(MidiEvent {
                        sample_offset: timing,
                        status: 0x90 | (channel & 0x0f),
                        data1: note,
                        data2: (velocity * 127.0).clamp(0.0, 127.0) as u8,
                    });
                }
                NoteEvent::NoteOff {
                    timing,
                    channel,
                    note,
                    velocity,
                    ..
                } => {
                    let _ = self.midi.push(MidiEvent {
                        sample_offset: timing,
                        status: 0x80 | (channel & 0x0f),
                        data1: note,
                        data2: (velocity * 127.0).clamp(0.0, 127.0) as u8,
                    });
                }
                _ => {}
            }
        }

        for channel_samples in buffer.iter_samples() {
            let gain = self.params.output_gain.smoothed.next();
            let mut tap: Option<f32> = None;
            for sample in channel_samples {
                if tap.is_none() {
                    tap = Some(*sample);
                }
                *sample *= gain;
            }
            if let Some(s) = tap {
                self.audio.push_audio(&[s]);
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for DawAgentPlugin {
    const CLAP_ID: &'static str = "com.dawmcp.plugin";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("AI DAW assistant plugin shell");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for DawAgentPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"DawMcpPlugShell!";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

#[cfg(test)]
mod tests {
    use super::PLUGIN_HOST_ID;
    use daw_contracts::HostId;

    #[test]
    fn plugin_host_id_is_plugin() {
        assert_eq!(PLUGIN_HOST_ID, HostId::Plugin);
    }

    #[test]
    fn default_plugin_constructs_rings() {
        let plugin = super::DawAgentPlugin::default();
        assert_eq!(plugin.audio.capacity(), 1 << 16);
        assert_eq!(plugin.params.output_gain.value(), 1.0);
    }
}
