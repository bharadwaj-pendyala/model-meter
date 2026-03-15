use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::process::{self, Command};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue, USER_AGENT};
use serde::Deserialize;
use serde_json::{Value, json};

const PROVIDERS: [ProviderSpec; 4] = [
    ProviderSpec {
        id: "openai",
        display_name: "Codex / OpenAI",
        support_tier: "official-api",
        configured_by: "OPENAI_ADMIN_KEY or Codex CLI login",
        quick_check_hint: "API-backed usage and billing surfaces, plus Codex CLI session detection",
        note: "Codex CLI login can be detected, but ChatGPT-plan Codex usage is still unsupported without a documented machine-readable surface.",
    },
    ProviderSpec {
        id: "claude",
        display_name: "Claude",
        support_tier: "official-api",
        configured_by: "ANTHROPIC_ADMIN_KEY or MODEL_METER_CLAUDE_USED and MODEL_METER_CLAUDE_LIMIT",
        quick_check_hint: "Anthropic Admin API cost report, with optional monthly limit for percentage left",
        note: "Uses the Anthropic Admin Cost API when ANTHROPIC_ADMIN_KEY is available. Manual counters remain the fallback.",
    },
    ProviderSpec {
        id: "cursor",
        display_name: "Cursor",
        support_tier: "official-api",
        configured_by: "CURSOR_ADMIN_API_KEY or MODEL_METER_CURSOR_USED and MODEL_METER_CURSOR_LIMIT",
        quick_check_hint: "Cursor Admin API usage and spend data, with optional limit env vars for percentage left",
        note: "Uses the official Cursor Admin API for team usage metrics. Direct remaining percentage depends on plan limits, so optional env limits may be needed.",
    },
    ProviderSpec {
        id: "windsurf",
        display_name: "Windsurf",
        support_tier: "official-api",
        configured_by: "WINDSURF_SERVICE_KEY or MODEL_METER_WINDSURF_USED and MODEL_METER_WINDSURF_LIMIT",
        quick_check_hint: "Windsurf enterprise usage and credit APIs, with optional user email for per-user usage",
        note: "Uses official Windsurf enterprise APIs when WINDSURF_SERVICE_KEY is available. Manual counters remain the fallback.",
    },
];

#[derive(Clone, Copy)]
struct ProviderSpec {
    id: &'static str,
    display_name: &'static str,
    support_tier: &'static str,
    configured_by: &'static str,
    quick_check_hint: &'static str,
    note: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
struct ManualCounter {
    used: f64,
    limit: f64,
    percentage: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct CodexUsageSnapshot {
    plan_type: Option<String>,
    primary: Option<CodexWindow>,
    secondary: Option<CodexWindow>,
    credits: Option<CodexCredits>,
}

#[derive(Debug, Clone, PartialEq)]
struct CodexWindow {
    used_percent: f64,
    remaining_percent: f64,
    reset_at: Option<i64>,
    reset_after_seconds: Option<i64>,
    window_seconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
struct CodexCredits {
    has_credits: bool,
    unlimited: bool,
    balance: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct ClaudeUsageSnapshot {
    total_cost_usd: f64,
    month_label: String,
    limit_usd: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
struct CursorUsageSnapshot {
    total_requests: i64,
    composer_requests: i64,
    chat_requests: i64,
    agent_requests: i64,
    included_requests: i64,
    usage_based_requests: i64,
    period_start_ms: i64,
    period_end_ms: i64,
    spend_cents: Option<i64>,
    spend_limit_dollars: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
struct WindsurfUsageSnapshot {
    prompt_credits_per_seat: i64,
    num_seats: i64,
    add_on_available_credits: i64,
    add_on_used_credits: i64,
    billing_cycle_start: Option<String>,
    billing_cycle_end: Option<String>,
    user_email: Option<String>,
    user_prompt_credits_used: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CodexAuthFile {
    auth_mode: Option<String>,
    tokens: Option<CodexAuthTokens>,
}

#[derive(Debug, Deserialize)]
struct CodexAuthTokens {
    access_token: Option<String>,
    account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CodexUsageResponse {
    plan_type: Option<String>,
    rate_limit: Option<CodexRateLimitDetails>,
    credits: Option<CodexCreditsResponse>,
}

#[derive(Debug, Deserialize)]
struct CodexRateLimitDetails {
    primary_window: Option<CodexWindowResponse>,
    secondary_window: Option<CodexWindowResponse>,
}

#[derive(Debug, Deserialize)]
struct CodexWindowResponse {
    used_percent: f64,
    reset_at: Option<i64>,
    reset_after_seconds: Option<i64>,
    #[serde(rename = "limit_window_seconds")]
    window_seconds: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CodexCreditsResponse {
    has_credits: bool,
    unlimited: bool,
    balance: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CursorDailyUsageResponse {
    data: Vec<CursorDailyUsageRow>,
}

#[derive(Debug, Deserialize)]
struct CursorDailyUsageRow {
    #[serde(default)]
    #[serde(rename = "composerRequests")]
    composer_requests: Option<i64>,
    #[serde(default)]
    #[serde(rename = "chatRequests")]
    chat_requests: Option<i64>,
    #[serde(default)]
    #[serde(rename = "agentRequests")]
    agent_requests: Option<i64>,
    #[serde(default)]
    #[serde(rename = "subscriptionIncludedReqs")]
    subscription_included_reqs: Option<i64>,
    #[serde(default)]
    #[serde(rename = "usageBasedReqs")]
    usage_based_reqs: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CursorSpendResponse {
    #[serde(default)]
    #[serde(rename = "teamMemberSpend")]
    team_member_spend: Vec<CursorSpendRow>,
}

#[derive(Debug, Deserialize)]
struct CursorSpendRow {
    #[serde(default)]
    #[serde(rename = "spendCents")]
    spend_cents: Option<i64>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    #[serde(rename = "hardLimitOverrideDollars")]
    hard_limit_override_dollars: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ClaudeCostReportResponse {
    #[serde(default)]
    data: Vec<ClaudeCostBucket>,
    #[serde(default)]
    has_more: bool,
    #[serde(default)]
    next_page: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeCostBucket {
    #[serde(default)]
    results: Vec<ClaudeCostResult>,
}

#[derive(Debug, Deserialize)]
struct ClaudeCostResult {
    amount: String,
}

#[derive(Debug, Deserialize)]
struct WindsurfTeamCreditBalanceResponse {
    #[serde(default)]
    #[serde(rename = "promptCreditsPerSeat")]
    prompt_credits_per_seat: Option<i64>,
    #[serde(default)]
    #[serde(rename = "numSeats")]
    num_seats: Option<i64>,
    #[serde(default)]
    #[serde(rename = "addOnCreditsAvailable")]
    add_on_credits_available: Option<i64>,
    #[serde(default)]
    #[serde(rename = "addOnCreditsUsed")]
    add_on_credits_used: Option<i64>,
    #[serde(default)]
    #[serde(rename = "billingCycleStart")]
    billing_cycle_start: Option<String>,
    #[serde(default)]
    #[serde(rename = "billingCycleEnd")]
    billing_cycle_end: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WindsurfUserPageAnalyticsResponse {
    #[serde(default)]
    #[serde(rename = "userTableStats")]
    user_table_stats: Vec<WindsurfUserStat>,
    #[serde(default)]
    #[serde(rename = "billingCycleStart")]
    billing_cycle_start: Option<String>,
    #[serde(default)]
    #[serde(rename = "billingCycleEnd")]
    billing_cycle_end: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WindsurfUserStat {
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    #[serde(rename = "promptCreditsUsed")]
    prompt_credits_used: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
enum ProviderState {
    OfficialApi {
        configured: bool,
        auth_state: &'static str,
        auth_source: &'static str,
        detail: String,
    },
    Manual {
        counter: Option<ManualCounter>,
        state: &'static str,
        detail: String,
    },
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    match run(&args) {
        Ok(output) => {
            println!("{output}");
        }
        Err(err) => {
            eprintln!("{err}");
            process::exit(1);
        }
    }
}

fn run(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Ok(help_text());
    }

    let json = args.iter().any(|arg| arg == "--json");
    let filtered_args: Vec<&str> = args
        .iter()
        .filter(|arg| arg.as_str() != "--json")
        .map(String::as_str)
        .collect();

    match filtered_args.as_slice() {
        ["codex"] => render_codex_usage(json),
        ["cursor"] => render_cursor_usage(json),
        ["claude"] => render_claude_usage(json),
        ["windsurf"] => render_windsurf_usage(json),
        ["providers"] => Ok(render_providers(json)),
        ["auth", "validate"] => render_auth_validation("openai", json),
        ["auth", "validate", provider] => render_auth_validation(provider, json),
        ["usage", "codex"] => render_codex_usage(json),
        ["usage", "openai"] => render_codex_usage(json),
        ["usage", "cursor"] => render_cursor_usage(json),
        ["usage", "claude"] => render_claude_usage(json),
        ["usage", "windsurf"] => render_windsurf_usage(json),
        ["status"] => Ok(render_status(json)),
        ["help"] | ["--help"] | ["-h"] => Ok(help_text()),
        _ => Err(format!(
            "unknown command: {}\n\n{}",
            args.join(" "),
            help_text()
        )),
    }
}

fn help_text() -> String {
    let mut text = String::new();
    text.push_str("model-meter 0.1.0\n");
    text.push_str("Quick usage checks for AI tools.\n\n");
    text.push_str("Commands:\n");
    text.push_str("  model-meter codex [--json]\n");
    text.push_str("  model-meter cursor [--json]\n");
    text.push_str("  model-meter claude [--json]\n");
    text.push_str("  model-meter windsurf [--json]\n");
    text.push_str("  model-meter providers [--json]\n");
    text.push_str("  model-meter auth validate [openai|codex|claude|cursor|windsurf] [--json]\n");
    text.push_str("  model-meter usage codex [--json]\n");
    text.push_str("  model-meter usage cursor [--json]\n");
    text.push_str("  model-meter usage claude [--json]\n");
    text.push_str("  model-meter usage windsurf [--json]\n");
    text.push_str("  model-meter status [--json]\n\n");
    text.push_str("Manual counters:\n");
    text.push_str("  MODEL_METER_CLAUDE_USED / MODEL_METER_CLAUDE_LIMIT\n");
    text.push_str("  MODEL_METER_CURSOR_USED / MODEL_METER_CURSOR_LIMIT\n");
    text.push_str("  MODEL_METER_WINDSURF_USED / MODEL_METER_WINDSURF_LIMIT\n");
    text.push_str("  MODEL_METER_OPENAI_USED / MODEL_METER_OPENAI_LIMIT\n\n");
    text.push_str("OpenAI auth:\n");
    text.push_str("  OPENAI_ADMIN_KEY\n");
    text.push_str("  or an existing Codex CLI login session\n");
    text
}

fn render_codex_usage(json: bool) -> Result<String, String> {
    let snapshot = fetch_codex_usage()?;

    if json {
        return Ok(codex_usage_json(&snapshot));
    }

    Ok(codex_usage_text(&snapshot))
}

fn render_cursor_usage(json: bool) -> Result<String, String> {
    let snapshot = fetch_cursor_usage()?;
    if json {
        return Ok(cursor_usage_json(&snapshot));
    }
    Ok(cursor_usage_text(&snapshot))
}

fn render_claude_usage(json: bool) -> Result<String, String> {
    let snapshot = fetch_claude_usage()?;
    if json {
        return Ok(claude_usage_json(&snapshot));
    }
    Ok(claude_usage_text(&snapshot))
}

fn render_windsurf_usage(json: bool) -> Result<String, String> {
    let snapshot = fetch_windsurf_usage()?;
    if json {
        return Ok(windsurf_usage_json(&snapshot));
    }
    Ok(windsurf_usage_text(&snapshot))
}

fn render_providers(json: bool) -> String {
    if json {
        let entries = PROVIDERS
            .iter()
            .map(|provider| {
                format!(
                    "{{\"id\":\"{}\",\"display_name\":\"{}\",\"support_tier\":\"{}\",\"configured_by\":\"{}\",\"quick_check_hint\":\"{}\",\"note\":\"{}\"}}",
                    escape_json(provider.id),
                    escape_json(provider.display_name),
                    escape_json(provider.support_tier),
                    escape_json(provider.configured_by),
                    escape_json(provider.quick_check_hint),
                    escape_json(provider.note),
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        return format!("{{\"providers\":[{entries}]}}");
    }

    let mut text = String::new();
    text.push_str("Supported providers\n");
    for provider in PROVIDERS {
        let _ = writeln!(
            text,
            "- {} ({})\n  config: {}\n  note: {}",
            provider.display_name, provider.support_tier, provider.configured_by, provider.note
        );
    }
    text.trim_end().to_string()
}

fn render_auth_validation(provider: &str, json: bool) -> Result<String, String> {
    match provider {
        "openai" | "codex" => render_openai_auth_validation(json),
        "claude" => render_claude_auth_validation(json),
        "cursor" => render_cursor_auth_validation(json),
        "windsurf" => render_windsurf_auth_validation(json),
        _ => Err(format!(
            "auth validation is currently implemented only for openai/codex, claude, cursor, and windsurf; got {provider}"
        )),
    }
}

fn render_openai_auth_validation(json: bool) -> Result<String, String> {
    let probe = detect_openai_auth();
    let valid = probe.configured;

    if json {
        return Ok(format!(
            "{{\"provider\":\"openai\",\"valid\":{},\"state\":\"{}\",\"auth_source\":\"{}\",\"detail\":\"{}\"}}",
            valid,
            escape_json(probe.auth_state),
            escape_json(probe.auth_source),
            escape_json(&probe.detail)
        ));
    }

    if valid {
        Ok(format!(
            "openai auth: {}\n{}",
            probe.auth_state, probe.detail
        ))
    } else {
        Err(format!(
            "openai auth: {}\n{}",
            probe.auth_state, probe.detail
        ))
    }
}

fn render_claude_auth_validation(json: bool) -> Result<String, String> {
    let probe = detect_claude_auth();
    let valid = probe.configured;

    if json {
        return Ok(format!(
            "{{\"provider\":\"claude\",\"valid\":{},\"state\":\"{}\",\"detail\":\"{}\"}}",
            valid,
            escape_json(probe.auth_state),
            escape_json(&probe.detail)
        ));
    }

    if valid {
        Ok(format!("claude auth: {}\n{}", probe.auth_state, probe.detail))
    } else {
        Err(format!("claude auth: {}\n{}", probe.auth_state, probe.detail))
    }
}

fn render_cursor_auth_validation(json: bool) -> Result<String, String> {
    let probe = detect_env_auth("CURSOR_ADMIN_API_KEY", "Cursor Admin API key");
    let valid = probe.configured;
    if json {
        return Ok(format!(
            "{{\"provider\":\"cursor\",\"valid\":{},\"state\":\"{}\",\"detail\":\"{}\"}}",
            valid,
            escape_json(probe.auth_state),
            escape_json(&probe.detail)
        ));
    }
    if valid {
        Ok(format!("cursor auth: {}\n{}", probe.auth_state, probe.detail))
    } else {
        Err(format!("cursor auth: {}\n{}", probe.auth_state, probe.detail))
    }
}

fn render_windsurf_auth_validation(json: bool) -> Result<String, String> {
    let probe = detect_env_auth("WINDSURF_SERVICE_KEY", "Windsurf service key");
    let valid = probe.configured;
    if json {
        return Ok(format!(
            "{{\"provider\":\"windsurf\",\"valid\":{},\"state\":\"{}\",\"detail\":\"{}\"}}",
            valid,
            escape_json(probe.auth_state),
            escape_json(&probe.detail)
        ));
    }
    if valid {
        Ok(format!("windsurf auth: {}\n{}", probe.auth_state, probe.detail))
    } else {
        Err(format!("windsurf auth: {}\n{}", probe.auth_state, probe.detail))
    }
}

fn render_status(json: bool) -> String {
    let mut provider_rows = Vec::new();

    for provider in PROVIDERS {
        let state = provider_state(provider);
        provider_rows.push((provider, state));
    }

    if json {
        let providers = provider_rows
            .iter()
            .map(|(provider, state)| provider_state_json(provider, state))
            .collect::<Vec<_>>()
            .join(",");
        return format!("{{\"providers\":[{providers}]}}");
    }

    let mut text = String::new();
    text.push_str("Current provider status\n");

    for (provider, state) in provider_rows {
        match state {
            ProviderState::OfficialApi {
                configured,
                auth_state,
                auth_source,
                detail,
            } => {
                let status = if configured {
                    "configured"
                } else {
                    "not configured"
                };
                let _ = writeln!(
                    text,
                    "- {}: {} ({}, {})\n  {}",
                    provider.display_name, status, auth_state, auth_source, detail
                );
            }
            ProviderState::Manual { counter, state, detail } => {
                if let Some(counter) = counter {
                    let _ = writeln!(
                        text,
                        "- {}: {:.1}% used ({:.2} / {:.2}) [{}]\n  {}",
                        provider.display_name,
                        counter.percentage,
                        counter.used,
                        counter.limit,
                        state,
                        detail
                    );
                } else {
                    let _ = writeln!(
                        text,
                        "- {}: not configured [{}]\n  {}",
                        provider.display_name, state, detail
                    );
                }
            }
        }
    }

    if let Ok(snapshot) = fetch_codex_usage() {
        text.push('\n');
        text.push_str("\nCodex usage snapshot\n");
        append_codex_usage_lines(&mut text, &snapshot);
    }

    text.trim_end().to_string()
}

fn provider_state(provider: ProviderSpec) -> ProviderState {
    match provider.id {
        "openai" => {
            let auth_probe = detect_openai_auth();
            let manual_counter = read_manual_counter("OPENAI");
            let detail = match (auth_probe.configured, manual_counter.as_ref()) {
                (true, Ok(Some(counter))) => format!(
                    "{} Manual quick-check counter: {:.1}% used ({:.2} / {:.2}).",
                    auth_probe.detail,
                    counter.percentage, counter.used, counter.limit
                ),
                (true, Ok(None)) => format!(
                    "{} Add MODEL_METER_OPENAI_USED and MODEL_METER_OPENAI_LIMIT for a local quick-check sample.",
                    auth_probe.detail
                ),
                (false, Ok(Some(counter))) => format!(
                    "{} Manual quick-check counter: {:.1}% used ({:.2} / {:.2}).",
                    auth_probe.detail,
                    counter.percentage, counter.used, counter.limit
                ),
                (false, Ok(None)) => auth_probe.detail.clone(),
                (_, Err(err)) => err.clone(),
            };

            ProviderState::OfficialApi {
                configured: auth_probe.configured,
                auth_state: auth_probe.auth_state,
                auth_source: auth_probe.auth_source,
                detail,
            }
        }
        "claude" => claude_provider_state(provider.note),
        "cursor" => cursor_provider_state(provider.note),
        "windsurf" => windsurf_provider_state(provider.note),
        _ => ProviderState::Manual {
            counter: None,
            state: "unsupported",
            detail: "No provider state is available.".to_string(),
        },
    }
}

fn cursor_provider_state(note: &str) -> ProviderState {
    let auth_probe = detect_env_auth("CURSOR_ADMIN_API_KEY", "Cursor Admin API key");
    let manual_counter = read_manual_counter("CURSOR");
    let detail = match (auth_probe.configured, manual_counter.as_ref()) {
        (true, Ok(Some(counter))) => format!(
            "{} Manual quick-check counter: {:.1}% used ({:.2} / {:.2}).",
            auth_probe.detail, counter.percentage, counter.used, counter.limit
        ),
        (true, Ok(None)) => format!(
            "{} Set CURSOR_REQUEST_LIMIT or CURSOR_SPEND_LIMIT_DOLLARS if you want a percentage-left display from official Cursor usage metrics. {note}",
            auth_probe.detail
        ),
        (false, Ok(Some(counter))) => format!(
            "{} Manual quick-check counter: {:.1}% used ({:.2} / {:.2}).",
            auth_probe.detail, counter.percentage, counter.used, counter.limit
        ),
        (false, Ok(None)) => auth_probe.detail.clone(),
        (_, Err(err)) => err.clone(),
    };
    ProviderState::OfficialApi {
        configured: auth_probe.configured,
        auth_state: auth_probe.auth_state,
        auth_source: auth_probe.auth_source,
        detail,
    }
}

fn windsurf_provider_state(note: &str) -> ProviderState {
    let auth_probe = detect_env_auth("WINDSURF_SERVICE_KEY", "Windsurf service key");
    let manual_counter = read_manual_counter("WINDSURF");
    let detail = match (auth_probe.configured, manual_counter.as_ref()) {
        (true, Ok(Some(counter))) => format!(
            "{} Manual quick-check counter: {:.1}% used ({:.2} / {:.2}).",
            auth_probe.detail, counter.percentage, counter.used, counter.limit
        ),
        (true, Ok(None)) => format!(
            "{} Set WINDSURF_USER_EMAIL for per-user credit usage when available. {note}",
            auth_probe.detail
        ),
        (false, Ok(Some(counter))) => format!(
            "{} Manual quick-check counter: {:.1}% used ({:.2} / {:.2}).",
            auth_probe.detail, counter.percentage, counter.used, counter.limit
        ),
        (false, Ok(None)) => auth_probe.detail.clone(),
        (_, Err(err)) => err.clone(),
    };
    ProviderState::OfficialApi {
        configured: auth_probe.configured,
        auth_state: auth_probe.auth_state,
        auth_source: auth_probe.auth_source,
        detail,
    }
}

fn fetch_codex_usage() -> Result<CodexUsageSnapshot, String> {
    let auth = read_codex_auth_file()?;
    if auth.auth_mode.as_deref() != Some("chatgpt") {
        return Err("Codex usage lookup currently requires a ChatGPT-backed Codex login.".to_string());
    }

    let tokens = auth
        .tokens
        .ok_or_else(|| "Codex auth file does not contain session tokens.".to_string())?;
    let access_token = tokens
        .access_token
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Codex auth file does not contain an access token.".to_string())?;
    let account_id = tokens
        .account_id
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Codex auth file does not contain an account id.".to_string())?;

    let base_url = codex_base_url();
    let url = format!("{}/wham/usage", base_url.trim_end_matches('/'));

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("model-meter"));

    let bearer = format!("Bearer {access_token}");
    let bearer = HeaderValue::from_str(&bearer).map_err(|err| err.to_string())?;
    headers.insert(AUTHORIZATION, bearer);

    let account_header = HeaderName::from_static("chatgpt-account-id");
    let account_value = HeaderValue::from_str(&account_id).map_err(|err| err.to_string())?;
    headers.insert(account_header, account_value);

    let client = Client::builder()
        .build()
        .map_err(|err| format!("failed to build HTTP client: {err}"))?;
    let response = client
        .get(&url)
        .headers(headers)
        .send()
        .map_err(|err| format!("failed to fetch Codex usage: {err}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Codex usage request failed: {status}; {body}"));
    }

    let payload: CodexUsageResponse = response
        .json()
        .map_err(|err| format!("failed to decode Codex usage payload: {err}"))?;

    Ok(CodexUsageSnapshot {
        plan_type: payload.plan_type,
        primary: payload
            .rate_limit
            .as_ref()
            .and_then(|details| details.primary_window.as_ref())
            .map(codex_window_from_response),
        secondary: payload
            .rate_limit
            .as_ref()
            .and_then(|details| details.secondary_window.as_ref())
            .map(codex_window_from_response),
        credits: payload.credits.map(|credits| CodexCredits {
            has_credits: credits.has_credits,
            unlimited: credits.unlimited,
            balance: credits.balance,
        }),
    })
}

fn fetch_cursor_usage() -> Result<CursorUsageSnapshot, String> {
    let key = env_var_required("CURSOR_ADMIN_API_KEY")
        .or_else(|_| env_var_required("CURSOR_API_KEY"))?;
    let client = http_client()?;
    let now_ms = epoch_millis_now()?;
    let start_ms = now_ms - 30 * 24 * 60 * 60 * 1000;

    let usage_response: CursorDailyUsageResponse = cursor_post(
        &client,
        "/teams/daily-usage-data",
        &key,
        json!({
            "startDate": start_ms,
            "endDate": now_ms
        }),
    )?;

    let total_composer_requests: i64 = usage_response
        .data
        .iter()
        .map(|row| row.composer_requests.unwrap_or(0))
        .sum();
    let total_chat_requests: i64 = usage_response
        .data
        .iter()
        .map(|row| row.chat_requests.unwrap_or(0))
        .sum();
    let total_agent_requests: i64 = usage_response
        .data
        .iter()
        .map(|row| row.agent_requests.unwrap_or(0))
        .sum();
    let total_included_requests: i64 = usage_response
        .data
        .iter()
        .map(|row| row.subscription_included_reqs.unwrap_or(0))
        .sum();
    let total_usage_based_requests: i64 = usage_response
        .data
        .iter()
        .map(|row| row.usage_based_reqs.unwrap_or(0))
        .sum();

    let mut spend_cents = None;
    let mut spend_limit_dollars = env::var("CURSOR_SPEND_LIMIT_DOLLARS")
        .ok()
        .and_then(|v| v.parse::<f64>().ok());
    if let Ok(spend_response) = cursor_post::<CursorSpendResponse>(&client, "/teams/spend", &key, json!({})) {
        if let Some(email) = env::var("CURSOR_USER_EMAIL").ok().filter(|v| !v.trim().is_empty()) {
            if let Some(row) = spend_response.team_member_spend.iter().find(|row| row.email.as_deref() == Some(email.as_str())) {
                spend_cents = row.spend_cents;
                if spend_limit_dollars.is_none() {
                    spend_limit_dollars = row.hard_limit_override_dollars;
                }
            }
        } else {
            let total: i64 = spend_response.team_member_spend.iter().map(|row| row.spend_cents.unwrap_or(0)).sum();
            if total > 0 {
                spend_cents = Some(total);
            }
        }
    }

    Ok(CursorUsageSnapshot {
        total_requests: total_composer_requests + total_chat_requests + total_agent_requests,
        composer_requests: total_composer_requests,
        chat_requests: total_chat_requests,
        agent_requests: total_agent_requests,
        included_requests: total_included_requests,
        usage_based_requests: total_usage_based_requests,
        period_start_ms: start_ms,
        period_end_ms: now_ms,
        spend_cents,
        spend_limit_dollars,
    })
}

fn fetch_claude_usage() -> Result<ClaudeUsageSnapshot, String> {
    let key = env_var_required("ANTHROPIC_ADMIN_KEY")
        .or_else(|_| env_var_required("CLAUDE_ADMIN_KEY"))?;
    let client = http_client()?;
    let month_start = current_month_start_utc();
    let now_iso = iso_timestamp_now()?;

    let mut total_cost_usd = 0.0;
    let mut next_page: Option<String> = None;

    loop {
        let mut url = format!(
            "https://api.anthropic.com/v1/organizations/cost_report?starting_at={}&ending_at={}&limit=1000",
            month_start, now_iso
        );
        if let Some(page) = &next_page {
            url.push_str("&page=");
            url.push_str(page);
        }
        let response = client
            .get(&url)
            .header("anthropic-version", "2023-06-01")
            .header("x-api-key", key.as_str())
            .header(USER_AGENT, "model-meter")
            .send()
            .map_err(|err| format!("failed to fetch Claude cost report: {err}"))?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!("Claude cost report request failed: {status}; {body}"));
        }
        let payload: ClaudeCostReportResponse = response
            .json()
            .map_err(|err| format!("failed to decode Claude cost report: {err}"))?;
        for bucket in &payload.data {
            for result in &bucket.results {
                total_cost_usd += result.amount.parse::<f64>().unwrap_or(0.0);
            }
        }
        if payload.has_more {
            next_page = payload.next_page;
        } else {
            break;
        }
    }

    Ok(ClaudeUsageSnapshot {
        total_cost_usd,
        month_label: month_start[..7].to_string(),
        limit_usd: env::var("CLAUDE_MONTHLY_LIMIT_USD")
            .ok()
            .and_then(|v| v.parse::<f64>().ok()),
    })
}

fn fetch_windsurf_usage() -> Result<WindsurfUsageSnapshot, String> {
    let service_key = env_var_required("WINDSURF_SERVICE_KEY")?;
    let client = http_client()?;
    let credit_balance: WindsurfTeamCreditBalanceResponse = windsurf_post(
        &client,
        "/GetTeamCreditBalance",
        json!({ "service_key": service_key }),
    )?;
    if let Some(error) = credit_balance.error.filter(|e| !e.trim().is_empty()) {
        return Err(format!("Windsurf GetTeamCreditBalance error: {error}"));
    }

    let user_email = env::var("WINDSURF_USER_EMAIL").ok().filter(|v| !v.trim().is_empty());
    let analytics: WindsurfUserPageAnalyticsResponse = windsurf_post(
        &client,
        "/UserPageAnalytics",
        json!({ "service_key": service_key }),
    )?;
    if let Some(error) = analytics.error.filter(|e| !e.trim().is_empty()) {
        return Err(format!("Windsurf UserPageAnalytics error: {error}"));
    }

    let user_prompt_credits_used = match &user_email {
        Some(email) => analytics
            .user_table_stats
            .iter()
            .find(|row| row.email.as_deref() == Some(email.as_str()))
            .and_then(|row| row.prompt_credits_used),
        None => None,
    };

    Ok(WindsurfUsageSnapshot {
        prompt_credits_per_seat: credit_balance.prompt_credits_per_seat.unwrap_or(0),
        num_seats: credit_balance.num_seats.unwrap_or(0),
        add_on_available_credits: credit_balance.add_on_credits_available.unwrap_or(0),
        add_on_used_credits: credit_balance.add_on_credits_used.unwrap_or(0),
        billing_cycle_start: credit_balance.billing_cycle_start.or(analytics.billing_cycle_start),
        billing_cycle_end: credit_balance.billing_cycle_end.or(analytics.billing_cycle_end),
        user_email,
        user_prompt_credits_used,
    })
}

fn codex_window_from_response(window: &CodexWindowResponse) -> CodexWindow {
    CodexWindow {
        used_percent: window.used_percent,
        remaining_percent: (100.0 - window.used_percent).clamp(0.0, 100.0),
        reset_at: window.reset_at,
        reset_after_seconds: window.reset_after_seconds,
        window_seconds: window.window_seconds,
    }
}

fn codex_usage_text(snapshot: &CodexUsageSnapshot) -> String {
    let mut text = String::new();
    text.push_str("Codex usage\n");
    append_codex_usage_lines(&mut text, snapshot);
    text.trim_end().to_string()
}

fn append_codex_usage_lines(text: &mut String, snapshot: &CodexUsageSnapshot) {
    if let Some(plan) = &snapshot.plan_type {
        let _ = writeln!(text, "- plan: {plan}");
    }
    if let Some(primary) = &snapshot.primary {
        let _ = writeln!(
            text,
            "- 5h window: {:.0}% left ({:.0}% used), resets {}",
            primary.remaining_percent,
            primary.used_percent,
            format_reset_detail(primary)
        );
    }
    if let Some(secondary) = &snapshot.secondary {
        let _ = writeln!(
            text,
            "- weekly window: {:.0}% left ({:.0}% used), resets {}",
            secondary.remaining_percent,
            secondary.used_percent,
            format_reset_detail(secondary)
        );
    }
    if let Some(credits) = &snapshot.credits
        && credits.has_credits
    {
        let credits_text = if credits.unlimited {
            "unlimited".to_string()
        } else {
            credits.balance.clone().unwrap_or_else(|| "unknown".to_string())
        };
        let _ = writeln!(text, "- credits: {credits_text}");
    }
}

fn codex_usage_json(snapshot: &CodexUsageSnapshot) -> String {
    let plan_json = match &snapshot.plan_type {
        Some(plan) => format!("\"{}\"", escape_json(plan)),
        None => "null".to_string(),
    };
    let primary_json = match &snapshot.primary {
        Some(window) => codex_window_json(window),
        None => "null".to_string(),
    };
    let secondary_json = match &snapshot.secondary {
        Some(window) => codex_window_json(window),
        None => "null".to_string(),
    };
    let credits_json = match &snapshot.credits {
        Some(credits) => format!(
            "{{\"has_credits\":{},\"unlimited\":{},\"balance\":{}}}",
            credits.has_credits,
            credits.unlimited,
            match &credits.balance {
                Some(balance) => format!("\"{}\"", escape_json(balance)),
                None => "null".to_string(),
            }
        ),
        None => "null".to_string(),
    };

    format!(
        "{{\"provider\":\"codex\",\"plan_type\":{},\"primary_window\":{},\"secondary_window\":{},\"credits\":{}}}",
        plan_json, primary_json, secondary_json, credits_json
    )
}

fn codex_window_json(window: &CodexWindow) -> String {
    let reset_at = match window.reset_at {
        Some(value) => value.to_string(),
        None => "null".to_string(),
    };
    let reset_after = match window.reset_after_seconds {
        Some(value) => value.to_string(),
        None => "null".to_string(),
    };
    let window_seconds = match window.window_seconds {
        Some(value) => value.to_string(),
        None => "null".to_string(),
    };

    format!(
        "{{\"used_percent\":{:.1},\"remaining_percent\":{:.1},\"reset_at\":{},\"reset_after_seconds\":{},\"window_seconds\":{}}}",
        window.used_percent,
        window.remaining_percent,
        reset_at,
        reset_after,
        window_seconds
    )
}

fn format_reset_detail(window: &CodexWindow) -> String {
    if let Some(seconds) = window.reset_after_seconds {
        return format!("in {}", format_duration_seconds(seconds));
    }
    if let Some(timestamp) = window.reset_at {
        return format!("at unix {}", timestamp);
    }
    "at an unknown time".to_string()
}

fn format_duration_seconds(total_seconds: i64) -> String {
    let hours = total_seconds / 3600;
    let days = hours / 24;
    let remaining_hours = hours % 24;
    let minutes = (total_seconds % 3600) / 60;
    if days > 0 && remaining_hours > 0 {
        format!("{days}d {remaining_hours}h")
    } else if days > 0 {
        format!("{days}d")
    } else if hours > 0 && minutes > 0 {
        format!("{hours}h {minutes}m")
    } else if hours > 0 {
        format!("{hours}h")
    } else {
        format!("{minutes}m")
    }
}

fn read_codex_auth_file() -> Result<CodexAuthFile, String> {
    let path = codex_auth_file_path()?;
    let contents = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&contents).map_err(|err| format!("failed to parse Codex auth file: {err}"))
}

fn codex_auth_file_path() -> Result<PathBuf, String> {
    let home = env::var("HOME").map_err(|_| "HOME is not set.".to_string())?;
    Ok(PathBuf::from(home).join(".codex").join("auth.json"))
}

fn codex_base_url() -> String {
    env::var("MODEL_METER_CODEX_BASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "https://chatgpt.com/backend-api".to_string())
}

fn claude_provider_state(note: &str) -> ProviderState {
    let auth_probe = detect_claude_auth();
    let manual_counter = read_manual_counter("CLAUDE");
    let detail = match (auth_probe.configured, manual_counter.as_ref()) {
        (true, Ok(Some(counter))) => format!(
            "{} Manual quick-check counter: {:.1}% used ({:.2} / {:.2}).",
            auth_probe.detail, counter.percentage, counter.used, counter.limit
        ),
        (true, Ok(None)) => format!(
            "{} Set CLAUDE_MONTHLY_LIMIT_USD if you want percentage-left output for monthly Claude costs. {note}",
            auth_probe.detail
        ),
        (false, Ok(Some(counter))) => format!(
            "{} Manual quick-check counter: {:.1}% used ({:.2} / {:.2}).",
            auth_probe.detail, counter.percentage, counter.used, counter.limit
        ),
        (false, Ok(None)) => auth_probe.detail.clone(),
        (_, Err(err)) => err.clone(),
    };
    ProviderState::OfficialApi {
        configured: auth_probe.configured,
        auth_state: auth_probe.auth_state,
        auth_source: auth_probe.auth_source,
        detail,
    }
}

fn provider_state_json(provider: &ProviderSpec, state: &ProviderState) -> String {
    match state {
        ProviderState::OfficialApi {
            configured,
            auth_state,
            auth_source,
            detail,
        } => {
            let manual = read_manual_counter("OPENAI").ok().flatten();
            let manual_json = match manual {
                Some(counter) => format!(
                    ",\"manual_counter\":{{\"used\":{:.2},\"limit\":{:.2},\"percentage\":{:.1}}}",
                    counter.used, counter.limit, counter.percentage
                ),
                None => String::new(),
            };

            format!(
                "{{\"id\":\"{}\",\"display_name\":\"{}\",\"support_tier\":\"{}\",\"configured\":{},\"auth_state\":\"{}\",\"auth_source\":\"{}\",\"detail\":\"{}\"{}}}",
                escape_json(provider.id),
                escape_json(provider.display_name),
                escape_json(provider.support_tier),
                configured,
                escape_json(auth_state),
                escape_json(auth_source),
                escape_json(detail),
                manual_json
            )
        }
        ProviderState::Manual {
            counter,
            state,
            detail,
        } => {
            let counter_json = match counter {
                Some(counter) => format!(
                    "\"counter\":{{\"used\":{:.2},\"limit\":{:.2},\"percentage\":{:.1}}},",
                    counter.used, counter.limit, counter.percentage
                ),
                None => String::new(),
            };

            format!(
                "{{\"id\":\"{}\",\"display_name\":\"{}\",\"support_tier\":\"{}\",{}\"state\":\"{}\",\"detail\":\"{}\"}}",
                escape_json(provider.id),
                escape_json(provider.display_name),
                escape_json(provider.support_tier),
                counter_json,
                escape_json(state),
                escape_json(detail)
            )
        }
    }
}

struct OpenAiAuthProbe {
    configured: bool,
    auth_state: &'static str,
    auth_source: &'static str,
    detail: String,
}

struct ClaudeCapabilityProbe {
    configured: bool,
    auth_state: &'static str,
    auth_source: &'static str,
    detail: String,
}

fn detect_env_auth(key: &str, label: &str) -> OpenAiAuthProbe {
    if env::var(key).ok().filter(|v| !v.trim().is_empty()).is_some() {
        OpenAiAuthProbe {
            configured: true,
            auth_state: "configured",
            auth_source: "env",
            detail: format!("{label} is present."),
        }
    } else {
        let label_text = label.to_lowercase();
        OpenAiAuthProbe {
            configured: false,
            auth_state: "missing",
            auth_source: "none",
            detail: format!("Set {key} to enable the official {label_text} integration."),
        }
    }
}

fn detect_openai_auth() -> OpenAiAuthProbe {
    if let Some(value) = env::var("OPENAI_ADMIN_KEY").ok().filter(|v| !v.trim().is_empty()) {
        let _ = value;
        return OpenAiAuthProbe {
            configured: true,
            auth_state: "configured",
            auth_source: "env",
            detail: "OPENAI_ADMIN_KEY is present. Codex CLI login is not required for API-backed work.".to_string(),
        };
    }

    if !command_exists("codex") {
        return OpenAiAuthProbe {
            configured: false,
            auth_state: "missing",
            auth_source: "none",
            detail: "Set OPENAI_ADMIN_KEY for API-backed work, or log in via the Codex CLI. ChatGPT-plan Codex usage still has no documented machine-readable usage export in this sample.".to_string(),
        };
    }

    match run_command("codex", &["login", "status"]) {
        Ok(output) if output.contains("Logged in") => OpenAiAuthProbe {
            configured: true,
            auth_state: "configured",
            auth_source: "codex-cli-session",
            detail: "Codex CLI login was detected through `codex login status`. This is enough for session-aware onboarding, but this sample does not have a documented Codex usage command to pull ChatGPT-plan usage automatically.".to_string(),
        },
        Ok(output) => OpenAiAuthProbe {
            configured: false,
            auth_state: "missing",
            auth_source: "none",
            detail: format!(
                "Codex CLI is installed, but no logged-in session was detected. Output: {}",
                output.trim()
            ),
        },
        Err(err) => OpenAiAuthProbe {
            configured: false,
            auth_state: "unknown",
            auth_source: "codex-cli-session",
            detail: format!(
                "Codex CLI is installed, but session status could not be confirmed: {err}"
            ),
        },
    }
}

fn detect_claude_auth() -> ClaudeCapabilityProbe {
    if env::var("ANTHROPIC_ADMIN_KEY")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .is_some()
        || env::var("CLAUDE_ADMIN_KEY")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .is_some()
    {
        return ClaudeCapabilityProbe {
            configured: true,
            auth_state: "configured",
            auth_source: "env",
            detail: "Anthropic Admin API key is present. Claude usage and cost data can be fetched through the official admin cost API.".to_string(),
        };
    }
    ClaudeCapabilityProbe {
        configured: false,
        auth_state: "missing",
        auth_source: "none",
        detail: "Set ANTHROPIC_ADMIN_KEY to enable the official Claude cost integration, or use MODEL_METER_CLAUDE_USED and MODEL_METER_CLAUDE_LIMIT for a manual quick check.".to_string(),
    }
}

fn http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| format!("failed to build HTTP client: {err}"))
}

fn env_var_required(key: &str) -> Result<String, String> {
    env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{key} is not set."))
}

fn epoch_millis_now() -> Result<i64, String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?;
    Ok(duration.as_millis() as i64)
}

fn iso_timestamp_now() -> Result<String, String> {
    let now = epoch_millis_now()? / 1000;
    Ok(run_command("date", &["-u", "-r", &now.to_string(), "+%Y-%m-%dT%H:%M:%SZ"])
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string()))
}

fn current_month_start_utc() -> String {
    run_command("date", &["-u", "+%Y-%m-01T00:00:00Z"])
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn cursor_post<T: for<'de> Deserialize<'de>>(
    client: &Client,
    path: &str,
    key: &str,
    body: Value,
) -> Result<T, String> {
    let response = client
        .post(format!("https://api.cursor.com{path}"))
        .basic_auth(key, Some(""))
        .json(&body)
        .send()
        .map_err(|err| format!("failed to call Cursor API {path}: {err}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Cursor API request failed for {path}: {status}; {body}"));
    }
    response
        .json::<T>()
        .map_err(|err| format!("failed to decode Cursor API response for {path}: {err}"))
}

fn windsurf_post<T: for<'de> Deserialize<'de>>(
    client: &Client,
    path: &str,
    body: Value,
) -> Result<T, String> {
    let response = client
        .post(format!("https://server.codeium.com/api/v1{path}"))
        .json(&body)
        .send()
        .map_err(|err| format!("failed to call Windsurf API {path}: {err}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Windsurf API request failed for {path}: {status}; {body}"));
    }
    response
        .json::<T>()
        .map_err(|err| format!("failed to decode Windsurf API response for {path}: {err}"))
}

fn cursor_usage_text(snapshot: &CursorUsageSnapshot) -> String {
    let mut text = String::new();
    text.push_str("Cursor usage\n");
    let _ = writeln!(text, "- total requests (30d): {}", snapshot.total_requests);
    let _ = writeln!(text, "- composer requests: {}", snapshot.composer_requests);
    let _ = writeln!(text, "- chat requests: {}", snapshot.chat_requests);
    let _ = writeln!(text, "- agent requests: {}", snapshot.agent_requests);
    if snapshot.included_requests > 0 || snapshot.usage_based_requests > 0 {
        let _ = writeln!(text, "- included requests: {}", snapshot.included_requests);
        let _ = writeln!(text, "- usage-based requests: {}", snapshot.usage_based_requests);
    }
    if let Some(limit) = env::var("CURSOR_REQUEST_LIMIT").ok().and_then(|v| v.parse::<f64>().ok()) {
        let used = snapshot.total_requests as f64;
        let left = (100.0 - ((used / limit) * 100.0)).clamp(0.0, 100.0);
        let _ = writeln!(text, "- request limit: {:.0}% left ({:.0} / {:.0})", left, used, limit);
    }
    if let Some(spend_cents) = snapshot.spend_cents {
        let spend_dollars = spend_cents as f64 / 100.0;
        if let Some(limit) = snapshot.spend_limit_dollars {
            let left = (100.0 - ((spend_dollars / limit) * 100.0)).clamp(0.0, 100.0);
            let _ = writeln!(text, "- spend limit: {:.0}% left (${:.2} / ${:.2})", left, spend_dollars, limit);
        } else {
            let _ = writeln!(text, "- spend this cycle: ${:.2}", spend_dollars);
        }
    }
    text.trim_end().to_string()
}

fn cursor_usage_json(snapshot: &CursorUsageSnapshot) -> String {
    format!(
        "{{\"provider\":\"cursor\",\"total_requests\":{},\"composer_requests\":{},\"chat_requests\":{},\"agent_requests\":{},\"included_requests\":{},\"usage_based_requests\":{},\"spend_cents\":{},\"spend_limit_dollars\":{}}}",
        snapshot.total_requests,
        snapshot.composer_requests,
        snapshot.chat_requests,
        snapshot.agent_requests,
        snapshot.included_requests,
        snapshot.usage_based_requests,
        snapshot.spend_cents.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
        snapshot.spend_limit_dollars.map(|v| format!("{v:.2}")).unwrap_or_else(|| "null".to_string()),
    )
}

fn claude_usage_text(snapshot: &ClaudeUsageSnapshot) -> String {
    let mut text = String::new();
    text.push_str("Claude usage\n");
    let _ = writeln!(text, "- month: {}", snapshot.month_label);
    if let Some(limit) = snapshot.limit_usd {
        let left = (100.0 - ((snapshot.total_cost_usd / limit) * 100.0)).clamp(0.0, 100.0);
        let _ = writeln!(text, "- monthly limit: {:.0}% left (${:.2} / ${:.2})", left, snapshot.total_cost_usd, limit);
    } else {
        let _ = writeln!(text, "- cost this month: ${:.2}", snapshot.total_cost_usd);
    }
    text.trim_end().to_string()
}

fn claude_usage_json(snapshot: &ClaudeUsageSnapshot) -> String {
    format!(
        "{{\"provider\":\"claude\",\"month\":\"{}\",\"total_cost_usd\":{:.4},\"limit_usd\":{}}}",
        escape_json(&snapshot.month_label),
        snapshot.total_cost_usd,
        snapshot.limit_usd.map(|v| format!("{v:.2}")).unwrap_or_else(|| "null".to_string()),
    )
}

fn windsurf_usage_text(snapshot: &WindsurfUsageSnapshot) -> String {
    let mut text = String::new();
    text.push_str("Windsurf usage\n");
    let _ = writeln!(text, "- prompt credits per seat: {}", snapshot.prompt_credits_per_seat);
    let _ = writeln!(text, "- seats: {}", snapshot.num_seats);
    let _ = writeln!(
        text,
        "- add-on credits: {} left ({} used)",
        snapshot.add_on_available_credits,
        snapshot.add_on_used_credits
    );
    if let Some(email) = &snapshot.user_email {
        if let Some(used) = snapshot.user_prompt_credits_used {
            let left = (snapshot.prompt_credits_per_seat - used).max(0);
            let percent_left = if snapshot.prompt_credits_per_seat > 0 {
                (left as f64 / snapshot.prompt_credits_per_seat as f64) * 100.0
            } else {
                0.0
            };
            let _ = writeln!(text, "- user: {}", email);
            let _ = writeln!(text, "- per-seat credits: {:.0}% left ({} / {})", percent_left, used, snapshot.prompt_credits_per_seat);
        } else {
            let _ = writeln!(text, "- user: {} (not found in analytics response)", email);
        }
    } else {
        text.push_str("- set WINDSURF_USER_EMAIL for per-user prompt credit usage\n");
    }
    text.trim_end().to_string()
}

fn windsurf_usage_json(snapshot: &WindsurfUsageSnapshot) -> String {
    format!(
        "{{\"provider\":\"windsurf\",\"prompt_credits_per_seat\":{},\"num_seats\":{},\"add_on_available_credits\":{},\"add_on_used_credits\":{},\"user_email\":{},\"user_prompt_credits_used\":{}}}",
        snapshot.prompt_credits_per_seat,
        snapshot.num_seats,
        snapshot.add_on_available_credits,
        snapshot.add_on_used_credits,
        snapshot.user_email.as_ref().map(|v| format!("\"{}\"", escape_json(v))).unwrap_or_else(|| "null".to_string()),
        snapshot.user_prompt_credits_used.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
    )
}

fn command_exists(command: &str) -> bool {
    Command::new(command)
        .arg("--help")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn run_command(command: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .map_err(|err| err.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        if stdout.is_empty() {
            Ok(stderr)
        } else {
            Ok(stdout)
        }
    } else if stderr.is_empty() {
        Err(stdout)
    } else {
        Err(stderr)
    }
}

fn read_manual_counter(prefix: &str) -> Result<Option<ManualCounter>, String> {
    let used_key = format!("MODEL_METER_{prefix}_USED");
    let limit_key = format!("MODEL_METER_{prefix}_LIMIT");

    let used = env::var(&used_key).ok();
    let limit = env::var(&limit_key).ok();

    match (used, limit) {
        (None, None) => Ok(None),
        (Some(_), None) | (None, Some(_)) => Err(format!(
            "both {used_key} and {limit_key} must be set together"
        )),
        (Some(used), Some(limit)) => {
            let used_value = parse_number(&used_key, &used)?;
            let limit_value = parse_number(&limit_key, &limit)?;
            if limit_value <= 0.0 {
                return Err(format!("{limit_key} must be greater than 0"));
            }

            Ok(Some(ManualCounter {
                used: used_value,
                limit: limit_value,
                percentage: (used_value / limit_value) * 100.0,
            }))
        }
    }
}

fn parse_number(key: &str, value: &str) -> Result<f64, String> {
    value
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("{key} must be a number, got {value:?}"))
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_manual_counter() {
        unsafe {
            env::set_var("MODEL_METER_CLAUDE_USED", "25");
            env::set_var("MODEL_METER_CLAUDE_LIMIT", "100");
        }

        let counter = read_manual_counter("CLAUDE").unwrap().unwrap();
        assert_eq!(counter.used, 25.0);
        assert_eq!(counter.limit, 100.0);
        assert_eq!(counter.percentage, 25.0);

        unsafe {
            env::remove_var("MODEL_METER_CLAUDE_USED");
            env::remove_var("MODEL_METER_CLAUDE_LIMIT");
        }
    }

    #[test]
    fn rejects_partial_manual_counter() {
        unsafe {
            env::set_var("MODEL_METER_CURSOR_USED", "4");
            env::remove_var("MODEL_METER_CURSOR_LIMIT");
        }

        let err = read_manual_counter("CURSOR").unwrap_err();
        assert!(err.contains("must be set together"));

        unsafe {
            env::remove_var("MODEL_METER_CURSOR_USED");
        }
    }

    #[test]
    fn renders_help_for_empty_args() {
        let output = run(&[]).unwrap();
        assert!(output.contains("model-meter 0.1.0"));
        assert!(output.contains("model-meter status"));
        assert!(output.contains("model-meter codex"));
    }

    #[test]
    fn short_codex_command_routes_to_usage() {
        let short = run(&["codex".to_string()]);
        let long = run(&["usage".to_string(), "codex".to_string()]);
        assert_eq!(short.is_ok(), long.is_ok());
        match (short, long) {
            (Ok(short_output), Ok(long_output)) => assert_eq!(short_output, long_output),
            (Err(short_err), Err(long_err)) => assert_eq!(short_err, long_err),
            _ => panic!("short and long codex commands should behave the same"),
        }
    }

    #[test]
    fn formats_long_duration_in_days() {
        assert_eq!(format_duration_seconds(446_159), "5d 3h");
    }

    #[test]
    fn maps_codex_window_to_remaining_percent() {
        let window = CodexWindowResponse {
            used_percent: 17.0,
            reset_at: Some(100),
            reset_after_seconds: Some(200),
            window_seconds: Some(300),
        };

        let snapshot = codex_window_from_response(&window);
        assert_eq!(snapshot.used_percent, 17.0);
        assert_eq!(snapshot.remaining_percent, 83.0);
        assert_eq!(snapshot.reset_at, Some(100));
        assert_eq!(snapshot.reset_after_seconds, Some(200));
        assert_eq!(snapshot.window_seconds, Some(300));
    }
}
