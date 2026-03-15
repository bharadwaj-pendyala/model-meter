use std::env;
use std::fmt::Write as _;
use std::process;

const PROVIDERS: [ProviderSpec; 4] = [
    ProviderSpec {
        id: "openai",
        display_name: "Codex / OpenAI",
        support_tier: "official-api",
        configured_by: "OPENAI_ADMIN_KEY",
        quick_check_hint: "API-backed usage and billing surfaces",
        note: "ChatGPT-plan Codex usage is still unsupported without a documented API.",
    },
    ProviderSpec {
        id: "claude",
        display_name: "Claude",
        support_tier: "manual",
        configured_by: "MODEL_METER_CLAUDE_USED and MODEL_METER_CLAUDE_LIMIT",
        quick_check_hint: "Manual plan counters",
        note: "Use manual counters until Anthropic exposes a stable supported usage surface.",
    },
    ProviderSpec {
        id: "cursor",
        display_name: "Cursor",
        support_tier: "manual",
        configured_by: "MODEL_METER_CURSOR_USED and MODEL_METER_CURSOR_LIMIT",
        quick_check_hint: "Manual plan counters",
        note: "Use manual counters until a trustworthy machine-readable surface exists.",
    },
    ProviderSpec {
        id: "windsurf",
        display_name: "Windsurf",
        support_tier: "manual",
        configured_by: "MODEL_METER_WINDSURF_USED and MODEL_METER_WINDSURF_LIMIT",
        quick_check_hint: "Manual plan counters",
        note: "Use manual counters until a trustworthy machine-readable surface exists.",
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
enum ProviderState {
    OfficialApi {
        configured: bool,
        auth_state: &'static str,
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
        ["providers"] => Ok(render_providers(json)),
        ["auth", "validate"] => render_auth_validation("openai", json),
        ["auth", "validate", provider] => render_auth_validation(provider, json),
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
    text.push_str("  model-meter providers [--json]\n");
    text.push_str("  model-meter auth validate [openai] [--json]\n");
    text.push_str("  model-meter status [--json]\n\n");
    text.push_str("Manual counters:\n");
    text.push_str("  MODEL_METER_CLAUDE_USED / MODEL_METER_CLAUDE_LIMIT\n");
    text.push_str("  MODEL_METER_CURSOR_USED / MODEL_METER_CURSOR_LIMIT\n");
    text.push_str("  MODEL_METER_WINDSURF_USED / MODEL_METER_WINDSURF_LIMIT\n");
    text.push_str("  MODEL_METER_OPENAI_USED / MODEL_METER_OPENAI_LIMIT\n\n");
    text.push_str("OpenAI auth:\n");
    text.push_str("  OPENAI_ADMIN_KEY\n");
    text
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
    if provider != "openai" {
        return Err(format!(
            "auth validation is currently implemented only for openai; got {provider}"
        ));
    }

    let key = env::var("OPENAI_ADMIN_KEY").ok();
    let (valid, state, detail) = match key {
        Some(value) if !value.trim().is_empty() => (
            true,
            "configured",
            "OPENAI_ADMIN_KEY is present. Network validation is not implemented yet in this sample."
                .to_string(),
        ),
        _ => (
            false,
            "missing",
            "OPENAI_ADMIN_KEY is not set. This sample can only validate presence locally.".to_string(),
        ),
    };

    if json {
        return Ok(format!(
            "{{\"provider\":\"openai\",\"valid\":{},\"state\":\"{}\",\"detail\":\"{}\"}}",
            valid,
            escape_json(state),
            escape_json(&detail)
        ));
    }

    if valid {
        Ok(format!("openai auth: configured\n{detail}"))
    } else {
        Err(format!("openai auth: missing\n{detail}"))
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
                detail,
            } => {
                let status = if configured {
                    "configured"
                } else {
                    "not configured"
                };
                let _ = writeln!(
                    text,
                    "- {}: {} ({})\n  {}",
                    provider.display_name, status, auth_state, detail
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

    text.trim_end().to_string()
}

fn provider_state(provider: ProviderSpec) -> ProviderState {
    match provider.id {
        "openai" => {
            let key = env::var("OPENAI_ADMIN_KEY").ok();
            let manual_counter = read_manual_counter("OPENAI");
            let detail = match (key.as_ref(), manual_counter.as_ref()) {
                (Some(_), Ok(Some(counter))) => format!(
                    "API auth is configured. Manual quick-check counter: {:.1}% used ({:.2} / {:.2}).",
                    counter.percentage, counter.used, counter.limit
                ),
                (Some(_), Ok(None)) => {
                    "API auth is configured. Add MODEL_METER_OPENAI_USED and MODEL_METER_OPENAI_LIMIT for a local quick-check sample.".to_string()
                }
                (None, Ok(Some(counter))) => format!(
                    "No API auth yet. Manual quick-check counter: {:.1}% used ({:.2} / {:.2}).",
                    counter.percentage, counter.used, counter.limit
                ),
                (None, Ok(None)) => {
                    "Set OPENAI_ADMIN_KEY for API-backed work, or set MODEL_METER_OPENAI_USED and MODEL_METER_OPENAI_LIMIT for a manual quick check.".to_string()
                }
                (_, Err(err)) => err.clone(),
            };

            let configured = key.as_ref().is_some();
            let auth_state = if configured { "configured" } else { "missing" };
            ProviderState::OfficialApi {
                configured,
                auth_state,
                detail,
            }
        }
        "claude" => manual_provider_state("CLAUDE", provider.note),
        "cursor" => manual_provider_state("CURSOR", provider.note),
        "windsurf" => manual_provider_state("WINDSURF", provider.note),
        _ => ProviderState::Manual {
            counter: None,
            state: "unsupported",
            detail: "No provider state is available.".to_string(),
        },
    }
}

fn manual_provider_state(prefix: &str, note: &str) -> ProviderState {
    match read_manual_counter(prefix) {
        Ok(Some(counter)) => ProviderState::Manual {
            detail: format!("Manual plan counter is configured. {note}"),
            counter: Some(counter),
            state: "manual",
        },
        Ok(None) => ProviderState::Manual {
            counter: None,
            state: "manual",
            detail: format!(
                "Set MODEL_METER_{prefix}_USED and MODEL_METER_{prefix}_LIMIT. {note}"
            ),
        },
        Err(err) => ProviderState::Manual {
            counter: None,
            state: "invalid-config",
            detail: err,
        },
    }
}

fn provider_state_json(provider: &ProviderSpec, state: &ProviderState) -> String {
    match state {
        ProviderState::OfficialApi {
            configured,
            auth_state,
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
                "{{\"id\":\"{}\",\"display_name\":\"{}\",\"support_tier\":\"{}\",\"configured\":{},\"auth_state\":\"{}\",\"detail\":\"{}\"{}}}",
                escape_json(provider.id),
                escape_json(provider.display_name),
                escape_json(provider.support_tier),
                configured,
                escape_json(auth_state),
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
    }
}
