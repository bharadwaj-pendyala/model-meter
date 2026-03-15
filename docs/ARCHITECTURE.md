# Architecture

## Stack

- Language: Go
- CLI framework: Cobra
- Config: YAML or TOML in user config dir
- Storage: SQLite
- Menu bar: SwiftBar plugin
- Background refresh: launchd

## Why Go + SwiftBar

This keeps the system simple:

- one compiled binary owns network calls, normalization, caching, and formatting
- one shared domain model can support many providers/tools
- the menu bar layer stays dumb and replaceable
- packaging is easier than a mixed Electron or native-app stack
- the user gets one fast place to check “where am I relative to my limit?” across tools

## Component model

### 1. Provider registry and adapters

Responsibilities:

- register supported providers
- expose a common interface for auth validation, sync or import, normalization, and support-level reporting
- isolate provider-specific API quirks from the rest of the system

First adapters:

- OpenAI/Codex API usage and costs
- one import-backed or manual-backed subscription surface to validate the non-API path

Future adapters:

- Cursor, Claude, Windsurf, and others when a reliable supported surface exists

### 2. Provider client

Responsibilities:

- authenticate with `OPENAI_ADMIN_KEY`
- fetch usage buckets
- fetch cost buckets
- handle retries, timeouts, and pagination
- fetch overlapping backfill windows so corrected upstream buckets can be reconciled
- import documented exports or accept manual plan counters for providers without an API surface

Key endpoints:

- `GET /v1/organization/usage/completions`
- `GET /v1/organization/costs`

Later endpoints:

- images
- audio
- embeddings
- vector stores
- code interpreter sessions

### 3. Normalizer

Responsibilities:

- flatten bucketed responses, imported rows, and manual counters into consistent rows
- compute rollups by provider, day, month, plan, project, model, and line item
- mark data freshness and partial failures
- assign deterministic row identities so sync can upsert rather than append duplicate facts

Suggested canonical dimensions:

- provider
- account or workspace
- plan
- source kind
- billing mode
- project
- model
- line item
- metric type
- aggregation granularity
- value status
- currency
- time bucket

Suggested row semantics:

- `source_kind` distinguishes official API data from future imported or inferred sources
- `billing_mode` distinguishes billed API usage from subscription-plan or entitlement-backed product modes
- `value_status` distinguishes authoritative values from estimates or partial breakdowns
- row identity should be derived from provider, account, source kind, billing mode, metric type, dimensions, and bucket bounds

Suggested support tiers:

- `official-api`: vendor exposes a documented machine-readable surface
- `documented-import`: vendor or user can provide a stable export the tool can parse
- `manual`: user enters plan limit or observed usage directly
- `unsupported`: no trustworthy automation path exists yet

### 4. Local store

Responsibilities:

- persist fetched snapshots
- store config and budgets
- store provider identities and account metadata
- support fast read paths for menu bar rendering

Suggested persistence model:

- store raw sync runs for auditability and fixture refresh
- store normalized fact rows with a uniqueness constraint on the canonical row identity
- store derived summary tables for fast CLI and menu bar reads
- store freshness metadata, partial-failure markers, and last successful sync per provider
- store provider support tier and active source type so the UI can explain how each number was obtained

Suggested paths:

- config: `~/Library/Application Support/model-meter/config.toml`
- db: `~/Library/Application Support/model-meter/tracker.db`
- logs: `~/Library/Logs/model-meter.log`

### 5. CLI layer

Responsibilities:

- human-readable reporting
- JSON output for machine consumers
- commands for auth validation, sync, status, provider selection, and budgets

Freshness contract:

- `status` and other read commands should return cached data with `fresh`, `stale`, or `error` state in the payload
- stale data alone should not force a non-zero exit code unless the user explicitly requests fresh-only behavior
- auth validation and sync commands should fail non-zero on missing, invalid, or insufficient credentials
- provider state should also indicate whether the current figure is API-backed, import-backed, manual, or unsupported

### 6. SwiftBar plugin

Responsibilities:

- call `model-meter status --json`
- render a concise title such as `Claude 42%` or `$12.40 / $100`
- show dropdown details and actions
- treat stale cache as readable degraded state and reserve hard error UI for unreadable cache or failed command execution

## Data flow

1. launchd or user runs `model-meter sync`
2. provider registry selects enabled adapters
3. provider client fetches usage and cost buckets from supported APIs or ingests import/manual data from another supported source
4. normalizer converts provider inputs into shared internal rows with deterministic identities, support tier, and freshness metadata
5. store upserts normalized facts, records the sync run, and refreshes summary tables
6. CLI reads summaries directly from SQLite
7. SwiftBar plugin calls CLI JSON output and renders menu bar text

## Security

- prefer admin key lookup from macOS Keychain before env vars
- never print secrets in logs
- redact headers and query strings in debug mode
- store only cached usage/cost metadata, never secrets
- fail onboarding early when required admin access is unavailable instead of allowing a partially installed but unusable setup

## Failure handling

- if API is unreachable, show last cached totals with stale marker
- if auth fails, show a clear error state in both CLI and menu bar
- if costs endpoint succeeds but usage endpoint fails, keep billing totals and mark breakdowns partial
- if one provider fails but others sync, preserve partial multi-provider results and report the failed provider explicitly
- if a provider is manual-only or unsupported, surface that explicitly instead of showing a misleading zero

## Test strategy

- unit tests for response normalization
- fixture tests for pagination and grouping
- CLI golden tests for text output
- plugin smoke test against canned JSON
