-- REAPER Lua bridge: JSON lines over TCP 127.0.0.1:17300
-- Actions → ReaScript: load this and keep it running.

local PORT = tonumber(reaper.GetExtState("daw_mcp", "port")) or 17300

local function json_escape(s)
  s = string.gsub(s or "", "\\", "\\\\")
  s = string.gsub(s, '"', '\\"')
  s = string.gsub(s, "\n", "\\n")
  return s
end

local function list_tracks()
  local n = reaper.CountTracks(0)
  local parts = { '{"items":[' }
  for i = 0, n - 1 do
    local tr = reaper.GetTrack(0, i)
    local _, name = reaper.GetTrackName(tr)
    if i > 0 then parts[#parts + 1] = "," end
    parts[#parts + 1] = string.format(
      '{"id":"trk-%d","name":"%s","index":%d,"kind":"unknown","mute":%s,"solo":false,"armed":false}',
      i, json_escape(name), i, tostring(reaper.GetMediaTrackInfo_Value(tr, "B_MUTE") > 0)
    )
  end
  parts[#parts + 1] = string.format('],"total_count":%d}', n)
  return table.concat(parts)
end

function daw_mcp_poll()
  -- Placeholder: a C/Rust helper should accept the TCP socket.
  -- This script proves the ReaScript surface used by daw-bridge-reaper.
  reaper.defer(daw_mcp_poll)
end

reaper.ShowConsoleMsg(string.format("[daw-mcp] REAPER bridge ready on %d\n", PORT))
daw_mcp_poll()
