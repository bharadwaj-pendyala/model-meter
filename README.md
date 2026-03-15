# Model Meter

`model-meter` is an open-source, model-agnostic CLI for tracking AI tool usage from the terminal.

If you are searching for:

- a Codex usage CLI
- a Cursor usage tracker
- a Claude usage tracker
- a Windsurf usage tracker
- a model-agnostic AI usage tool

this repository is for that category of problem.

The project is intended to be provider-agnostic:

- one tool for checking usage across AI products such as Codex, Cursor, Claude, and Windsurf
- one clear support model for working usage, partial local-session support, and planned providers
- start with the integrations that are practical today and expand carefully over time

## At A Glance

Use `model-meter` if you want a tool that helps you:

- check Codex usage from the CLI
- track AI usage across tools like Codex, Cursor, Claude, and Windsurf
- understand what is supported today and what is still partial or unsupported

The current strongest integration is Codex. The broader product direction is a shared usage meter across multiple AI coding tools.

Current first working integration:

- Codex usage tracking from an existing local Codex session

## Current Support

What works today:

- `model-meter codex` reads the current Codex usage snapshot from an existing local Codex login session
- `model-meter cursor` detects a local Cursor session and shows the current local plan
- `model-meter claude` detects a local Claude session and shows the current local plan
- `model-meter windsurf` probes for local Windsurf / Codeium install state
- `model-meter status` shows the current provider summary
- `model-meter auth validate <provider>` checks whether a reusable local session is available

So the current shape is:

- Model Meter is model-agnostic by design
- Codex is the first real integration
- other providers are represented with local-session partial support until trustworthy usage surfaces are mapped

Current provider state:

- `codex`: supported for usage snapshot lookup from an existing local session
- `cursor`: local-session account and plan detection
- `claude`: local-session account and plan detection
- `windsurf`: local install/session probing

## Provider Support Matrix

| Provider | Current support | What it means |
| --- | --- | --- |
| Codex | Working usage snapshot | `model-meter codex` shows current usage left from an existing local Codex session |
| Cursor | Partial local-session support | `model-meter cursor` detects the local account state and current plan, but not usage percent yet |
| Claude | Partial local-session support | `model-meter claude` detects the local account state and current plan, but not usage percent yet |
| Windsurf | Probe only | local install/session paths are probed, but usage is not implemented yet |

Search-intent summary:

- `Codex usage CLI`: supported today
- `Cursor usage tracker`: local-session account detection today
- `Claude usage tracker`: local-session account detection today
- `Windsurf usage tracker`: provider probe today

## What This Tool Does Not Do Yet

- full multi-provider syncing
- dashboard scraping
- undocumented private integrations presented as authoritative
- packaged install methods beyond source install and GitHub release binaries

## Install

### GitHub release binary

Download the release for your platform from Releases.

Examples:

macOS:

```bash
open model-meter-aarch64-apple-darwin.pkg
```

The signed, notarized installer places `model-meter` in `/usr/local/bin`, so it should run from a normal Terminal session without extra `PATH` changes. macOS may still prompt for administrator credentials during installation because the package writes to a system-wide location.

Linux, system-wide:

```bash
chmod +x model-meter
sudo mv model-meter /usr/local/bin/model-meter
```

Linux, user-local:

```bash
chmod +x model-meter
mkdir -p "$HOME/.local/bin"
mv model-meter "$HOME/.local/bin/model-meter"
export PATH="$HOME/.local/bin:$PATH"
```

Windows PowerShell, user-local:

```powershell
New-Item -ItemType Directory -Force "$HOME\\bin" | Out-Null
Move-Item .\model-meter.exe "$HOME\\bin\\model-meter.exe"
$env:Path = "$HOME\\bin;" + $env:Path
```

Releases:

`https://github.com/bharadwaj-pendyala/model-meter/releases`

### Build from source

```bash
cargo install --path .
```

Fastest path after install:

```bash
model-meter codex
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

Other provider examples:

```text
$ model-meter cursor
Cursor usage
- plan: free
- usage: not exposed through the local session yet
- state: session-detected
- detail: Local Cursor session markers were detected in .../Cursor/User/globalStorage/state.vscdb with plan free. Current local state does not expose quota or remaining percentage yet.
```

```text
$ model-meter claude
Claude usage
- account: you@example.com
- plan: claude_free
- usage: not exposed through the local session yet
- state: session-detected
- detail: Local Claude desktop session markers were detected in .../Claude. Organization type: claude_free. Current local state does not expose quota or remaining percentage yet.
```

```text
$ model-meter windsurf
Windsurf usage
- usage: session not detected
- state: missing
- detail: Windsurf or Codeium local app data was not found in the standard macOS paths.
```

Other useful commands:

```bash
model-meter cursor
model-meter claude
model-meter windsurf
model-meter providers
model-meter auth validate codex
model-meter auth validate cursor
model-meter auth validate claude
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

## Why Some Providers Do Not Show A Usage Percentage Yet

For Codex, the local logged-in session currently exposes real rate-limit window data, so `model-meter` can calculate percent left.

For Cursor and Claude, the local session data I can currently read exposes account and plan markers, but not a trustworthy pair of values like:

- used quota
- total quota

Without both of those numbers, a remaining percentage would be made up rather than measured.

So the current rule is:

- if the local session exposes real usage windows or used/limit values, `model-meter` computes a percentage
- if the local session only exposes account metadata, `model-meter` reports that honestly and does not invent a percentage

## Roadmap

What users can expect next:

- better packaging and install flow
- cleaner provider status output
- clearer support tiers per provider
- more reliable configuration and error messages
- expansion from local-session account detection to actual usage retrieval for providers like Cursor, Claude, Windsurf, and future tools only where the data source is trustworthy
- a menu bar layer once the CLI contract is stable

What this means in practice:

- Codex should keep getting easier to install and use
- other providers should move from local-session account detection to stronger usage integrations only when the data source is safe and maintainable
- the project should become a clearer answer to “how do I track my AI tool usage across multiple coding tools?”

What will continue to guide the project:

- prefer official or clearly-supported usage surfaces
- avoid misleading users with fake precision
- label unsupported or partial local-session integrations clearly

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
