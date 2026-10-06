-- Logic Pro: MCU/MMC over IAC + limited AppleScript reads.
-- Grant Automation permission to the daw-mcp process.

on session_summary()
  tell application "Logic Pro"
    set trackCount to count of tracks of front document
    return trackCount
  end tell
end session_summary
