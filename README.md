# Keystroke Visualizer

Local CLI tool for recording keyboard activity in a session and rendering a local HTML report with stats, charts, and a keyboard heatmap.

## Current MVP

- Detached background collector
- `start`, `status`, `stop`, `report`, and `list` commands
- Per-key counts
- Per-minute activity buckets
- Local HTML report with:
  - summary stats
  - timeline chart
  - top-key chart
  - keyboard heatmap
  - per-key table

## Usage

```powershell
cargo run -- start --name work-session
cargo run -- status
cargo run -- stop --open
cargo run -- list
```

## Notes

- All data is stored locally under the platform application data directory.
- The collector stores aggregated counts only. It does not store typed text or replayable key sequences.
- The current stop flow force-terminates the collector process after periodic state flushes. That is acceptable for the MVP but should be replaced with a proper IPC shutdown path in the next iteration.
- Linux and macOS still need real platform verification. The implementation was compiled and smoke-tested on Windows in this workspace.
