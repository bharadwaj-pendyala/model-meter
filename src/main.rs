use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue, USER_AGENT};
use serde::Deserialize;

const PROVIDERS: [ProviderSpec; 4] = [
    ProviderSpec {
        id: "codex",
        display_name: "Codex / OpenAI",
        support_tier: "session-usage",
        quick_check_hint: "Reads usage from an existing local Codex ChatGPT session.",
        note: "This is the strongest working provider today.",
    },
    ProviderSpec {
        id: "cursor",
        display_name: "Cursor",
        support_tier: "session-account",
        quick_check_hint: "Detects local Cursor auth state and plan from app storage.",
        note: "Usage percent is not exposed through the local session data this build can read yet.",
    },
    ProviderSpec {
        id: "claude",
        display_name: "Claude",
        support_tier: "session-account",
        quick_check_hint: "Detects local Claude session markers and plan metadata from app storage.",
        note: "Usage percent is not exposed through the local session data this build can read yet.",
    },
    ProviderSpec {
        id: "windsurf",
        display_name: "Windsurf",
        support_tier: "session-probe",
        quick_check_hint: "Probes common local Windsurf / Codeium install and data paths.",
        note: "Local session reuse is not implemented yet because a stable session store has not been mapped.",
    },
];

#[derive(Clone, Copy)]
struct ProviderSpec {
    id: &'static str,
    display_name: &'static str,
    support_tier: &'static str,
    quick_check_hint: &'static str,
    note: &'static str,
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

#[derive(Debug, Clone, PartialEq)]
struct LocalSessionProbe {
    provider: &'static str,
    state: &'static str,
    session_detected: bool,
    usage_available: bool,
    auth_source: &'static str,
    detail: String,
    email: Option<String>,
    plan: Option<String>,
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
        ["codex"] => render_provider_usage("codex", json),
        ["cursor"] => render_provider_usage("cursor", json),
        ["claude"] => render_provider_usage("claude", json),
        ["windsurf"] => render_provider_usage("windsurf", json),
        ["providers"] => Ok(render_providers(json)),
        ["auth", "validate"] => render_auth_validation("codex", json),
        ["auth", "validate", provider] => render_auth_validation(provider, json),
        ["usage", provider] => render_provider_usage(provider, json),
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
    text.push_str("Model-agnostic usage checks from local AI tool sessions.\n\n");
    text.push_str("Commands:\n");
    text.push_str("  model-meter codex [--json]\n");
    text.push_str("  model-meter cursor [--json]\n");
    text.push_str("  model-meter claude [--json]\n");
    text.push_str("  model-meter windsurf [--json]\n");
    text.push_str("  model-meter usage [codex|cursor|claude|windsurf] [--json]\n");
    text.push_str("  model-meter auth validate [codex|cursor|claude|windsurf|openai] [--json]\n");
    text.push_str("  model-meter providers [--json]\n");
    text.push_str("  model-meter status [--json]\n\n");
    text.push_str("Current behavior:\n");
    text.push_str("  - Codex: working local-session usage snapshot\n");
    text.push_str("  - Cursor: local-session account detection and plan\n");
    text.push_str("  - Claude: local-session account detection and plan\n");
    text.push_str("  - Windsurf: local install/session probing only\n");
    text
}

fn render_provider_usage(provider: &str, json: bool) -> Result<String, String> {
    match provider {
        "codex" | "openai" => render_codex_usage(json),
        "cursor" => Ok(render_local_probe_usage(&detect_cursor_session(), json)),
        "claude" => Ok(render_local_probe_usage(&detect_claude_session(), json)),
        "windsurf" => Ok(render_local_probe_usage(&detect_windsurf_session(), json)),
        _ => Err(format!("usage is not implemented for provider {provider}")),
    }
}

fn render_codex_usage(json: bool) -> Result<String, String> {
    let snapshot = fetch_codex_usage()?;

    if json {
        return Ok(codex_usage_json(&snapshot));
    }

    Ok(codex_usage_text(&snapshot))
}

fn render_local_probe_usage(probe: &LocalSessionProbe, json: bool) -> String {
    if json {
        return local_probe_json(probe);
    }

    let mut text = String::new();
    let _ = writeln!(text, "{} usage", provider_title(probe.provider));

    if let Some(email) = &probe.email {
        let _ = writeln!(text, "- account: {email}");
    }
    if let Some(plan) = &probe.plan {
        let _ = writeln!(text, "- plan: {plan}");
    }

    if probe.usage_available {
        let _ = writeln!(text, "- usage: available");
    } else if probe.session_detected {
        let _ = writeln!(text, "- usage: not exposed through the local session yet");
    } else {
        let _ = writeln!(text, "- usage: session not detected");
    }

    let _ = writeln!(text, "- state: {}", probe.state);
    let _ = writeln!(text, "- detail: {}", probe.detail);
    text.trim_end().to_string()
}

fn render_providers(json: bool) -> String {
    if json {
        let entries = PROVIDERS
            .iter()
            .map(|provider| {
                format!(
                    "{{\"id\":\"{}\",\"display_name\":\"{}\",\"support_tier\":\"{}\",\"quick_check_hint\":\"{}\",\"note\":\"{}\"}}",
                    escape_json(provider.id),
                    escape_json(provider.display_name),
                    escape_json(provider.support_tier),
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
            "- {} ({})\n  hint: {}\n  note: {}",
            provider.display_name, provider.support_tier, provider.quick_check_hint, provider.note
        );
    }
    text.trim_end().to_string()
}

fn render_auth_validation(provider: &str, json: bool) -> Result<String, String> {
    let probe = match provider {
        "openai" | "codex" => detect_codex_session(),
        "cursor" => detect_cursor_session(),
        "claude" => detect_claude_session(),
        "windsurf" => detect_windsurf_session(),
        _ => {
            return Err(format!(
                "auth validation is currently implemented for codex, cursor, claude, and windsurf; got {provider}"
            ));
        }
    };

    if json {
        return Ok(local_probe_json(&probe));
    }

    let heading = format!("{} auth", provider_title(probe.provider).to_lowercase());
    if probe.session_detected {
        Ok(format!("{heading}: {}\n{}", probe.state, probe.detail))
    } else {
        Err(format!("{heading}: {}\n{}", probe.state, probe.detail))
    }
}

fn render_status(json: bool) -> String {
    let probes = vec![
        detect_codex_session(),
        detect_cursor_session(),
        detect_claude_session(),
        detect_windsurf_session(),
    ];

    if json {
        let providers = probes
            .iter()
            .map(local_probe_json)
            .collect::<Vec<_>>()
            .join(",");
        return format!("{{\"providers\":[{providers}]}}");
    }

    let mut text = String::new();
    text.push_str("Current provider status\n");

    for probe in probes {
        let status = if probe.session_detected {
            "session detected"
        } else {
            "session not detected"
        };
        let usage = if probe.usage_available {
            "usage available"
        } else {
            "usage unavailable"
        };

        let _ = writeln!(
            text,
            "- {}: {} ({}, {})\n  {}",
            provider_title(probe.provider),
            status,
            probe.state,
            usage,
            probe.detail
        );
    }

    if let Ok(snapshot) = fetch_codex_usage() {
        text.push('\n');
        text.push_str("Codex usage snapshot\n");
        append_codex_usage_lines(&mut text, &snapshot);
    }

    text.trim_end().to_string()
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

fn detect_codex_session() -> LocalSessionProbe {
    if let Ok(auth) = read_codex_auth_file()
        && auth.auth_mode.as_deref() == Some("chatgpt")
        && let Some(tokens) = auth.tokens
        && tokens.access_token.as_deref().is_some_and(|value| !value.trim().is_empty())
    {
        return LocalSessionProbe {
            provider: "codex",
            state: "session-detected",
            session_detected: true,
            usage_available: true,
            auth_source: "codex-auth-file",
            detail: "Local Codex ChatGPT session detected through ~/.codex/auth.json.".to_string(),
            email: None,
            plan: None,
        };
    }

    LocalSessionProbe {
        provider: "codex",
        state: "missing",
        session_detected: false,
        usage_available: false,
        auth_source: "none",
        detail: "Run `codex login` first so model-meter can reuse the local Codex session.".to_string(),
        email: None,
        plan: None,
    }
}

fn detect_cursor_session() -> LocalSessionProbe {
    let path = match cursor_state_db_path() {
        Some(path) => path,
        None => {
            return LocalSessionProbe {
                provider: "cursor",
                state: "missing",
                session_detected: false,
                usage_available: false,
                auth_source: "none",
                detail: "Cursor user state was not found in the standard local app data path.".to_string(),
                email: None,
                plan: None,
            };
        }
    };

    let email = sqlite_query_scalar(&path, "ItemTable", "cursorAuth/cachedEmail");
    let plan = sqlite_query_scalar(&path, "ItemTable", "cursorAuth/stripeMembershipType");
    let session_detected = email.is_some() || plan.is_some();

    if !session_detected {
        return LocalSessionProbe {
            provider: "cursor",
            state: "session-unknown",
            session_detected: false,
            usage_available: false,
            auth_source: "cursor-state-db",
            detail: format!(
                "Cursor state DB exists at {}, but no cached account markers were found.",
                path.display()
            ),
            email: None,
            plan: None,
        };
    }

    let detail = match (&email, &plan) {
        (Some(email), Some(plan)) => format!(
            "Local Cursor session markers were detected in {} for {email} on the {plan} plan. Current local state does not expose quota or remaining percentage yet.",
            path.display()
        ),
        (Some(email), None) => format!(
            "Local Cursor session markers were detected in {} for {email}. Current local state does not expose quota or remaining percentage yet.",
            path.display()
        ),
        (None, Some(plan)) => format!(
            "Local Cursor session markers were detected in {} with plan {plan}. Current local state does not expose quota or remaining percentage yet.",
            path.display()
        ),
        (None, None) => unreachable!(),
    };

    LocalSessionProbe {
        provider: "cursor",
        state: "session-detected",
        session_detected: true,
        usage_available: false,
        auth_source: "cursor-state-db",
        detail,
        email,
        plan,
    }
}

fn detect_claude_session() -> LocalSessionProbe {
    let app_dir = match claude_app_support_dir() {
        Some(path) => path,
        None => {
            return LocalSessionProbe {
                provider: "claude",
                state: "missing",
                session_detected: false,
                usage_available: false,
                auth_source: "none",
                detail: "Claude app data was not found in the standard local app data path.".to_string(),
                email: None,
                plan: None,
            };
        }
    };

    let metadata = read_claude_local_metadata(&app_dir);
    let session_cookie_present = claude_cookie_marker_exists(&app_dir);
    let session_detected = session_cookie_present || metadata.email.is_some() || metadata.plan.is_some();

    if !session_detected {
        return LocalSessionProbe {
            provider: "claude",
            state: "session-unknown",
            session_detected: false,
            usage_available: false,
            auth_source: "claude-local-storage",
            detail: format!(
                "Claude app data exists at {}, but no reusable local session markers were found.",
                app_dir.display()
            ),
            email: None,
            plan: None,
        };
    }

    let mut detail = format!(
        "Local Claude desktop session markers were detected in {}.",
        app_dir.display()
    );
    if let Some(org_type) = &metadata.org_type {
        let _ = write!(detail, " Organization type: {org_type}.");
    }
    detail.push_str(" Current local state does not expose quota or remaining percentage yet.");

    LocalSessionProbe {
        provider: "claude",
        state: "session-detected",
        session_detected: true,
        usage_available: false,
        auth_source: "claude-local-storage",
        detail,
        email: metadata.email,
        plan: metadata.plan,
    }
}

fn detect_windsurf_session() -> LocalSessionProbe {
    let app_paths = windsurf_paths();
    if app_paths.is_empty() {
        return LocalSessionProbe {
            provider: "windsurf",
            state: "missing",
            session_detected: false,
            usage_available: false,
            auth_source: "none",
            detail: "Windsurf or Codeium local app data was not found in the standard macOS paths.".to_string(),
            email: None,
            plan: None,
        };
    }

    let joined = app_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    LocalSessionProbe {
        provider: "windsurf",
        state: "install-detected",
        session_detected: false,
        usage_available: false,
        auth_source: "windsurf-local-paths",
        detail: format!(
            "Windsurf-related local paths were found at {joined}, but this build does not yet know which files hold a reusable logged-in session."
        ),
        email: None,
        plan: None,
    }
}

fn local_probe_json(probe: &LocalSessionProbe) -> String {
    let email_json = probe
        .email
        .as_ref()
        .map(|value| format!("\"{}\"", escape_json(value)))
        .unwrap_or_else(|| "null".to_string());
    let plan_json = probe
        .plan
        .as_ref()
        .map(|value| format!("\"{}\"", escape_json(value)))
        .unwrap_or_else(|| "null".to_string());

    format!(
        "{{\"provider\":\"{}\",\"state\":\"{}\",\"session_detected\":{},\"usage_available\":{},\"auth_source\":\"{}\",\"email\":{},\"plan\":{},\"detail\":\"{}\"}}",
        escape_json(probe.provider),
        escape_json(probe.state),
        probe.session_detected,
        probe.usage_available,
        escape_json(probe.auth_source),
        email_json,
        plan_json,
        escape_json(&probe.detail)
    )
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

fn cursor_state_db_path() -> Option<PathBuf> {
    home_dir().map(|home| {
        home.join("Library")
            .join("Application Support")
            .join("Cursor")
            .join("User")
            .join("globalStorage")
            .join("state.vscdb")
    }).filter(|path| path.exists())
}

fn claude_app_support_dir() -> Option<PathBuf> {
    home_dir()
        .map(|home| home.join("Library").join("Application Support").join("Claude"))
        .filter(|path| path.exists())
}

fn windsurf_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = home_dir() {
        let app_support = home.join("Library").join("Application Support");
        for name in ["Windsurf", "Codeium"] {
            let path = app_support.join(name);
            if path.exists() {
                paths.push(path);
            }
        }
    }
    for path in [
        PathBuf::from("/Applications/Windsurf.app"),
        PathBuf::from("/Applications/Codeium.app"),
    ] {
        if path.exists() {
            paths.push(path);
        }
    }
    paths
}

fn home_dir() -> Option<PathBuf> {
    env::var("HOME").ok().map(PathBuf::from)
}

fn sqlite_query_scalar(path: &Path, table: &str, key: &str) -> Option<String> {
    let query = format!(
        "select quote(value) from {table} where key='{}' limit 1;",
        key.replace('\'', "''")
    );
    let output = run_command("sqlite3", &[path.to_string_lossy().as_ref(), query.as_str()]).ok()?;
    parse_sqlite_quoted_scalar(&output)
}

fn parse_sqlite_quoted_scalar(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "NULL" {
        return None;
    }
    if let Some(stripped) = trimmed.strip_prefix('\'').and_then(|value| value.strip_suffix('\'')) {
        let unescaped = stripped.replace("''", "'");
        let cleaned = unescaped.trim().to_string();
        if cleaned.is_empty() {
            None
        } else {
            Some(cleaned)
        }
    } else {
        let cleaned = trimmed.to_string();
        if cleaned.is_empty() {
            None
        } else {
            Some(cleaned)
        }
    }
}

struct ClaudeLocalMetadata {
    email: Option<String>,
    plan: Option<String>,
    org_type: Option<String>,
}

fn read_claude_local_metadata(app_dir: &Path) -> ClaudeLocalMetadata {
    let leveldb = app_dir.join("Local Storage").join("leveldb");
    let mut email = None;
    let mut plan = None;
    let mut org_type = None;

    if let Ok(entries) = fs::read_dir(leveldb) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if let Ok(contents) = fs::read(&path) {
                for chunk in printable_chunks(&contents) {
                    if email.is_none() {
                        email = extract_json_string_field(&chunk, "email");
                    }
                    if plan.is_none() {
                        plan = extract_json_string_field(&chunk, "subscription_plan")
                            .or_else(|| extract_json_string_field(&chunk, "plan"));
                    }
                    if org_type.is_none() {
                        org_type = extract_json_string_field(&chunk, "org_type");
                    }
                    if email.is_some() && plan.is_some() && org_type.is_some() {
                        return ClaudeLocalMetadata {
                            email,
                            plan,
                            org_type,
                        };
                    }
                }
            }
        }
    }

    ClaudeLocalMetadata {
        email,
        plan,
        org_type,
    }
}

fn claude_cookie_marker_exists(app_dir: &Path) -> bool {
    let cookie_path = app_dir.join("Cookies");
    if !cookie_path.exists() {
        return false;
    }

    let query = "select count(*) from cookies where host_key like '%claude.ai%' and name in ('sessionKey','lastActiveOrg');";
    let output = match run_command("sqlite3", &[cookie_path.to_string_lossy().as_ref(), query]) {
        Ok(output) => output,
        Err(_) => return false,
    };

    output.trim().parse::<u64>().map(|count| count > 0).unwrap_or(false)
}

fn printable_chunks(bytes: &[u8]) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for &byte in bytes {
        let ch = byte as char;
        if ch.is_ascii_graphic() || ch == ' ' {
            current.push(ch);
        } else {
            if current.len() >= 24 {
                chunks.push(current.clone());
            }
            current.clear();
        }
    }

    if current.len() >= 24 {
        chunks.push(current);
    }

    chunks
}

fn extract_json_string_field(haystack: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\":\"");
    let start = haystack.find(&needle)? + needle.len();
    let rest = &haystack[start..];
    let end = rest.find('"')?;
    let value = &rest[..end];
    if value.trim().is_empty() || value == "null" {
        None
    } else {
        Some(value.to_string())
    }
}

fn provider_title(provider: &str) -> &'static str {
    match provider {
        "codex" | "openai" => "Codex",
        "cursor" => "Cursor",
        "claude" => "Claude",
        "windsurf" => "Windsurf",
        _ => "Provider",
    }
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
    fn renders_help_for_empty_args() {
        let output = run(&[]).unwrap();
        assert!(output.contains("model-meter 0.1.0"));
        assert!(output.contains("model-meter status"));
        assert!(output.contains("model-meter codex"));
        assert!(output.contains("model-meter cursor"));
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
    fn short_cursor_command_routes_to_usage() {
        let short = run(&["cursor".to_string()]).unwrap();
        let long = run(&["usage".to_string(), "cursor".to_string()]).unwrap();
        assert_eq!(short, long);
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

    #[test]
    fn parses_sqlite_quoted_scalars() {
        assert_eq!(
            parse_sqlite_quoted_scalar("'free'"),
            Some("free".to_string())
        );
        assert_eq!(parse_sqlite_quoted_scalar("NULL"), None);
        assert_eq!(parse_sqlite_quoted_scalar("''"), None);
    }

    #[test]
    fn extracts_json_string_field_from_chunk() {
        let chunk = r#"{"email":"a@example.com","subscription_plan":"claude_free","org_type":"claude_free"}"#;
        assert_eq!(
            extract_json_string_field(chunk, "email"),
            Some("a@example.com".to_string())
        );
        assert_eq!(
            extract_json_string_field(chunk, "subscription_plan"),
            Some("claude_free".to_string())
        );
    }
}
