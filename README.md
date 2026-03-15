# Model Meter

`model-meter` is a small CLI for checking AI tool usage without opening vendor dashboards.

Right now, the main working flow is Codex:

- detect your existing Codex login
- fetch your current Codex usage windows
- show how much usage is left

## What You Can Do Today

- run `model-meter codex` to see your current Codex usage
- run `model-meter codex --json` for machine-readable output
- run `model-meter status` for a broader provider snapshot
- use manual counters for providers that do not yet have a supported automated usage surface

## Install

### Option 1: run from source

```bash
cargo run -- codex
```

### Option 2: install the binary locally

```bash
cargo install --path .
model-meter codex
```

## Quick Start

If you already use the Codex CLI and are logged in, this is the main command:

```bash
model-meter codex
```

Example output:

```text
Codex usage
- plan: plus
- 5h window: 82% left (18% used), resets in 3h 37m
- weekly window: 93% left (7% used), resets in 5d 3h
```

For JSON:

```bash
model-meter codex --json
```

## How It Works

For Codex, `model-meter` currently reuses your existing local Codex login session.

That means:

- you do not need to set a separate OpenAI key just to read Codex usage
- you do need to be logged in through the Codex CLI already

You can check that with:

```bash
codex login status
```

## Available Commands

```bash
model-meter codex
model-meter codex --json
model-meter providers
model-meter auth validate openai
model-meter auth validate codex
model-meter auth validate claude
model-meter usage codex
model-meter usage codex --json
model-meter status
model-meter status --json
```

## Provider Support

Current support is intentionally uneven and explicit.

- `codex`: working usage lookup from an existing local Codex session
- `openai`: auth detection through `OPENAI_ADMIN_KEY` or Codex CLI login
- `claude`: manual-only outside the Claude session
- `cursor`: manual counters only
- `windsurf`: manual counters only

## Manual Counters

For providers without a supported automated usage source yet, you can set local counters.

Examples:

```bash
export MODEL_METER_CLAUDE_USED=42
export MODEL_METER_CLAUDE_LIMIT=100

export MODEL_METER_CURSOR_USED=15
export MODEL_METER_CURSOR_LIMIT=50
```

You can also set optional local counters for OpenAI:

```bash
export MODEL_METER_OPENAI_USED=18
export MODEL_METER_OPENAI_LIMIT=100
```

## Current Limitations

- `model-meter codex` is not calling a public documented `codex usage` command
- it currently relies on the same local session-backed usage path the Codex client appears to use
- Claude does not yet have a documented non-interactive usage command that this tool can call from outside the Claude session
- broader multi-provider sync is not implemented yet

## Docs

- [Quickstart](/Users/bharad/Downloads/model-meter/docs/QUICKSTART.md)
- [Plan](/Users/bharad/Downloads/model-meter/docs/PLAN.md)
- [Architecture](/Users/bharad/Downloads/model-meter/docs/ARCHITECTURE.md)
- [OpenAI Surfaces](/Users/bharad/Downloads/model-meter/docs/OPENAI_SURFACES.md)
