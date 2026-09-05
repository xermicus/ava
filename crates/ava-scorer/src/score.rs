//! The `score` sub command: verifying a submission, fighting two entries and
//! aggregating the logs of a run.

/// Where the verifier expects the submission, relative to the working directory.
pub const SUBMISSION_DIRECTORY: &str = "submission";

/// The directories under a fight directory holding the entry of each seat.
pub const FIRST_DIRECTORY: &str = "first";
pub const SECOND_DIRECTORY: &str = "second";

const SUCCESS_STATUS: u32 = 200;

/// The combats a fight plays unless the command asks for more.
const DEFAULT_COMBATS: u64 = 1;

/// What the proxy logs for a request it finished serving.
const COMPLETED: &str = "OK";

/// The share of a request an upstream has to withhold its headers for before
/// the answer counts as buffered rather than streamed. A streamed answer sends
/// its headers within the first fraction of the request, while a buffered one
/// holds them until the body is ready, which puts the two three orders of
/// magnitude apart and leaves the exact share uncritical.
const BUFFERED_HEADER_SHARE: f64 = 0.95;

/// The scoring command.
#[derive(Debug, Default)]
pub struct Score {
    /// The proxy access log to aggregate into metrics.
    pub metrics: Option<String>,
    /// The game verifying the submission or fighting the entries.
    pub game: Option<String>,
    /// The attempts log to read.
    pub attempts: Option<String>,
    /// The directory holding the two entries to fight, under `first` and `second`.
    pub fight: Option<String>,
    /// The combats the fight plays.
    pub combats: Option<u64>,
    /// The turn of the game the submission plays, counted from zero, the first
    /// unless given.
    pub turn: Option<usize>,
    /// The directory holding the inputs of the turn, the entries of the other
    /// seats by name.
    pub inputs: Option<String>,
}

/// Where the verifier of a turn without inputs looks for them: an empty
/// directory, so a game reads the same place either way.
const NO_INPUTS_PREFIX: &str = "ava-no-inputs-";

/// One request as the proxy sidecar logged it.
#[derive(serde::Deserialize)]
struct Record {
    host: String,
    status: u32,
    /// What the proxy logged for the completion of the request. Empty means the
    /// client went away before the answer was fully written, which is what the
    /// restart at the end of a turn does to whatever request is in flight. A
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

/// The document `score` prints, holding whatever was requested.
#[derive(serde::Serialize)]
struct Report {
    #[serde(skip_serializing_if = "Option::is_none")]
    verdict: Option<ava_wire::Verdict>,
    /// The file the game keeps as the entry of a passing submission.
    #[serde(skip_serializing_if = "Option::is_none")]
    entry: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fight: Option<ava_wire::Tally>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metrics: Option<ava_wire::Metrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attempts: Option<Vec<ava_wire::Attempt>>,
}

/// Run the score sub command and print the requested reports as one JSON document.
pub fn run(command: &Score) -> std::io::Result<i32> {
    let mut report = Report {
        verdict: None,
        entry: None,
        fight: None,
        metrics: None,
        attempts: None,
    };

    if let Some(name) = &command.game {
        let game = find(name)?;
        let turn = command.turn.unwrap_or_default();
        let played = game.turns().get(turn).ok_or_else(|| {
            std::io::Error::other(format!(
                "{name} has {} turns, there is no turn {turn}",
                game.turns().len()
            ))
        })?;

        match &command.fight {
            Some(directory) => {
                let directory = std::path::Path::new(directory);
                let fought = game.turns().last().expect("a game has a turn").entry;
                let tally = game.fight(
                    &directory.join(FIRST_DIRECTORY).join(fought),
                    &directory.join(SECOND_DIRECTORY).join(fought),
                    command.combats.unwrap_or(DEFAULT_COMBATS),
                )?;
                log::info!(
                    "the {name} fight went {} won, {} drawn, {} lost for the first entry",
                    tally.won,
                    tally.drawn,
                    tally.lost
                );
                report.fight = Some(tally);
            }
            None => {
                let no_inputs =
                    std::env::temp_dir().join(format!("{NO_INPUTS_PREFIX}{}", std::process::id()));
                let inputs = match &command.inputs {
                    Some(inputs) => std::path::PathBuf::from(inputs),
                    None => {
                        std::fs::create_dir_all(&no_inputs)?;
                        no_inputs.clone()
                    }
                };
                let verdict =
                    game.verify(turn, std::path::Path::new(SUBMISSION_DIRECTORY), &inputs)?;
                let _ = std::fs::remove_dir(&no_inputs);
                log::info!(
                    "the {name} submission of turn {turn} {}{}",
                    if verdict.passed { "passed" } else { "failed" },
                    if verdict.defeated.is_empty() {
                        String::new()
                    } else {
                        format!(", defeating {}", verdict.defeated.join(" "))
                    }
                );
                report.verdict = Some(verdict);
                report.entry = Some(played.entry);
            }
        }
    }

    if let Some(log) = &command.metrics {
        report.metrics = Some(aggregate_metrics(log)?);
    }

    if let Some(log) = &command.attempts {
        report.attempts = Some(read_attempts(log)?);
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(std::io::Error::other)?
    );

    Ok(0)
}

/// The game registered under `name`, or the error naming the known ones.
pub fn find(name: &str) -> std::io::Result<&'static dyn ava_game::Game> {
    ava_game::find(name).ok_or_else(|| {
        let known: Vec<&str> = ava_game::GAMES.iter().map(|game| game.name()).collect();
        std::io::Error::other(format!(
            "unknown game `{name}`, known are: {}",
            known.join(", ")
        ))
    })
}

/// Aggregate the proxy access log at `log` into one [`ava_wire::Metrics`].
pub fn aggregate_metrics(log: &str) -> std::io::Result<ava_wire::Metrics> {
    let contents = std::fs::read_to_string(log)
        .map_err(|error| std::io::Error::other(format!("{log}: {error}")))?;
    let metrics =
        aggregate(&contents).map_err(|error| std::io::Error::other(format!("{log}: {error}")))?;

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

/// Aggregate `records`, the JSON lines of a proxy access log, into one
/// [`ava_wire::Metrics`]: what the run record is completed with, and what the
/// interface runs over the output of the proxy of a live run.
pub fn aggregate(records: &str) -> std::io::Result<ava_wire::Metrics> {
    let mut metrics = ava_wire::Metrics::default();
    let mut first_token_seconds = 0f64;
    let mut streamed_requests = 0u64;

    for line in records.lines().filter(|line| line.starts_with('{')) {
        let record: Record = serde_json::from_str(line).map_err(std::io::Error::other)?;

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

/// Every attempt in the attempts log at `log`, in the order they were graded.
pub fn read_attempts(log: &str) -> std::io::Result<Vec<ava_wire::Attempt>> {
    let contents = std::fs::read_to_string(log)
        .map_err(|error| std::io::Error::other(format!("{log}: {error}")))?;

    let attempts = contents
        .lines()
        .filter(|line| line.starts_with('{'))
        .map(|line| {
            serde_json::from_str(line)
                .map_err(|error| std::io::Error::other(format!("{log}: {error}")))
        })
        .collect::<std::io::Result<Vec<ava_wire::Attempt>>>()?;

    log::info!(
        "{log}: {} attempts, {} passed",
        attempts.len(),
        attempts
            .iter()
            .filter(|attempt| attempt.verdict.passed)
            .count()
    );

    Ok(attempts)
}

fn record_distinct(seen: &mut Vec<String>, value: &str) {
    if !seen.iter().any(|entry| entry == value) {
        seen.push(value.to_string());
    }
}

#[cfg(test)]
mod tests {
    const ANSWERED: &str = r#"{"host":"api.anthropic.com","status":200,"completed":"OK","request_bytes":10,"response_bytes":20,"request_seconds":1.5,"header_seconds":"0.2","upstream_seconds":"1.4","first_token_seconds":0.3,"served_models":"claude-sonnet-5","input_tokens":100,"output_tokens":40,"cache_read_tokens":0,"cache_write_tokens":0,"streamed_deltas":5,"ratelimits":"","gateway_cost":"0.01"}"#;
    const CUT: &str = r#"{"host":"api.anthropic.com","status":200,"completed":"OK","request_bytes":10,"response_bytes":20,"request_seconds":2.0,"header_seconds":"0.2","upstream_seconds":"1.9","first_token_seconds":0.5,"served_models":"claude-sonnet-5","input_tokens":100,"output_tokens":0,"cache_read_tokens":0,"cache_write_tokens":0,"streamed_deltas":9,"ratelimits":"","gateway_cost":""}"#;

    #[test]
    fn records_aggregate_without_a_file() {
        let metrics = super::aggregate(&format!("{ANSWERED}\n{CUT}\n")).unwrap();

        assert_eq!(metrics.requests, 2);
        assert_eq!(metrics.truncated_requests, 1);
        assert_eq!(metrics.output_tokens, 40);
        assert_eq!(metrics.served_models, ["claude-sonnet-5"]);
        assert!((metrics.gateway_cost - 0.01).abs() < f64::EPSILON);
        assert!((metrics.mean_first_token_seconds - 0.4).abs() < 1e-9);
    }

    #[test]
    fn a_broken_record_is_an_error() {
        assert!(super::aggregate("{not json\n").is_err());
    }
}
