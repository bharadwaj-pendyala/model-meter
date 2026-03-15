# Model Meter

`model-meter` is an open-source, model-agnostic CLI for tracking AI tool usage from the terminal.

The project is intended to be provider-agnostic:

- one tool for checking usage across AI products such as Codex, Cursor, Claude, and Windsurf
- one clear support model for automated, manual, and unsupported providers
- start with the integrations that are practical today and expand carefully over time

## At A Glance

Use `model-meter` if you want a tool that helps you:

- check Codex usage from the CLI
- track AI usage across tools like Codex, Cursor, Claude, and Windsurf
- understand what is supported today and what is still manual or unsupported

The current strongest integration is Codex. The broader product direction is a shared usage meter across multiple AI coding tools.

## Current Support

What works today:

- `model-meter codex` reads the current Codex usage snapshot from an existing local Codex login session
- `model-meter codex --json` returns the same snapshot in JSON
- `model-meter status` shows the current provider summary
- `model-meter auth validate codex` checks whether a Codex session is available
- manual counters can be used for providers that do not yet have a supported automated usage path

So the current shape is:

- Model Meter is model-agnostic by design
- Codex is the first real integration
- other providers are represented with manual or partial support until trustworthy automated sources exist

Current provider state:

- `codex`: supported for usage snapshot lookup from an existing local session
- `openai`: auth detection through `OPENAI_ADMIN_KEY` or Codex login detection
- `claude`: manual-only outside the Claude session
- `cursor`: manual-only
- `windsurf`: manual-only

## Provider Support Matrix

| Provider | Current support | What it means |
| --- | --- | --- |
| Codex | Working usage snapshot | `model-meter codex` shows current usage left from an existing local Codex session |
| Cursor | Manual | you can represent usage with local counters for now |
| Claude | Manual / partial | manual counters today; no supported non-interactive usage command integrated yet |
| Windsurf | Manual | you can represent usage with local counters for now |
| OpenAI API | Auth detection | API auth can be detected, but full usage syncing is not the current primary flow |

## What This Tool Does Not Do Yet

- full multi-provider syncing
- dashboard scraping
- undocumented private integrations presented as authoritative
- packaged install methods beyond source install and GitHub release binaries

## Install

### GitHub release binary

Download the archive for your platform from Releases, extract it, and place `model-meter` on your `PATH`.

Releases:

`https://github.com/bharadwaj-pendyala/model-meter/releases`

### Build from source

```bash
cargo install --path .
```

## Usage

If you already use Codex and are logged in, the main working command today is:

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

For JSON output:

```bash
model-meter codex --json
```

Other useful commands:

```bash
model-meter providers
model-meter auth validate codex
model-meter status
model-meter status --json
```

If you found this repository because you were searching for any of these, you are in the right place:

- track Codex usage from CLI
- AI usage tracker for Cursor
- Claude usage meter
- Windsurf usage tracker
- model-agnostic AI usage tool

## Requirements For `model-meter codex`

`model-meter codex` is one provider-specific command inside a broader model-agnostic CLI.

This command works when:

- you already have the Codex CLI installed
- you are already logged in through Codex
- the local Codex session is valid
- the current Codex usage endpoint remains available

Check your Codex login with:

```bash
codex login status
```

## Manual Counters

For providers without supported automated usage yet, you can supply local counters so the tool still acts as a shared usage meter.

Examples:

```bash
export MODEL_METER_CLAUDE_USED=42
export MODEL_METER_CLAUDE_LIMIT=100

export MODEL_METER_CURSOR_USED=15
export MODEL_METER_CURSOR_LIMIT=50
```

## Roadmap

What users can expect next:

- better packaging and install flow
- cleaner provider status output
- clearer support tiers per provider
- more reliable configuration and error messages
- expansion to additional providers such as Claude, Cursor, Windsurf, and future tools only where the data source is trustworthy
- a menu bar layer once the CLI contract is stable

What this means in practice:

- Codex should keep getting easier to install and use
- other providers should move from manual support to stronger integrations only when the data source is safe and maintainable
- the project should become a clearer answer to “how do I track my AI tool usage across multiple coding tools?”

What will continue to guide the project:

- prefer official or clearly-supported usage surfaces
- avoid misleading users with fake precision
- label unsupported or partial integrations clearly

## Open Source

This repository is open for public use, feedback, and contributions.

If you try the tool and hit a problem, please open an issue here:

`https://github.com/bharadwaj-pendyala/model-meter/issues`

If you want to contribute code, please read [CONTRIBUTING.md](/Users/bharad/Downloads/model-meter/CONTRIBUTING.md) first.

If you want to request support for another provider, open an issue and describe:

- the provider
- what usage surface exists today
- whether that surface is official, manual, or unsupported

## Docs

- [Quickstart](/Users/bharad/Downloads/model-meter/docs/QUICKSTART.md)
- [Plan](/Users/bharad/Downloads/model-meter/docs/PLAN.md)
- [Architecture](/Users/bharad/Downloads/model-meter/docs/ARCHITECTURE.md)
- [OpenAI Surfaces](/Users/bharad/Downloads/model-meter/docs/OPENAI_SURFACES.md)
