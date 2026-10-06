# Ableton Live Remote Script → JSON TCP 127.0.0.1:17301
# Copy this folder into MIDI Remote Scripts as `DawMcp`.

from __future__ import annotations

HOST = "127.0.0.1"
PORT = 17301


class DawMcp:
    def __init__(self, c_instance):
        self._c = c_instance
        self.song = c_instance.song()

    def disconnect(self):
        pass

    def list_tracks(self):
        out = []
        for i, t in enumerate(self.song.tracks):
            out.append(
                {
                    "id": f"trk-{i}",
                    "name": t.name,
                    "index": i,
                    "kind": "midi" if t.has_midi_input else "audio",
                    "mute": bool(t.mute),
                    "solo": bool(t.solo),
                    "armed": bool(getattr(t, "arm", False)),
                }
            )
        return out

    def set_track_mute(self, index, mute):
        self.song.tracks[index].mute = mute


def create_instance(c_instance):
    return DawMcp(c_instance)
