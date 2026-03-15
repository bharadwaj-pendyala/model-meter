# Quickstart

This sample is meant to be easy to try locally. It does not talk to provider APIs yet. It gives you a simple status view from environment-based inputs.

## 1. Build

```bash
cargo build
```

## 2. Run the sample commands

```bash
cargo run -- providers
cargo run -- auth validate openai
cargo run -- status
cargo run -- status --json
```

## 3. Optional: add an OpenAI admin key

If you set `OPENAI_ADMIN_KEY`, the sample will treat OpenAI as configured for API auth validation.

```bash
export OPENAI_ADMIN_KEY=your-key
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

## What to expect

- `providers` shows the supported providers known by the sample
- `auth validate openai` checks whether `OPENAI_ADMIN_KEY` is present
- `status` prints a human-readable summary
- `status --json` prints the same data in JSON form

## Important limitation

The counters above are manual sample values, not authoritative provider data.
