# Quickstart

The current sample is a small Rust CLI that focuses on a fast local status check.

## Build

```bash
cargo build
```

## Commands

```bash
cargo run -- providers
cargo run -- auth validate openai
cargo run -- status
cargo run -- status --json
```

## OpenAI / Codex sample

Set an admin key to mark the OpenAI provider as API-configured:

```bash
export OPENAI_ADMIN_KEY=your-key
```

Optional quick-check counters can also be set for a local status view:

```bash
export MODEL_METER_OPENAI_USED=18
export MODEL_METER_OPENAI_LIMIT=100
```

## Manual counters for subscription products

```bash
export MODEL_METER_CLAUDE_USED=42
export MODEL_METER_CLAUDE_LIMIT=100

export MODEL_METER_CURSOR_USED=15
export MODEL_METER_CURSOR_LIMIT=50
```

The sample treats these as manual counters, not authoritative provider data.
