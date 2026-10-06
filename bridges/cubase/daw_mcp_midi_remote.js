// Cubase MIDI Remote (sandboxed ES5). Mixer/transport only.
var midiremote = require("midiremote")
var deviceDriver = midiremote.makeDeviceDriver("daw-mcp", "Cubase Surface", "daw-mcp")
