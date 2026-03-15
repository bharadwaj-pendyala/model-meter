# OpenAI Surfaces

## What is officially trackable from a CLI

Officially documented for API organizations:

- Usage API
- Costs API
- Usage Dashboard in the API platform

Relevant docs:

- API reference: https://platform.openai.com/docs/api-reference/usage/cost
- Help article: https://help.openai.com/en/articles/10478918-api-usage-dashboard
- Admin key article: https://help.openai.com/en/articles/9687866-admin-and-audit-logs-api-for-the-api-platform

## Key constraints

- organization-wide usage and costs are exposed through API endpoints
- example requests in the official docs use `OPENAI_ADMIN_KEY`
- costs are the better source of truth for billing reconciliation
- usage can differ slightly from costs at short intervals
- dashboard timestamps are UTC

## What is not yet a safe basis for this project

ChatGPT Codex plan usage and credits:

- the ChatGPT Codex Usage Dashboard exists in product UI
- OpenAI documents where to view it in the web app
- no public API is documented for polling that dashboard from a CLI

Relevant docs:

- https://help.openai.com/en/articles/12642688-using-credits-for-flexible-usage-in-chatgpt-free-go-plus-pro
- https://help.openai.com/en/articles/11369540-using-codex-with-your-chatgpt-plan

## Product implication

For the broader product, OpenAI should be treated as the first provider adapter, not the whole architecture.

For the target audience, this means OpenAI may offer better automation for API usage than for ChatGPT-plan usage. The product should present that difference clearly rather than pretending both are equally trackable.

The tracker should have two explicit modes in the product language for the OpenAI integration:

- `api-mode`: supported and built on official endpoints
- `chatgpt-mode`: unavailable until a documented API exists

Do not blur those modes in the UX. A user should not mistake API billing totals for ChatGPT included-plan usage.

Implementation note:

- rows originating from the official OpenAI organization APIs should be marked as authoritative `api-mode` data
- `chatgpt-mode` should not produce synthetic totals, placeholder rollups, or estimated usage in v1
- if the product later adds imports or user-entered estimates, those should use a different source kind and value status from authoritative API billing data

This file should remain OpenAI-specific. Cross-provider behavior belongs in the main architecture and plan docs.
