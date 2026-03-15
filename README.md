# Model Meter

`model-meter` is a small developer tool for checking AI usage without digging through vendor dashboards.

The long-term product shape is:

- a local CLI
- a macOS menu bar view
- one shared model for multiple providers

## Start here

If you want to try the current sample, read [docs/QUICKSTART.md](/Users/bharad/Downloads/model-meter/docs/QUICKSTART.md).

If you want the roadmap, read [docs/PLAN.md](/Users/bharad/Downloads/model-meter/docs/PLAN.md).

If you want the system design, read [docs/ARCHITECTURE.md](/Users/bharad/Downloads/model-meter/docs/ARCHITECTURE.md).

If you want the OpenAI-specific constraints, read [docs/OPENAI_SURFACES.md](/Users/bharad/Downloads/model-meter/docs/OPENAI_SURFACES.md).

## What works today

This repo currently ships a small Rust CLI sample.

Available commands:

- `model-meter providers`
- `model-meter auth validate openai`
- `model-meter auth validate codex`
- `model-meter auth validate claude`
- `model-meter usage codex`
- `model-meter status`
- `model-meter status --json`

Current behavior:

- OpenAI can be detected through `OPENAI_ADMIN_KEY` or an existing Codex CLI login
- Codex usage can be read from an existing ChatGPT-backed Codex CLI session and shown as percentage left for the current windows
- Claude currently exposes only a documented in-session `/cost` surface, so this sample still treats Claude as manual outside the Claude session
- Cursor and similar tools can be shown with manual counters
- broader vendor sync is still not implemented yet

## What this project is solving

Developers want a fast answer to one question:

`How close am I to my limit?`

That limit can mean different things depending on the provider:

- API spend
- credits
- message caps
- subscription usage

That matters because not every provider exposes the same kind of data. Some give official APIs. Some only expose usage in a web UI. Some provide no safe automation path at all.

So the project is intentionally built around support levels:

- `official-api`: trustworthy automated sync
- `documented-import`: trustworthy import flow
- `manual`: user-entered counters
- `unsupported`: no safe automation yet

## Product direction

The target shape for v1 is:

- one core CLI
- one provider-agnostic data model
- one SwiftBar plugin that reads CLI JSON output

The recommended implementation stack is:

- Go for the main CLI and sync engine
- SQLite for local cache
- SwiftBar for the menu bar layer

This keeps the product simple:

- one binary owns the logic
- provider integrations stay behind one adapter boundary
- the menu bar stays thin

## Non-goals for v1

- scraping ChatGPT pages
- scraping Cursor, Claude, Windsurf, or similar dashboards
- using undocumented or private endpoints
- presenting unsupported subscription usage as authoritative
- building a full native macOS app before the data model is stable

## Working rules

- billing totals must come from authoritative rows, not ad hoc CLI math
- the UI must clearly label whether a number is API-backed, imported, manual, or unsupported
- stale cached data is acceptable for read flows if it is labeled clearly
- the product should optimize for fast status checks, not heavy reporting

## Official references

- https://platform.openai.com/docs/api-reference/usage/cost
- https://help.openai.com/en/articles/10478918-api-usage-dashboard
- https://help.openai.com/en/articles/9687866-admin-and-audit-logs-api-for-the-api-platform
- https://help.openai.com/en/articles/12642688-using-credits-for-flexible-usage-in-chatgpt-free-go-plus-pro
