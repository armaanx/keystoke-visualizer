# Keystroke Visualizer

Local CLI tool for recording keyboard activity in a session and rendering a React-based local analytics dashboard through a short-lived localhost server.

## Current Capabilities

- Detached background collector
- SQLite-backed local session storage
- Local daemon control channel for clean stop/flush
- `start`, `status`, `stop`, `report`, `list`, and `doctor` commands
- Per-key counts and per-minute activity buckets
- React report UI served by Rust with:
  - session header and summary metrics
  - activity timeline
  - top-key ranking
  - keyboard heatmap
  - detailed per-key analytics
  - recent session history

## Usage

```powershell
cargo run -- start --name work-session
cargo run -- status
cargo run -- stop
cargo run -- report <session-id> --open
cargo run -- doctor
```

## Frontend

The report UI lives in `ui/` and builds with Vite.

```powershell
cd ui
npm install
npm run build
```

Rust serves the built assets from `ui/dist` and embeds them into the binary for the integrated report flow.

## Notes

- All data is stored locally under the platform application data directory.
- The collector stores aggregate counts only. It does not store typed text or replayable key sequences.
- `report` starts a local server on `127.0.0.1` and protects report/API routes with a one-time token in the browser URL.
- The first integrated web UI is focused on single-session reporting plus read-only recent-session history.
- Linux and macOS still need deeper platform verification. The integrated web report flow was compiled and smoke-tested on Windows in this workspace.
