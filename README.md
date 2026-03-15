# Model Meter

Plan and design docs for a tracker that exposes AI tool cost and usage data through:

- a local CLI
- a macOS menu bar plugin

## Current sample

This repo now includes an initial local CLI sample in Rust:

- `model-meter providers`
- `model-meter auth validate openai`
- `model-meter status`
- `model-meter status --json`

The current implementation is intentionally small:

- OpenAI/Codex auth is checked locally through `OPENAI_ADMIN_KEY`
- subscription-oriented providers like Claude, Cursor, and Windsurf can be represented with manual counters for quick status checks
- no vendor network sync is implemented yet

See [docs/QUICKSTART.md](/Users/bharad/Downloads/model-meter/docs/QUICKSTART.md) for sample usage.

## Why this project

The product direction is:

- help indie developers quickly check their current AI tool usage while they are working
- support multiple tools/providers over time
- start with the products developers already pay for, such as Codex/OpenAI and Claude

That means the architecture should be provider-agnostic even if v1 ships with a single integration.

The product problem is broader than API billing:

- many indie developers care about subscription-plan usage, credits, caps, or message limits
- many of those surfaces live in vendor product UIs rather than stable public APIs
- some users will have API usage in parallel, but that is not the default mental model for the target audience

The main limitation is important:

- API org usage and spend: sometimes officially trackable via API
- subscription-plan usage: often visible in vendor dashboards, but not consistently exposed through public APIs

Because of that, the product should be framed as a usage meter first, with multiple evidence levels:

- official API sync where supported
- user-provided imports or manual counters where official APIs do not exist
- clear unsupported states where the product cannot safely infer subscription usage

## Recommended v1 shape

Use a single Go codebase for a provider-agnostic core tracker and CLI, then expose it in the macOS menu bar via a SwiftBar plugin.

Why this shape:

- Go gives a simple single-binary CLI with low runtime overhead
- the tracker logic lives in one place
- provider-specific fetchers can plug into one shared storage and reporting model
- SwiftBar is already a menu bar plugin system, so it is the fastest route to a usable macOS menu bar experience
- this avoids building and maintaining a native macOS app before the data contracts are stable

## Proposed deliverables

- `model-meter` CLI binary
- SwiftBar plugin script that calls `model-meter status --json`
- an internal provider adapter boundary so other tools can be added later
- local cache and budget state
- install script for the CLI and plugin
- launchd job for periodic refresh

Naming note:

- use `model-meter` as the product and binary name from the start
- if needed during migration, `codex-cost` can exist only as a temporary alias, not as the primary UX surface

## Core features

- a quick current-status view for the tools the user actively uses
- provider-level rollups so a user can compare Codex, Claude, Cursor, Windsurf, and future integrations
- usage-level visibility aligned to each provider surface: spend, credits, message caps, or budget progress
- budget progress and threshold alerts
- model-, project-, line-item-, or plan-level breakdowns where the source supports them
- UTC-aware reporting with local-time display
- offline cache so the menu bar stays responsive

## Non-goals for v1

- scraping ChatGPT web pages
- scraping Cursor, Claude, Windsurf, or other vendor dashboards
- using undocumented/private endpoints
- pretending unsupported subscription usage is authoritative
- a full native macOS app

## Official sources

- https://platform.openai.com/docs/api-reference/usage/cost
- https://help.openai.com/en/articles/10478918-api-usage-dashboard
- https://help.openai.com/en/articles/9687866-admin-and-audit-logs-api-for-the-api-platform
- https://help.openai.com/en/articles/12642688-using-credits-for-flexible-usage-in-chatgpt-free-go-plus-pro

## Document map

- `docs/PLAN.md`: implementation phases and milestones
- `docs/ARCHITECTURE.md`: system design and data flow
- `docs/OPENAI_SURFACES.md`: official API/UI surfaces and limitations

## Integration strategy

- Phase 1: define a shared usage schema and ship the fastest trustworthy surfaces for indie developers
- Phase 2: add more providers through official APIs, documented exports, or explicit manual/import flows
- Phase 3: unify cross-provider reporting around a shared internal schema and source-confidence model

## Implementation guardrails

- billing totals must be derived from idempotent syncs and authoritative cost rows, not from incremental arithmetic in the CLI
- the product must distinguish authoritative data from imported, manual, estimated, or unsupported usage in both storage and UX
- the UX should optimize for a fast “how close am I to my limit?” check, not just financial reporting
- users should be told clearly which providers are fully supported, partially supported, or unsupported
