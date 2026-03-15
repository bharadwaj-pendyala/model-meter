# Build Plan

## Product goal

Build a small, reliable tracker that lets an indie developer quickly check AI tool usage from the command line and the macOS menu bar while they are working.

The implementation strategy is:

- design the core around multiple providers/tools
- ship the fastest trustworthy surfaces first
- add tools like Codex, Claude, Cursor, and Windsurf through adapters when they have a trustworthy data source

## Scope choice

V1 should optimize for trustworthiness and speed of access, not for any single provider integration.

That means:

- supported: official API usage and costs where a provider exposes them
- supported: documented imports or user-entered plan counters where a provider does not expose a public API
- supported: budget thresholds configured locally in the tool
- supported by design, but not yet implemented: multiple providers in one shared reporting model
- not supported in v1: undocumented polling of web dashboards for subscription-plan usage
- not supported in v1: undocumented scraping of Cursor, Claude, Windsurf, or other vendor dashboards

Operational constraints for v1:

- some providers will only support partial visibility in v1, so the UX must communicate support level explicitly
- stale cached data is a supported degraded state for read commands and the menu bar, not an automatic hard failure
- sync must be idempotent and safe to rerun on overlapping time windows

## Milestones

### Milestone 1: surface and schema validation

Goal:
- define the provider adapter contract
- define the source-confidence model
- validate which provider surfaces are stable enough for reporting
- capture example payloads or import formats for the first supported sources

Tasks:
- define a shared internal schema for provider, account, plan, metric type, unit type, time bucket, source kind, and value status
- define the first provider adapter interface around fetch or import, normalize, auth validation, and support-level reporting
- document support tiers such as `official-api`, `documented-import`, `manual`, and `unsupported`
- validate the first official API surface
- validate the first non-API import or manual-entry surface
- verify pagination, grouping, and time bucket behavior where APIs exist
- define stable identities for normalized rows and the backfill window used during recurring sync
- define user-facing states for missing credentials, insufficient privileges, manual-only support, and unsupported providers
- document all assumptions

Acceptance criteria:
- a provider contract exists before provider-specific code grows
- sample responses or import fixtures are saved
- support tiers are documented for the first providers
- known data latency and confidence levels are documented
- sync identity and backfill rules are documented before the cache schema is implemented

### Milestone 2: Core fetch and normalize layer

Goal:
- convert provider responses, imports, or manual entries into a stable provider-agnostic internal model

Tasks:
- create API and import adapters with retries and timeout handling where relevant
- normalize usage buckets, cost buckets, import rows, and manual counters
- map provider, account, plan, project, line item, model, billing mode, source kind, aggregation granularity, and date dimensions
- support UTC storage with local-time presentation
- mark rows as authoritative, estimated, or unsupported where applicable

Acceptance criteria:
- core package returns a clean domain model
- the first API adapter and the first non-API adapter output fit the same schema
- fixtures cover empty, paginated, partial-data, and import/manual cases
- the model distinguishes billing truth from operational breakdowns without overloading one field

### Milestone 3: Local storage and budgets

Goal:
- keep the tool fast and support offline reads

Tasks:
- store snapshots in SQLite
- store config in a small local config file
- persist provider identities and provider-specific account metadata
- persist provider support level and current source type
- persist budgets, alert thresholds, and last successful sync
- support cache TTL and forced refresh
- persist sync runs, row fingerprints, source timestamps, and freshness state so repeated syncs can upsert rather than append blindly

Acceptance criteria:
- menu bar reads from cache in under 100 ms
- CLI can show last-sync age and stale-data warnings
- overlapping sync windows do not duplicate totals

### Milestone 4: CLI

Goal:
- provide the primary operator interface

Commands:
- `model-meter auth validate`
- `model-meter sync`
- `model-meter status`
- `model-meter providers`
- `model-meter usage`
- `model-meter daily --days 30`
- `model-meter daily --provider openai --days 30`
- `model-meter breakdown --by project`
- `model-meter breakdown --by provider`
- `model-meter breakdown --by model`
- `model-meter import`
- `model-meter plan set-limit`
- `model-meter budget set --monthly 100`
- `model-meter budget set --daily 5`
- `model-meter status --json`

Acceptance criteria:
- human-readable output for normal usage
- JSON output for plugin integration
- non-zero exit codes for auth or refresh failures when fresh data is explicitly required
- read commands return cached data with freshness metadata when stale data is allowed
- `status` shows current level and support confidence clearly enough for a quick glance

### Milestone 5: macOS menu bar plugin

Goal:
- expose the tracker in the menu bar with minimal overhead

Tasks:
- write a SwiftBar plugin script
- render current usage level, provider mix, budget or plan percentage, and last sync time
- add menu actions for refresh, open logs, and open OpenAI usage dashboard
- degrade gracefully when the cache is stale or auth fails

Acceptance criteria:
- menu bar title updates correctly
- dropdown shows daily and monthly totals
- refresh path works without opening Terminal manually

### Milestone 6: install and operations

Goal:
- make the tool easy to set up and reliable in the background

Tasks:
- create install script for binary, config path, and plugin file
- add launchd job for periodic sync
- document uninstall flow
- add log rotation guidance
- add onboarding checks that verify whether each configured provider is API-backed, import-backed, manual-only, or unsupported before background automation is installed

Acceptance criteria:
- fresh install works on a clean macOS machine
- auto-refresh runs without user interaction after setup
- unsupported or manual-only states are surfaced before launchd or SwiftBar setup completes

## Risks

- OpenAI usage and costs may not reconcile perfectly at short intervals
- many indie users will not have admin or org-owner access to provider billing APIs
- many subscription products will not expose a public machine-readable usage surface
- ChatGPT subscription usage cannot be treated as officially trackable unless OpenAI adds a public API
- other vendors may not expose a supported cost surface, which means multi-provider coverage will expand unevenly
- some upstream buckets may be corrected after initial fetch, requiring rolling backfill during sync

## Explicit product decisions

- Use the Costs endpoint as the billing source of truth
- Use Usage endpoints for richer breakdowns and operational insight
- allow documented imports and manual counters when that is the only trustworthy surface
- keep provider-specific collectors behind one adapter boundary
- Avoid any scraping or unsupported browser automation in v1
- Keep the menu bar UI as a thin layer over the CLI JSON output
- treat sync as a rolling-window upsert, not append-only accumulation
- treat stale cache as a valid degraded read state unless a command explicitly requests fresh data
- fail onboarding early for users who expect unsupported automatic tracking
- use `model-meter` as the primary binary and product name

## Suggested implementation order

1. support tiers, fixtures, and first source validation
2. sync identity, backfill policy, and normalization
3. SQLite cache and config
4. CLI commands and freshness contract
5. SwiftBar plugin
6. installer and launchd
