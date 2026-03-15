# Quickstart

This sample is meant to be easy to try locally. `model-meter` is intended to be model-agnostic, but today the strongest working integration is Codex. Other providers are currently represented through auth detection or manual counters.

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
cargo run -- auth validate openai
cargo run -- auth validate codex
cargo run -- auth validate claude
cargo run -- auth validate cursor
cargo run -- auth validate windsurf
cargo run -- usage codex
cargo run -- usage codex --json
cargo run -- usage cursor
cargo run -- usage claude
cargo run -- usage windsurf
cargo run -- status
cargo run -- status --json
```

## 3. Optional: add an OpenAI admin key

If you set `OPENAI_ADMIN_KEY`, the sample will treat OpenAI as configured for API auth validation.

```bash
export OPENAI_ADMIN_KEY=your-key
```

If you are already logged in through the Codex CLI, the sample can also detect that session:

```bash
codex login status
```

## 4. Optional: add local usage counters

These counters are just local sample inputs. They are useful for seeing how status output looks.

OpenAI example:

```bash
export MODEL_METER_OPENAI_USED=18
export MODEL_METER_OPENAI_LIMIT=100
```

Claude and Cursor examples:

```bash
export MODEL_METER_CLAUDE_USED=42
export MODEL_METER_CLAUDE_LIMIT=100

export MODEL_METER_CURSOR_USED=15
export MODEL_METER_CURSOR_LIMIT=50
```

Provider API examples:

```bash
export ANTHROPIC_ADMIN_KEY=your-key
export CURSOR_ADMIN_API_KEY=your-key
export WINDSURF_SERVICE_KEY=your-key
export WINDSURF_USER_EMAIL=you@example.com
```

## What to expect

- `providers` shows the supported providers known by the sample
- `auth validate openai` checks for `OPENAI_ADMIN_KEY` first, then falls back to Codex CLI session detection
- `auth validate codex` reports whether a Codex CLI session is present
- `auth validate claude` explains the current Claude limitation
- `auth validate cursor` checks for `CURSOR_ADMIN_API_KEY`
- `auth validate windsurf` checks for `WINDSURF_SERVICE_KEY`
- `codex` is the short command for the current Codex usage snapshot inside the broader model-agnostic CLI
- `cursor` shows official Cursor usage metrics when `CURSOR_ADMIN_API_KEY` is set
- `claude` shows monthly Claude cost when `ANTHROPIC_ADMIN_KEY` is set
- `windsurf` shows Windsurf credit usage when `WINDSURF_SERVICE_KEY` is set
- `usage codex` fetches the current Codex usage windows from the logged-in local Codex session and shows percent left
- `status` prints a human-readable summary
- `status --json` prints the same data in JSON form

## Important limitation

- The counters above are manual sample values, not authoritative provider data.
- `usage codex` currently relies on the same session-backed usage path the local Codex client uses, not on a public documented `codex usage` command.
- Claude percentage-left output depends on a monthly limit you provide.
- Cursor percentage-left output depends on request or spend limits you provide.
- Windsurf per-user usage is strongest when `WINDSURF_USER_EMAIL` is set.
