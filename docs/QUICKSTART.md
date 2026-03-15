# Quickstart

This sample is meant to be easy to try locally. `model-meter` is model-agnostic, with Codex as the first working usage integration and Cursor / Claude as local-session account probes.

## 1. Build

```bash
cargo build
```

## 2. Run the sample commands

```bash
cargo run -- codex
cargo run -- codex --json
cargo run -- cursor
cargo run -- cursor --json
cargo run -- claude
cargo run -- claude --json
cargo run -- windsurf
cargo run -- windsurf --json
cargo run -- providers
cargo run -- auth validate codex
cargo run -- auth validate cursor
cargo run -- auth validate claude
cargo run -- auth validate windsurf
cargo run -- usage codex
cargo run -- usage codex --json
cargo run -- usage cursor
cargo run -- usage claude
cargo run -- usage windsurf
cargo run -- status
cargo run -- status --json
```

## 3. Check your local provider sessions

If you are already logged in through the Codex CLI, the sample can detect that session:

```bash
codex login status
```

## What to expect

- `providers` shows the supported providers known by the sample
- `auth validate codex` reports whether a Codex CLI session is present
- `auth validate cursor` reports whether Cursor local session markers are present
- `auth validate claude` reports whether Claude local session markers are present
- `auth validate windsurf` reports whether Windsurf / Codeium local install state was found
- `codex` is the short command for the current Codex usage snapshot inside the broader model-agnostic CLI
- `cursor` shows local Cursor account / plan detection
- `claude` shows local Claude account / plan detection
- `windsurf` shows local Windsurf probe output
- `usage codex` fetches the current Codex usage windows from the logged-in local Codex session and shows percent left
- `usage cursor`, `usage claude`, and `usage windsurf` currently report the local session/probe state and whether usage data is available
- `status` prints a human-readable summary
- `status --json` prints the same data in JSON form

## Important limitation

- `usage codex` currently relies on the same session-backed usage path the local Codex client uses, not on a public documented `codex usage` command.
- Cursor and Claude currently expose local account / plan state, but not a local remaining-usage percentage.
- Windsurf probing is wired in, but a reusable logged-in session store has not been mapped yet.
