# Keystroke Visualizer

Local-first CLI tool for recording keyboard activity in a session and serving a React analytics UI through a short-lived localhost server.

## Current Capabilities

- Detached background collector
- SQLite-backed local session storage
- Local daemon control channel for clean stop/flush
- `start`, `status`, `stop`, `report`, `live`, `list`, and `doctor` commands
- Per-key counts and per-minute activity buckets
- Short-lived token-protected local web server on `127.0.0.1`
- React dashboard UI served by Rust with:
  - editorial session hero and KPI strip
  - activity timeline
  - top-key distribution
  - keyboard heatmap
  - detailed per-key analytics
  - recent session history
  - data-integrity/status panel
- Dedicated live mode for the active session:
  - `live --open` launches a separate live route
  - updates stream over WebSockets using flushed local session snapshots
  - live duration and freshness update continuously in the browser
  - auto-transitions to the final report when the session stops, fails, or is interrupted

## Usage

```powershell
cargo run -- start --name work-session
cargo run -- status
cargo run -- live --open
cargo run -- stop --open
cargo run -- report <session-id> --open
cargo run -- list
cargo run -- doctor
```

Typical flow:

```powershell
cargo run -- start --name "Documentation Session"
cargo run -- live --open
cargo run -- stop --open
```

## Frontend

The UI lives in `ui/` and builds with Vite + React 19.

```powershell
cd ui
pnpm install
pnpm lint
pnpm build
```

Rust serves the built assets from `ui/dist` and embeds them into the binary for the integrated report/live flow.
`ui/dist` is committed on purpose so a fresh clone can build and run the Rust app without requiring an immediate frontend build step.
Cargo builds and runs automatically invoke `pnpm install` and `pnpm build` through the Rust build script, so source builds require `pnpm` to be installed.

## Notes

- All data is stored locally under the platform application data directory.
- The collector stores aggregate counts only. It does not store typed text or replayable key sequences.
- `report` starts a local server on `127.0.0.1` and protects UI/API routes with a one-time token in the browser URL.
- `live` requires an active session. If no session is currently running, it exits with a clear error.
- Live mode reads from the same flushed SQLite session snapshots used by reports; it does not read directly from daemon memory.
- The current web UI is focused on single-session analytics plus read-only recent-session history.
- Linux and macOS still need deeper platform verification. The integrated web report flow was compiled and smoke-tested on Windows in this workspace.
