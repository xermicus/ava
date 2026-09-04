//! The usage of the backends.

use crate::registry::{Backend, Registry, Service};

const QUERY_TIMEOUT_SECONDS: &str = "20";
const SUCCESS_STATUS: &str = "200";
const ERROR_BODY_BYTES: usize = 200;

/// Only an answered message carries the limits, so the probe spends one output token.
const ANTHROPIC_MESSAGES_PATH: &str = "/v1/messages";
const ANTHROPIC_HEADERS: [&str; 3] = [
    "anthropic-version: 2023-06-01",
    "anthropic-beta: oauth-2025-04-20",
    "content-type: application/json",
];
const MODEL_PLACEHOLDER: &str = "__MODEL__";
const PROBE: &str =
    r#"{"model":__MODEL__,"max_tokens":1,"messages":[{"role":"user","content":"hi"}]}"#;

const GATEWAY_KEY_INFO_PATH: &str = "/key/info";

/// The header prefixes the proxy captures.
const LIMIT_HEADER_PREFIXES: [&str; 3] = ["anthropic-ratelimit-", "x-ratelimit-", "x-litellm-key-"];
const ANTHROPIC_LIMIT_PREFIX: &str = "anthropic-ratelimit-unified-";
const ANTHROPIC_WINDOWS: [(&str, &str); 2] = [("5h", "session, 5 hours"), ("7d", "week, 7 days")];
const GATEWAY_SPEND_HEADER: &str = "x-litellm-key-spend";
const GATEWAY_BUDGET_HEADER: &str = "x-litellm-key-max-budget";
const GATEWAY_RESET_HEADER: &str = "x-litellm-key-budget-reset-at";
const GATEWAY_DURATION_HEADER: &str = "x-litellm-key-budget-duration";
const GATEWAY_INFO_FIELDS: [(&str, &str); 4] = [
    ("spend", GATEWAY_SPEND_HEADER),
    ("max_budget", GATEWAY_BUDGET_HEADER),
    ("budget_reset_at", GATEWAY_RESET_HEADER),
    ("budget_duration", GATEWAY_DURATION_HEADER),
];
const GATEWAY_LIMIT_PREFIX: &str = "x-ratelimit-";
const GATEWAY_WINDOWS: [&str; 2] = ["requests", "tokens"];

const OVERAGE_LABEL: &str = "overage";
const BUDGET_LABEL: &str = "budget";
const UNBUDGETED_LABEL: &str = "unbudgeted";
const LIVE_SOURCE: &str = "live";
const UNAVAILABLE_SOURCE: &str = "unavailable";
const NOTHING_REPORTED: &str = "nothing reported";

const STATE_HEADERS: [&str; 7] = [
    "BACKEND", "SOURCE", "WINDOW", "USED", "LEFT", "STATUS", "RESETS",
];
const RECORDED_HEADERS: [&str; 8] = [
    "BACKEND",
    "RUNS",
    "REQUESTS",
    "INPUT",
    "OUTPUT",
    "CACHE READ",
    "CACHE WRITE",
    "COST",
];

const SECONDS_PER_DAY: u64 = 86_400;
const SECONDS_PER_HOUR: u64 = 3_600;
const SECONDS_PER_MINUTE: u64 = 60;

/// The usage of a backend over the runs on disk.
#[derive(Default)]
pub struct Recorded {
    pub runs: u64,
    pub requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub gateway_cost: f64,
    /// The limits the newest run captured, with the second it started.
    pub newest_limits: Option<(u64, String)>,
}

impl Recorded {
    fn add(&mut self, metrics: &Metrics, started: u64) {
        self.runs += 1;
        self.requests += metrics.requests;
        self.input_tokens += metrics.input_tokens;
        self.output_tokens += metrics.output_tokens;
        self.cache_read_tokens += metrics.cache_read_tokens;
        self.cache_write_tokens += metrics.cache_write_tokens;
        self.gateway_cost += metrics.gateway_cost;

        let older = self
            .newest_limits
            .as_ref()
            .is_some_and(|(newest, _)| started <= *newest);
        if !older && !metrics.ratelimits.is_empty() {
            self.newest_limits = Some((started, metrics.ratelimits.clone()));
        }
    }
}

#[derive(serde::Deserialize)]
struct Metadata {
    started_seconds: u64,
}

#[derive(serde::Deserialize)]
struct Report {
    metrics: Option<Metrics>,
}

#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct Metrics {
    hosts: Vec<String>,
    requests: u64,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
    ratelimits: String,
    gateway_cost: f64,
}

/// The recorded usage of every backend, in registry order. A run counts
/// towards every backend whose host it requested.
pub fn recorded(registry: &Registry) -> std::io::Result<Vec<Recorded>> {
    let mut recorded: Vec<Recorded> = registry
        .backends
        .iter()
        .map(|_| Recorded::default())
        .collect();

    for directory in crate::docker::run_directories()? {
        let Some(metadata) = read_json::<Metadata>(&directory.join(crate::docker::METADATA_FILE))
        else {
            continue;
        };
        let Some(Report {
            metrics: Some(metrics),
        }) = read_json(&directory.join(crate::docker::SCORE_FILE))
        else {
            continue;
        };
        for (backend, usage) in registry.backends.iter().zip(recorded.iter_mut()) {
            if metrics.hosts.contains(&backend.host) {
                usage.add(&metrics, metadata.started_seconds);
            }
        }
    }

    Ok(recorded)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Option<T> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Ask `backend` for its limits, as the `name=value` pairs the proxy captures.
pub fn limits(backend: &Backend, registry: &Registry) -> std::io::Result<String> {
    let credential = backend.credential()?;

    match backend.service {
        Service::Anthropic => anthropic_limits(backend, registry, &credential),
        Service::OpenApi => gateway_limits(backend, &credential),
    }
}

fn anthropic_limits(
    backend: &Backend,
    registry: &Registry,
    credential: &str,
) -> std::io::Result<String> {
    let url = format!("https://{}{ANTHROPIC_MESSAGES_PATH}", backend.host);
    let mut answered = Err(std::io::Error::other(format!(
        "no model is routed to the {} backend",
        backend.name
    )));
    for model in registry.models.iter().flat_map(|model| &model.routes) {
        if model.backend != backend.name {
            continue;
        }
        let body = PROBE.replace(MODEL_PLACEHOLDER, &serde_json::to_string(&model.id)?);
        answered = request(&url, credential, &ANTHROPIC_HEADERS, Some(&body));
        if answered.is_ok() {
            break;
        }
    }
    let (headers, _) = answered?;

    let mut pairs: Vec<String> = headers
        .into_iter()
        .filter(|(name, _)| {
            LIMIT_HEADER_PREFIXES
                .iter()
                .any(|prefix| name.starts_with(prefix))
        })
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    pairs.sort();

    Ok(pairs.join(" "))
}

fn gateway_limits(backend: &Backend, credential: &str) -> std::io::Result<String> {
    let url = format!("https://{}{GATEWAY_KEY_INFO_PATH}", backend.host);
    let (_, body) = request(&url, credential, &[], None)?;
    let info: serde_json::Value = serde_json::from_str(&body).map_err(|error| {
        std::io::Error::other(format!("{url}: the key info does not parse: {error}"))
    })?;

    let pairs: Vec<String> = GATEWAY_INFO_FIELDS
        .iter()
        .filter_map(|(field, header)| {
            let value = info
                .pointer(&format!("/info/{field}"))
                .filter(|value| !value.is_null())?;
            let text = value
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| value.to_string());
            Some(format!("{header}={text}"))
        })
        .collect();

    Ok(pairs.join(" "))
}

type Headers = Vec<(String, String)>;

/// Send a request with `credential` as bearer token. The answer is its lowercase
/// headers and its body.
fn request(
    url: &str,
    credential: &str,
    headers: &[&str],
    body: Option<&str>,
) -> std::io::Result<(Headers, String)> {
    let mut command = std::process::Command::new("curl");
    command
        .args([
            "--silent",
            "--show-error",
            "--include",
            "--max-time",
            QUERY_TIMEOUT_SECONDS,
        ])
        .args(["--config", "-", url])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    for header in headers {
        command.args(["--header", header]);
    }
    if let Some(body) = body {
        command.args(["--data", body]);
    }

    // The credential goes in as configuration, which keeps it out of the process list.
    let mut child = command.spawn()?;
    let configuration = format!("header = \"Authorization: Bearer {credential}\"\n");
    std::io::Write::write_all(
        &mut child.stdin.take().expect("the stdin of curl is piped"),
        configuration.as_bytes(),
    )?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        let reason = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(std::io::Error::other(format!("{url}: {reason}")));
    }

    answer(url, &String::from_utf8_lossy(&output.stdout))
}

fn answer(url: &str, printed: &str) -> std::io::Result<(Headers, String)> {
    let (head, body) = printed.split_once("\r\n\r\n").unwrap_or((printed, ""));
    let mut lines = head.lines();
    let status = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or_default();
    if status != SUCCESS_STATUS {
        let excerpt: String = body.trim().chars().take(ERROR_BODY_BYTES).collect();
        return Err(std::io::Error::other(format!(
            "{url} answered {status}: {excerpt}"
        )));
    }

    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_lowercase(), value.trim().to_string()))
        .collect();

    Ok((headers, body.to_string()))
}

/// A line of the limits of a backend.
#[derive(Default)]
pub struct Line {
    pub window: String,
    /// The used amount and its ceiling, for a meter.
    pub used: Option<(u64, u64)>,
    pub used_label: String,
    pub left: String,
    pub status: String,
    pub resets: String,
    /// The seconds left until the reset and the length of the window, for a meter.
    pub wait: Option<(u64, u64)>,
}

type Pairs<'a> = std::collections::BTreeMap<&'a str, &'a str>;

fn value<'a>(pairs: &Pairs<'a>, name: &str) -> &'a str {
    pairs.get(name).copied().unwrap_or_default()
}

/// The lines of the `name=value` pairs in `limits`.
pub fn lines(limits: &str) -> Vec<Line> {
    let pairs: Pairs = limits
        .split_whitespace()
        .filter_map(|pair| pair.split_once('='))
        .collect();

    let mut lines = window_lines(&pairs);
    lines.extend(overage_line(&pairs));
    lines.extend(budget_line(&pairs));
    lines.extend(quota_lines(&pairs));
    lines
}

fn window_lines(pairs: &Pairs) -> Vec<Line> {
    let mut lines = Vec::new();

    for (window, label) in ANTHROPIC_WINDOWS {
        let prefix = format!("{ANTHROPIC_LIMIT_PREFIX}{window}-");
        let Ok(utilization) = value(pairs, &format!("{prefix}utilization")).parse::<f64>() else {
            continue;
        };
        let used = (utilization * 100.0).round() as u64;
        let reset = value(pairs, &format!("{prefix}reset")).parse::<u64>().ok();
        lines.push(Line {
            window: label.to_string(),
            used: Some((used, 100)),
            used_label: format!("{used}%"),
            left: format!("{}%", 100u64.saturating_sub(used)),
            status: value(pairs, &format!("{prefix}status")).to_string(),
            resets: reset.map(utc_date).unwrap_or_default(),
            wait: reset.map(|reset| (reset.saturating_sub(epoch_now()), window_seconds(window))),
        });
    }

    lines
}

/// The length of an Anthropic window such as `5h` or `7d`, in seconds.
fn window_seconds(window: &str) -> u64 {
    let (count, unit) = window.split_at(window.len() - 1);
    let count: u64 = count.parse().unwrap_or(0);
    match unit {
        "d" => count * SECONDS_PER_DAY,
        _ => count * SECONDS_PER_HOUR,
    }
}

fn overage_line(pairs: &Pairs) -> Option<Line> {
    let status = value(pairs, &format!("{ANTHROPIC_LIMIT_PREFIX}overage-status"));
    if status.is_empty() {
        return None;
    }
    let reason = value(
        pairs,
        &format!("{ANTHROPIC_LIMIT_PREFIX}overage-disabled-reason"),
    );

    Some(Line {
        window: OVERAGE_LABEL.to_string(),
        status: format!("{status} {reason}").trim_end().to_string(),
        ..Line::default()
    })
}

fn budget_line(pairs: &Pairs) -> Option<Line> {
    let spend: f64 = value(pairs, GATEWAY_SPEND_HEADER).parse().ok()?;
    let budget: Option<f64> = value(pairs, GATEWAY_BUDGET_HEADER).parse().ok();
    let left = match budget {
        Some(budget) => format!("{} of {}", money((budget - spend).max(0.0)), money(budget)),
        None => UNBUDGETED_LABEL.to_string(),
    };

    let reset = epoch_of(value(pairs, GATEWAY_RESET_HEADER));
    let duration = value(pairs, GATEWAY_DURATION_HEADER);

    Some(Line {
        window: BUDGET_LABEL.to_string(),
        used: budget.map(|budget| ((spend * 100.0) as u64, (budget * 100.0) as u64)),
        used_label: money(spend),
        left,
        resets: reset.map(utc_date).unwrap_or_default(),
        wait: reset
            .filter(|_| !duration.is_empty())
            .map(|reset| (reset.saturating_sub(epoch_now()), window_seconds(duration))),
        ..Line::default()
    })
}

fn quota_lines(pairs: &Pairs) -> Vec<Line> {
    let mut lines = Vec::new();

    for window in GATEWAY_WINDOWS {
        let limit = value(pairs, &format!("{GATEWAY_LIMIT_PREFIX}limit-{window}")).parse::<u64>();
        let remaining =
            value(pairs, &format!("{GATEWAY_LIMIT_PREFIX}remaining-{window}")).parse::<u64>();
        let (Ok(limit), Ok(remaining)) = (limit, remaining) else {
            continue;
        };
        let used = limit.saturating_sub(remaining);
        lines.push(Line {
            window: window.to_string(),
            used: Some((used, limit)),
            used_label: used.to_string(),
            left: format!("{remaining} of {limit}"),
            resets: value(pairs, &format!("{GATEWAY_LIMIT_PREFIX}reset-{window}")).to_string(),
            ..Line::default()
        });
    }

    lines
}

/// The usage of a backend.
pub struct Usage {
    pub recorded: Recorded,
    /// The live limits, or the newest recorded ones when the backend did not answer.
    pub limits: String,
    /// Where the limits came from.
    pub source: String,
    /// Why the backend did not answer.
    pub failure: Option<String>,
}

impl Usage {
    fn new(recorded: Recorded, live: std::io::Result<String>) -> Self {
        let (limits, source, failure) = match live {
            Ok(limits) => (limits, LIVE_SOURCE.to_string(), None),
            Err(error) => match &recorded.newest_limits {
                Some((started, limits)) => (
                    limits.clone(),
                    format!("recorded {} ago", age(*started)),
                    Some(error.to_string()),
                ),
                None => (
                    String::new(),
                    UNAVAILABLE_SOURCE.to_string(),
                    Some(error.to_string()),
                ),
            },
        };

        Self {
            recorded,
            limits,
            source,
            failure,
        }
    }
}

/// The usage of every backend, in registry order. The backends are asked in parallel.
pub fn report(registry: &Registry) -> std::io::Result<Vec<Usage>> {
    let recorded = recorded(registry)?;
    let live: Vec<std::io::Result<String>> = std::thread::scope(|scope| {
        let queries: Vec<_> = registry
            .backends
            .iter()
            .map(|backend| scope.spawn(move || limits(backend, registry)))
            .collect();
        queries
            .into_iter()
            .map(|query| query.join().expect("a query does not panic"))
            .collect()
    });

    Ok(recorded
        .into_iter()
        .zip(live)
        .map(|(recorded, live)| Usage::new(recorded, live))
        .collect())
}

/// Print the usage of every backend, then exit.
pub fn print() -> ! {
    let registry = crate::registry::load_or_exit();
    let report = report(&registry).unwrap_or_else(|error| {
        eprintln!("error: {error}");
        std::process::exit(1);
    });
    let named = || {
        registry
            .backends
            .iter()
            .map(|backend| backend.name.as_str())
            .zip(&report)
    };

    print_table(
        &STATE_HEADERS,
        &named()
            .flat_map(|(name, usage)| state_rows(name, usage))
            .collect::<Vec<_>>(),
    );
    for (name, failure) in named().filter_map(|(name, usage)| Some((name, usage.failure.as_ref()?)))
    {
        println!("{name}: {failure}");
    }
    println!();
    print_table(
        &RECORDED_HEADERS,
        &named()
            .map(|(name, usage)| recorded_row(name, &usage.recorded))
            .collect::<Vec<_>>(),
    );

    std::process::exit(0);
}

fn state_rows(name: &str, usage: &Usage) -> Vec<Vec<String>> {
    let mut lines = lines(&usage.limits);
    if lines.is_empty() {
        lines.push(Line {
            window: NOTHING_REPORTED.to_string(),
            ..Line::default()
        });
    }

    lines
        .into_iter()
        .map(|line| {
            vec![
                name.to_string(),
                usage.source.clone(),
                line.window,
                line.used_label,
                line.left,
                line.status,
                line.resets,
            ]
        })
        .collect()
}

fn recorded_row(name: &str, recorded: &Recorded) -> Vec<String> {
    vec![
        name.to_string(),
        recorded.runs.to_string(),
        recorded.requests.to_string(),
        recorded.input_tokens.to_string(),
        recorded.output_tokens.to_string(),
        recorded.cache_read_tokens.to_string(),
        recorded.cache_write_tokens.to_string(),
        money(recorded.gateway_cost),
    ]
}

fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    let widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(column, header)| {
            rows.iter()
                .map(|row| row[column].len())
                .fold(header.len(), usize::max)
        })
        .collect();
    let print_row = |cells: Vec<&str>| {
        let padded: Vec<String> = cells
            .iter()
            .zip(&widths)
            .map(|(cell, width)| format!("{cell:<width$}"))
            .collect();
        println!("{}", padded.join("  ").trim_end());
    };

    print_row(headers.to_vec());
    for row in rows {
        print_row(row.iter().map(String::as_str).collect());
    }
}

/// How long ago the epoch second `started` was.
pub fn age(started: u64) -> String {
    let elapsed = epoch_now().saturating_sub(started);

    match elapsed {
        seconds if seconds < SECONDS_PER_MINUTE => format!("{seconds}s"),
        seconds if seconds < SECONDS_PER_HOUR => format!("{}m", seconds / SECONDS_PER_MINUTE),
        seconds if seconds < SECONDS_PER_DAY => format!(
            "{}h {}m",
            seconds / SECONDS_PER_HOUR,
            seconds % SECONDS_PER_HOUR / SECONDS_PER_MINUTE
        ),
        seconds => format!(
            "{}d {}h",
            seconds / SECONDS_PER_DAY,
            seconds % SECONDS_PER_DAY / SECONDS_PER_HOUR
        ),
    }
}

/// The current epoch second.
pub fn epoch_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|now| now.as_secs())
        .unwrap_or(0)
}

/// The epoch second `epoch` as a UTC date and time, to the minute.
/// The epoch of an ISO 8601 UTC date such as `2026-10-01T00:00:00+00:00`.
fn epoch_of(date: &str) -> Option<u64> {
    let mut fields = date
        .split(['-', 'T', ':', '+', 'Z'])
        .filter(|field| !field.is_empty())
        .map(|field| field.parse::<i64>().ok());
    let (year, month, day, hour, minute) = (
        fields.next()??,
        fields.next()??,
        fields.next()??,
        fields.next()??,
        fields.next()??,
    );

    // Days since the epoch from the civil date, after Howard Hinnant.
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let shifted_month = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;

    u64::try_from(days * SECONDS_PER_DAY as i64 + hour * SECONDS_PER_HOUR as i64 + minute * 60).ok()
}

pub fn utc_date(epoch: u64) -> String {
    let days = (epoch / SECONDS_PER_DAY) as i64;
    let seconds = epoch % SECONDS_PER_DAY;

    // Civil date from days since the epoch, after Howard Hinnant.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);

    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        seconds / SECONDS_PER_HOUR,
        seconds % SECONDS_PER_HOUR / SECONDS_PER_MINUTE
    )
}

/// A dollar amount with cents.
pub fn money(amount: f64) -> String {
    format!("${amount:.2}")
}
