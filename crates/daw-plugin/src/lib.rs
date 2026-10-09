//! Realtime-safe ring buffers. The audio callback must never allocate, lock, or do I/O.
//!
//! vst3-sys's COM vtable macros trip this rustc lint; it is not our code.
#![allow(semicolon_in_expressions_from_non_local_macros)]

pub mod chat;
#[cfg(feature = "vst-clap")]
pub mod editor;
pub mod ipc;
#[cfg(feature = "vst-clap")]
pub mod plugin;
pub mod sidecar;

#[cfg(feature = "vst-clap")]
pub use plugin::DawAgentPlugin;

use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

/// Single-producer single-consumer f32 ring. Capacity is fixed at construction.
pub struct AudioRing {
    buf: Box<[AtomicU32]>,
    write: AtomicUsize,
    read: AtomicUsize,
}

impl AudioRing {
    pub fn with_capacity(capacity: usize) -> Self {
        let cap = capacity.max(2).next_power_of_two();
        let buf = (0..cap)
            .map(|_| AtomicU32::new(0))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            buf,
            write: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
        }
    }

    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// Audio thread: push samples. Drops oldest on overflow (no alloc).
    pub fn push_audio(&self, samples: &[f32]) {
        let cap = self.buf.len();
        let mut w = self.write.load(Ordering::Relaxed);
        for &s in samples {
            let next = (w + 1) & (cap - 1);
            let r = self.read.load(Ordering::Acquire);
            if next == r {
                let nr = (r + 1) & (cap - 1);
                self.read.store(nr, Ordering::Release);
            }
            self.buf[w].store(s.to_bits(), Ordering::Relaxed);
            w = next;
        }
        self.write.store(w, Ordering::Release);
    }

    /// Non-audio thread: drain into `out`. Returns frames copied.
    pub fn drain(&self, out: &mut [f32]) -> usize {
        let cap = self.buf.len();
        let mut r = self.read.load(Ordering::Relaxed);
        let w = self.write.load(Ordering::Acquire);
        let mut n = 0;
        while r != w && n < out.len() {
            out[n] = f32::from_bits(self.buf[r].load(Ordering::Relaxed));
            r = (r + 1) & (cap - 1);
            n += 1;
        }
        self.read.store(r, Ordering::Release);
        n
    }
}

/// MIDI event queued from process() without allocation after init.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MidiEvent {
    pub sample_offset: u32,
    pub status: u8,
    pub data1: u8,
    pub data2: u8,
}

impl MidiEvent {
    fn pack(self) -> u64 {
        let midi =
            u32::from(self.status) | (u32::from(self.data1) << 8) | (u32::from(self.data2) << 16);
        (u64::from(self.sample_offset) << 32) | u64::from(midi)
    }

    fn unpack(v: u64) -> Self {
        let sample_offset = (v >> 32) as u32;
        let midi = v as u32;
        Self {
            sample_offset,
            status: (midi & 0xff) as u8,
            data1: ((midi >> 8) & 0xff) as u8,
            data2: ((midi >> 16) & 0xff) as u8,
        }
    }
}

pub struct MidiRing {
    buf: Box<[AtomicU64]>,
    write: AtomicUsize,
    read: AtomicUsize,
}

impl MidiRing {
    pub fn with_capacity(capacity: usize) -> Self {
        let cap = capacity.max(2).next_power_of_two();
        let buf = (0..cap)
            .map(|_| AtomicU64::new(0))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            buf,
            write: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, ev: MidiEvent) -> bool {
        let cap = self.buf.len();
        let w = self.write.load(Ordering::Relaxed);
        let next = (w + 1) & (cap - 1);
        if next == self.read.load(Ordering::Acquire) {
            return false;
        }
        self.buf[w].store(ev.pack(), Ordering::Relaxed);
        self.write.store(next, Ordering::Release);
        true
    }

    pub fn pop(&self) -> Option<MidiEvent> {
        let cap = self.buf.len();
        let r = self.read.load(Ordering::Relaxed);
        let w = self.write.load(Ordering::Acquire);
        if r == w {
            return None;
        }
        let ev = MidiEvent::unpack(self.buf[r].load(Ordering::Relaxed));
        self.read.store((r + 1) & (cap - 1), Ordering::Release);
        Some(ev)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_ring_roundtrip() {
        let ring = AudioRing::with_capacity(16);
        ring.push_audio(&[0.1, 0.2, 0.3]);
        let mut out = [0.0; 8];
        let n = ring.drain(&mut out);
        assert_eq!(n, 3);
        assert!((out[0] - 0.1).abs() < 1e-6);
    }

    #[test]
    fn midi_ring_drops_when_full() {
        let ring = MidiRing::with_capacity(4);
        let ev = MidiEvent {
            sample_offset: 0,
            status: 0x90,
            data1: 60,
            data2: 100,
        };
        assert!(ring.push(ev));
        assert!(ring.push(ev));
        assert!(ring.push(ev));
        assert!(!ring.push(ev));
        assert_eq!(ring.pop().unwrap(), ev);
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn rings_are_send_sync() {
        assert_send_sync::<AudioRing>();
        assert_send_sync::<MidiRing>();
    }
}

#[cfg(feature = "vst-clap")]
use nih_plug::prelude::*;

#[cfg(feature = "vst-clap")]
nih_export_clap!(DawAgentPlugin);
#[cfg(feature = "vst-clap")]
nih_export_vst3!(DawAgentPlugin);
