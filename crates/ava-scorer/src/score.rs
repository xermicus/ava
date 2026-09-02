//! The `score` sub command: scoring submissions and aggregating proxy metrics.

/// Where the scorer expects the submission, relative to the working directory.
const SUBMISSION_DIRECTORY: &str = "submission";

const SUCCESS_STATUS: u32 = 200;

/// What the proxy logs for a request it finished serving.
const COMPLETED: &str = "OK";

/// The share of a request an upstream has to withhold its headers for before
/// the answer counts as buffered rather than streamed. A streamed answer sends
/// its headers within the first fraction of the request, while a buffered one
/// holds them until the body is ready, which puts the two three orders of
/// magnitude apart and leaves the exact share uncritical.
const BUFFERED_HEADER_SHARE: f64 = 0.95;

/// The submission scoring command.
#[derive(Debug, Default)]
pub struct Score {
    /// The proxy access log to aggregate into metrics.
    pub metrics: Option<String>,
    /// The game scoring the submission.
    pub game: Option<String>,
    /// The live scoring log to aggregate into attempts.
    pub attempts: Option<String>,
}

/// One request as the proxy sidecar logged it.
#[derive(serde::Deserialize)]
struct Record {
    host: String,
    status: u32,
    /// What the proxy logged for the completion of the request. Empty means the
    /// client went away before the answer was fully written, which is what the
    /// restart at the end of a phase does to whatever request is in flight. A
    /// log written before the proxy recorded it holds no answer either way, so
    /// it reads as completed and reports nothing abandoned.
    #[serde(default = "completed")]
    completed: String,
    request_bytes: u64,
    response_bytes: u64,
    request_seconds: f64,
    /// When the upstream answered with its headers, formatted by the proxy and
    /// empty for a request that never reached an upstream.
    #[serde(default)]
    header_seconds: String,
    /// How long the upstream took for the whole answer, in the same shape.
    #[serde(default)]
    upstream_seconds: String,
    first_token_seconds: f64,
    served_models: String,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
    /// Absent from logs written before the proxy counted deltas.
    #[serde(default)]
    streamed_deltas: u64,
    /// The account limits the backend reported, absent from older logs.
    #[serde(default)]
    ratelimits: String,
    /// The cost the gateway reported for the answer, empty off the gateway.
    #[serde(default)]
    gateway_cost: String,
}

/// The aggregate over every request in one proxy access log.
#[derive(Default, serde::Serialize)]
struct Metrics {
    requests: u64,
    /// The requests answered with a non-200 status.
    failed_requests: u64,
    /// The requests a model answered in full without ever reporting its usage,
    /// so the stream was cut short upstream.
    truncated_requests: u64,
    /// The requests the client abandoned before the answer was written. The
    /// restart at the end of a phase leaves one of these behind whenever the
    /// agent had a request in flight, so these are expected rather than a
    /// fault of the upstream.
    aborted_requests: u64,
    /// The answers an upstream withheld until it had generated all of them, so
    /// nothing reached the harness while the model worked. Time to the first
    /// token is the whole generation time for these, not a latency.
    buffered_requests: u64,
    /// Every distinct host that was requested.
    hosts: Vec<String>,
    /// Every distinct model identifier seen in a response body.
    served_models: Vec<String>,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
    /// Content delta events counted as the streams passed, the approximate
    /// volume of the requests whose usage report never arrived.
    streamed_deltas: u64,
    /// The account limits of the newest answer that reported them.
    #[serde(skip_serializing_if = "String::is_empty")]
    ratelimits: String,
    /// The cost the gateway reported, summed over the answers that carried one.
    gateway_cost: f64,
    request_bytes: u64,
    response_bytes: u64,
    request_seconds: f64,
    /// The mean time to the first generated token, over the requests reporting one.
    mean_first_token_seconds: f64,
}

/// One attempt the scoring server recorded.
#[derive(serde::Deserialize)]
struct Attempt {
    seconds: u64,
    solved: bool,
    points: u64,
}

/// The attempts of one run; the best solving one is the submission of record.
#[derive(serde::Serialize)]
struct Attempts {
    attempts: u64,
    solved: bool,
    points: u64,
    /// Seconds to the best solving attempt, breaking point ties.
    #[serde(skip_serializing_if = "Option::is_none")]
    first_solved_seconds: Option<u64>,
}

/// The parameters one run was played with, repeated in its report.
#[derive(serde::Serialize)]
pub struct Run<'a> {
    pub harness: &'a str,
    pub harness_version: &'a str,
    pub model: &'a str,
    pub game: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<&'a str>,
    pub limit_seconds: u64,
}

/// The document `score` prints, holding whatever was requested.
#[derive(serde::Serialize)]
struct Report<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    run: Option<Run<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<ava_game::Score>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metrics: Option<Metrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attempts: Option<Attempts>,
}

/// Run the score sub command and print the requested reports as one JSON document.
pub fn run(command: &Score) -> std::io::Result<i32> {
    println!("{}", report(command, None)?);

    Ok(0)
}

/// The document holding whatever reports `command` requested, opened by the
/// `run` parameters when the caller knows them.
pub fn report(command: &Score, run: Option<Run>) -> std::io::Result<String> {
    let score = match &command.game {
        Some(name) => {
            let game = ava_game::find(name).ok_or_else(|| {
                let known: Vec<&str> = ava_game::GAMES.iter().map(|game| game.name()).collect();
                std::io::Error::other(format!(
                    "unknown game `{name}`, known are: {}",
                    known.join(", ")
                ))
            })?;
            let score = game.score(std::path::Path::new(SUBMISSION_DIRECTORY))?;
            log::info!(
                "the {name} submission is {}, at {} points",
                if score.solved { "solved" } else { "unsolved" },
                score.points
            );
            Some(score)
        }
        None => None,
    };

    let metrics = match &command.metrics {
        Some(log) => Some(aggregate(log)?),
        None => None,
    };

    let attempts = match &command.attempts {
        Some(log) => Some(aggregate_attempts(log)?),
        None => None,
    };

    let report = Report {
        run,
        score,
        metrics,
        attempts,
    };

    serde_json::to_string_pretty(&report).map_err(std::io::Error::other)
}

/// Aggregate the access log at `log` into one [`Metrics`].
fn aggregate(log: &str) -> std::io::Result<Metrics> {
    let contents = std::fs::read_to_string(log)
        .map_err(|error| std::io::Error::other(format!("{log}: {error}")))?;

    let mut metrics = Metrics::default();
    let mut first_token_seconds = 0f64;
    let mut streamed_requests = 0u64;

    for line in contents.lines().filter(|line| line.starts_with('{')) {
        let record: Record = serde_json::from_str(line)
            .map_err(|error| std::io::Error::other(format!("{log}: {error}")))?;

        metrics.requests += 1;
        if record.status != SUCCESS_STATUS {
            metrics.failed_requests += 1;
        }

        record_distinct(&mut metrics.hosts, &record.host);
        for model in record.served_models.split_whitespace() {
            record_distinct(&mut metrics.served_models, model);
        }

        let answered = !record.served_models.is_empty();
        let aborted = record.completed != COMPLETED;

        if aborted {
            metrics.aborted_requests += 1;
        } else if answered && record.output_tokens == 0 {
            metrics.truncated_requests += 1;
        }

        if answered && buffered(&record) {
            metrics.buffered_requests += 1;
        }

        metrics.input_tokens += record.input_tokens;
        metrics.output_tokens += record.output_tokens;
        metrics.cache_read_tokens += record.cache_read_tokens;
        metrics.cache_write_tokens += record.cache_write_tokens;
        metrics.streamed_deltas += record.streamed_deltas;
        if !record.ratelimits.is_empty() {
            metrics.ratelimits = record.ratelimits;
        }
        if let Ok(cost) = record.gateway_cost.parse::<f64>() {
            metrics.gateway_cost += cost;
        }
        metrics.request_bytes += record.request_bytes;
        metrics.response_bytes += record.response_bytes;
        metrics.request_seconds += record.request_seconds;

        if record.first_token_seconds > 0.0 {
            first_token_seconds += record.first_token_seconds;
            streamed_requests += 1;
        }
    }

    if streamed_requests > 0 {
        metrics.mean_first_token_seconds = first_token_seconds / streamed_requests as f64;
    }

    log::info!(
        "{log}: {} requests, {} failed, {} truncated, {} aborted, {} buffered, models: {}",
        metrics.requests,
        metrics.failed_requests,
        metrics.truncated_requests,
        metrics.aborted_requests,
        metrics.buffered_requests,
        metrics.served_models.join(" ")
    );

    Ok(metrics)
}

/// What a request the proxy did not record a completion for reads as.
fn completed() -> String {
    COMPLETED.to_string()
}

/// Whether the upstream held `record` back until it had generated the whole
/// answer, so the harness saw nothing while the model worked.
fn buffered(record: &Record) -> bool {
    let header = seconds(&record.header_seconds);
    let upstream = seconds(&record.upstream_seconds);

    upstream > 0.0 && header >= upstream * BUFFERED_HEADER_SHARE
}

/// One elapsed time as the proxy formats it, zero when there is none. A
/// request answered from more than one upstream carries one time per try, and
/// the last of them is the try that answered.
fn seconds(field: &str) -> f64 {
    field
        .rsplit(',')
        .next()
        .unwrap_or_default()
        .trim()
        .parse()
        .unwrap_or_default()
}

/// Aggregate the scoring log at `log` into one [`Attempts`].
fn aggregate_attempts(log: &str) -> std::io::Result<Attempts> {
    let contents = std::fs::read_to_string(log)
        .map_err(|error| std::io::Error::other(format!("{log}: {error}")))?;

    let mut report = Attempts {
        attempts: 0,
        solved: false,
        points: 0,
        first_solved_seconds: None,
    };

    for line in contents.lines().filter(|line| line.starts_with('{')) {
        let attempt: Attempt = serde_json::from_str(line)
            .map_err(|error| std::io::Error::other(format!("{log}: {error}")))?;

        report.attempts += 1;
        if !attempt.solved || attempt.points == 0 {
            continue;
        }

        if !report.solved || attempt.points > report.points {
            report.solved = true;
            report.points = attempt.points;
            report.first_solved_seconds = Some(attempt.seconds);
        }
    }

    log::info!(
        "{log}: {} attempts, the best one {} at {} points",
        report.attempts,
        if report.solved { "solved" } else { "unsolved" },
        report.points
    );

    Ok(report)
}

fn record_distinct(seen: &mut Vec<String>, value: &str) {
    if !seen.iter().any(|entry| entry == value) {
        seen.push(value.to_string());
    }
}
