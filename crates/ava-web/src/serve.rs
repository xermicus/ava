//! The `serve` sub command: the http layer of the web interface.
//!
//! Reading views render from the records on disk; the actions start a run or
//! a round in a background thread, end a live run through its done marker
//! and change the seats of a tournament. The one vendored asset is tailwind,
//! compiling the utility classes in the browser.

use std::io::Read;

use ava_run::{docker, process, registry, tournament};

use crate::views;

const BIND_ADDRESS: &str = "127.0.0.1";

/// How long a wait for a request is before the interrupt flag is polled.
const INTERRUPT_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);

/// The port `serve` binds unless the command names another one.
pub const DEFAULT_PORT: u16 = 2828;

/// The web interface command.
#[derive(Debug)]
pub struct Serve {
    /// The local port the interface binds.
    pub port: u16,
}

impl Default for Serve {
    fn default() -> Self {
        Self { port: DEFAULT_PORT }
    }
}

const TAILWIND: &str = include_str!("../assets/tailwind.js");

/// The vendored interface fonts, served by file name.
const FONTS: [(&str, &[u8]); 4] = [
    (
        "geist-mono-latin-400-normal.woff2",
        include_bytes!("../assets/fonts/geist-mono-latin-400-normal.woff2"),
    ),
    (
        "geist-mono-latin-500-normal.woff2",
        include_bytes!("../assets/fonts/geist-mono-latin-500-normal.woff2"),
    ),
    (
        "geist-latin-400-normal.woff2",
        include_bytes!("../assets/fonts/geist-latin-400-normal.woff2"),
    ),
    (
        "geist-latin-600-normal.woff2",
        include_bytes!("../assets/fonts/geist-latin-600-normal.woff2"),
    ),
];

const HTML_CONTENT_TYPE: &str = "text/html; charset=utf-8";
const TEXT_CONTENT_TYPE: &str = "text/plain; charset=utf-8";
const BINARY_CONTENT_TYPE: &str = "application/octet-stream";
const JAVASCRIPT_CONTENT_TYPE: &str = "text/javascript";
const FONT_CONTENT_TYPE: &str = "font/woff2";

/// A form submission larger than this is not one of ours.
const MAX_FORM_BYTES: u64 = 16 * 1024;

/// The form fields choosing an agent: the harness, the model and the thinking
/// level, under a prefix telling apart the agents one form chooses.
pub(crate) const AGENT_FIELDS: [&str; 3] = ["agent", "model", "thinking"];

/// The prefix of the fields choosing the analyst on the start panel.
pub(crate) const ANALYST_PREFIX: &str = "analyst_";

/// The start fields carried back to the form, so a submission does not reset it.
const START_FIELDS: [&str; 12] = [
    "agent",
    "model",
    "game",
    "thinking",
    "limit",
    "parallel",
    "analyze",
    "force",
    "analyst_agent",
    "analyst_model",
    "analyst_thinking",
    "analyst_seconds",
];

/// The tournament creation fields carried back to its form.
const CREATE_FIELDS: [&str; 9] = [
    "name",
    "game",
    "limit",
    "combats",
    "analyze",
    "analyst_agent",
    "analyst_model",
    "analyst_thinking",
    "analyst_seconds",
];

/// The starts whose runs are not on disk yet, shown as starting rows.
static PENDING: std::sync::Mutex<Vec<(u64, views::Pending)>> = std::sync::Mutex::new(Vec::new());

/// Tickets telling the pending starts apart.
static PENDING_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// One buffered answer, the only response shape the interface sends.
type Answer = tiny_http::Response<std::io::Cursor<Vec<u8>>>;

/// Serve the web interface until interrupted.
pub fn run(command: &Serve) -> std::io::Result<i32> {
    let address = format!("{BIND_ADDRESS}:{}", command.port);
    let server = tiny_http::Server::http(&address)
        .map_err(|error| std::io::Error::other(format!("{address}: {error}")))?;

    views::watch_containers();
    log::info!("serving http://{address}");

    loop {
        if ava_run::interrupt::interrupted() {
            return Ok(0);
        }

        let Some(mut request) = server.recv_timeout(INTERRUPT_POLL_INTERVAL)? else {
            continue;
        };

        std::thread::spawn(move || {
            let response = respond(&mut request);
            if let Err(error) = request.respond(response) {
                log::warn!("answering the browser failed: {error}");
            }
        });
    }
}

/// Route one request to its view or action.
fn respond(request: &mut tiny_http::Request) -> Answer {
    let path = request
        .url()
        .split('?')
        .next()
        .unwrap_or_default()
        .to_string();
    let segments: Vec<&str> = path.trim_matches('/').split('/').collect();

    let query = request
        .url()
        .split_once('?')
        .map(|(_, query)| query.to_string());

    match request.method() {
        tiny_http::Method::Get => view(&segments, query.as_deref()),
        tiny_http::Method::Post => action(&segments, &form(request)),
        _ => plain_response(405, "only GET and POST are served\n"),
    }
}

/// Answer one reading request.
fn view(segments: &[&str], query: Option<&str>) -> Answer {
    let notice = views::Notice {
        started: query_value(query, "started"),
        refused: query_value(query, "refused"),
    };
    let selection = views::Selection {
        fields: START_FIELDS
            .iter()
            .chain(CREATE_FIELDS.iter())
            .chain(AGENT_FIELDS.iter())
            .map(|key| (key.to_string(), query_value(query, key)))
            .chain(AGENT_FIELDS.iter().map(|key| {
                let key = format!("{ANALYST_PREFIX}{key}");
                let value = query_value(query, &key);
                (key, value)
            }))
            .filter_map(|(key, value)| Some((key, value?)))
            .collect(),
    };

    let pending: Vec<views::Pending> = PENDING
        .lock()
        .expect("no thread panics while holding the starts")
        .iter()
        .map(|(_, start)| start.clone())
        .collect();

    let outcome = match segments {
        [""] => views::runs_page(&notice, &selection, &pending),
        ["scoreboard"] => views::scoreboard_page(),
        ["games"] => views::games_page(),
        ["games", name, "cover"] => {
            return match views::game_cover(name) {
                Some((contents, kind)) => {
                    tiny_http::Response::from_data(contents).with_header(content_type(kind))
                }
                None => plain_response(404, "no such cover\n"),
            };
        }
        ["tournaments"] => views::tournaments_page(&notice, &selection),
        ["tournament", name] => views::tournament_page(name, &notice, &selection),
        ["tournament", name, file] => {
            return match views::tournament_file(name, file) {
                Some(contents) => tiny_http::Response::from_data(contents)
                    .with_header(content_type(TEXT_CONTENT_TYPE)),
                None => plain_response(404, "no such file\n"),
            };
        }
        ["setup"] => views::setup_page(),
        ["assets", "tailwind.js"] => {
            return tiny_http::Response::from_string(TAILWIND)
                .with_header(content_type(JAVASCRIPT_CONTENT_TYPE));
        }
        ["assets", "fonts", file] => {
            return match FONTS.iter().find(|(name, _)| name == file) {
                Some((_, contents)) => tiny_http::Response::from_data(contents.to_vec())
                    .with_header(content_type(FONT_CONTENT_TYPE)),
                None => plain_response(404, "no such font\n"),
            };
        }
        ["run", name] => views::run_page(name, &notice),
        ["run", name, file] => {
            return match views::run_file(name, file) {
                Some(contents) => tiny_http::Response::from_data(contents)
                    .with_header(content_type(TEXT_CONTENT_TYPE)),
                None => plain_response(404, "no such file\n"),
            };
        }
        ["run", name, "entries", seconds, file] => {
            return match views::run_entry(name, seconds, file) {
                Some(contents) => tiny_http::Response::from_data(contents)
                    .with_header(content_type(BINARY_CONTENT_TYPE)),
                None => plain_response(404, "no such entry\n"),
            };
        }
        _ => return plain_response(404, "no such page\n"),
    };

    match outcome {
        Ok(page) => html_response(200, &page),
        Err(error) => {
            log::error!("/{}: {error}", segments.join("/"));
            html_response(500, &views::error_page(&error.to_string()))
        }
    }
}

/// Why an action did not go ahead.
///
/// A submission the browser could correct is not the same as an action the
/// server could not carry out, and the console must not report the two alike:
/// the first says the form was wrong, the second says this deployment needs
/// attention before any submission works.
enum Refusal {
    /// The submission asked for something that does not exist.
    Rejected(String),
    /// The submission was sound and the server could not act on it.
    Failed(String),
}

impl Refusal {
    /// What the browser is told.
    fn reason(&self) -> &str {
        match self {
            Self::Rejected(reason) | Self::Failed(reason) => reason,
        }
    }

    /// Report the refusal on the console at the severity it deserves.
    fn report(&self) {
        match self {
            Self::Rejected(reason) => log::warn!("refused: {reason}"),
            Self::Failed(reason) => log::error!("the action failed: {reason}"),
        }
    }
}

/// Answer one action request by sending the browser back to where it acted,
/// with the outcome in the query for the page to show.
///
/// An action that went ahead may name another page to land on, which is how
/// a created tournament opens.
fn action(segments: &[&str], form: &[(String, String)]) -> Answer {
    let (origin, carried, outcome) = match segments {
        ["start"] => (
            "/".to_string(),
            preserved(form, &START_FIELDS),
            start_run(form),
        ),
        ["run", name, "stop"] => (format!("/run/{name}"), String::new(), stop_run(name)),
        ["run", name, "analyze"] => (
            format!("/run/{name}"),
            String::new(),
            analyze_run(name, form),
        ),
        ["tournaments", "create"] => (
            "/tournaments".to_string(),
            preserved(form, &CREATE_FIELDS),
            create_tournament(form),
        ),
        ["tournament", name, "seat"] => (
            format!("/tournament/{name}"),
            preserved(form, &AGENT_FIELDS),
            seat(name, form),
        ),
        ["tournament", name, "unseat"] => (
            format!("/tournament/{name}"),
            String::new(),
            unseat(name, form),
        ),
        ["tournament", name, "play"] => (
            format!("/tournament/{name}"),
            String::new(),
            play_round(name, form),
        ),
        _ => return plain_response(404, "no such action\n"),
    };

    match outcome {
        Ok(Done { note, landing }) => redirect(&format!(
            "{}?started={}{carried}",
            landing.unwrap_or(origin),
            urlencode(&note)
        )),
        Err(refusal) => {
            refusal.report();
            redirect(&format!(
                "{origin}?refused={}{carried}",
                urlencode(refusal.reason())
            ))
        }
    }
}

/// An action that went ahead: what to tell the browser, and where, when not
/// back where it acted.
struct Done {
    note: String,
    landing: Option<String>,
}

impl Done {
    fn note(note: String) -> Self {
        Self {
            note,
            landing: None,
        }
    }
}

/// The submitted `fields` as query parameters, so the form the browser
/// returns to holds what was submitted instead of resetting.
fn preserved(form: &[(String, String)], fields: &[&str]) -> String {
    fields
        .iter()
        .map(|key| (key, value(form, key)))
        .filter(|(_, submitted)| !submitted.is_empty())
        .map(|(key, submitted)| format!("&{key}={}", urlencode(submitted)))
        .collect()
}

/// Start a run from the submitted form, in a thread of its own.
///
/// Everything that can be checked without running is checked here, so a
/// refusal reaches the browser instead of a log line: the thread only fails
/// on what docker does at runtime.
fn start_run(form: &[(String, String)]) -> Result<Done, Refusal> {
    let registry = registry::load().map_err(|error| Refusal::Failed(error.to_string()))?;
    let agent = agent_choice(&registry, form, "")?;

    let analyst = if value(form, "analyze") == "on" {
        let analyst = agent_choice(&registry, form, ANALYST_PREFIX)?;
        Some(docker::Analyst {
            name: analyst.harness,
            model: analyst.model,
            thinking: analyst.thinking,
            limit: analyst_seconds(form, ANALYST_PREFIX)?,
        })
    } else {
        None
    };

    let game = value(form, "game");
    let games = views::games().map_err(|error| Refusal::Failed(error.to_string()))?;
    if !games.iter().any(|known| known == game) {
        return Err(Refusal::Rejected(format!("unknown game `{game}`")));
    }

    let limit = limit_choice(form)?;
    let parallel: u64 = value(form, "parallel")
        .parse()
        .map_err(|_| Refusal::Rejected("the parallel count is a number".to_string()))?;
    if parallel == 0 {
        return Err(Refusal::Rejected(
            "the parallel count is at least 1".to_string(),
        ));
    }

    let command = docker::Agent {
        name: agent.harness,
        model: agent.model,
        game: game.to_string(),
        limit,
        parallel,
        thinking: agent.thinking,
        force_build_images: value(form, "force") == "on",
        analyst,
        challenge: None,
    };

    log::info!(
        "starting {} on {}: game {game}, thinking {}, {limit}s, {parallel} parallel{}",
        command.name,
        command.model,
        command.thinking.as_deref().unwrap_or("default"),
        if command.force_build_images {
            ", rebuilding the images"
        } else {
            ""
        }
    );

    let ticket = PENDING_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    PENDING
        .lock()
        .expect("no thread panics while holding the starts")
        .push((
            ticket,
            views::Pending {
                agent: command.name.clone(),
                model: command.model.clone(),
                game: command.game.clone(),
                thinking: command.thinking.clone().unwrap_or_default(),
                parallel: command.parallel,
                started: ava_run::usage::epoch_now(),
            },
        ));

    let note = format!("starting {} on {}", command.name, command.model);
    std::thread::spawn(move || {
        let outcome = docker::run_agent(&command);
        PENDING
            .lock()
            .expect("no thread panics while holding the starts")
            .retain(|(id, _)| *id != ticket);
        match outcome {
            Ok(code) => log::info!(
                "the {} run on {} finished with code {code}",
                command.name,
                command.model
            ),
            Err(error) => log::error!(
                "the {} run on {} failed: {error}",
                command.name,
                command.model
            ),
        }
    });

    Ok(Done::note(note))
}

/// The seconds the analyst under `prefix` is given, its default when the form
/// carries none.
fn analyst_seconds(form: &[(String, String)], prefix: &str) -> Result<u64, Refusal> {
    let submitted = value(form, &format!("{prefix}seconds"));
    if submitted.is_empty() {
        return Ok(docker::Analyst::DEFAULT_LIMIT_SECONDS);
    }

    let seconds: u64 = submitted
        .parse()
        .map_err(|_| Refusal::Rejected("the analyst seconds are a number".to_string()))?;

    docker::Analyst::checked_limit(seconds).map_err(|error| Refusal::Rejected(error.to_string()))
}

/// The seconds a run is given, from the submitted form.
fn limit_choice(form: &[(String, String)]) -> Result<u64, Refusal> {
    let limit: u64 = value(form, "limit")
        .parse()
        .map_err(|_| Refusal::Rejected("the seconds are a number".to_string()))?;

    docker::Agent::checked_limit(limit).map_err(|error| Refusal::Rejected(error.to_string()))
}

/// The agent chosen under the `prefix` fields of a form, checked against the
/// registry.
fn agent_choice(
    registry: &registry::Registry,
    form: &[(String, String)],
    prefix: &str,
) -> Result<ava_wire::Agent, Refusal> {
    let [harness_field, model_field, thinking_field] =
        AGENT_FIELDS.map(|field| format!("{prefix}{field}"));

    let harness = value(form, &harness_field);
    if !registry.harnesses.iter().any(|known| known.name == harness) {
        return Err(Refusal::Rejected(format!("unknown harness `{harness}`")));
    }

    let model = value(form, &model_field);
    if !registry.models.iter().any(|known| known.name == model) {
        return Err(Refusal::Rejected(format!("unknown model `{model}`")));
    }

    let thinking = match value(form, &thinking_field) {
        "" => None,
        level if registry::THINKING_LEVELS.contains(&level) => Some(level.to_string()),
        level => {
            return Err(Refusal::Rejected(format!(
                "unknown thinking level `{level}`"
            )));
        }
    };

    // A credential the host never set is the operator's to fix, unlike a
    // pairing this harness cannot serve, which is the form's to correct.
    registry
        .invocation(
            harness,
            model,
            "",
            thinking.as_deref(),
            registry::Start::Task,
        )
        .map_err(|error| {
            let reason = error.to_string();
            if registry::is_missing_credential(&error) {
                Refusal::Failed(reason)
            } else {
                Refusal::Rejected(reason)
            }
        })?;

    Ok(ava_wire::Agent {
        harness: harness.to_string(),
        model: model.to_string(),
        thinking,
    })
}

/// Analyze the run in a thread of its own.
fn analyze_run(name: &str, form: &[(String, String)]) -> Result<Done, Refusal> {
    views::run_directory(name).map_err(|error| Refusal::Rejected(error.to_string()))?;
    if views::analyzing(name) {
        return Err(Refusal::Rejected(format!(
            "{name} is being analyzed already"
        )));
    }

    let registry = registry::load().map_err(|error| Refusal::Failed(error.to_string()))?;
    let analyst = agent_choice(&registry, form, "")?;
    let seconds = analyst_seconds(form, "")?;

    let command = docker::Analyze {
        run: name.to_string(),
        analyst: docker::Analyst {
            name: analyst.harness,
            model: analyst.model,
            thinking: analyst.thinking,
            limit: seconds,
        },
    };

    let note = format!(
        "analyzing {name} with {} on {}",
        command.analyst.name, command.analyst.model
    );
    log::info!("{note}");

    std::thread::spawn(move || match docker::analyze(&command) {
        Ok(code) => log::info!("the analysis of {} finished with code {code}", command.run),
        Err(error) => log::error!("the analysis of {} failed: {error}", command.run),
    });

    Ok(Done::note(note))
}

/// End a live run early by leaving its done marker, as a release tag would.
fn stop_run(name: &str) -> Result<Done, Refusal> {
    views::run_directory(name).map_err(|error| Refusal::Rejected(error.to_string()))?;

    log::info!("stopping {name} through its done marker");

    process::run_and_assume_success(
        "docker",
        &[
            "exec",
            &docker::scorer_container(name),
            "touch",
            docker::DONE_MARKER,
        ],
    )
    .map(|_| Done::note(format!("stopping {name}")))
    .map_err(|error| Refusal::Failed(format!("stopping {name} failed: {error}")))
}

/// Create a tournament from the submitted form and open its lobby.
fn create_tournament(form: &[(String, String)]) -> Result<Done, Refusal> {
    let name = value(form, "name");
    let game = value(form, "game");
    let limit = limit_choice(form)?;
    let combats: u64 = value(form, "combats")
        .parse()
        .map_err(|_| Refusal::Rejected("the combats are a number".to_string()))?;
    let combats = tournament::checked_combats(combats)
        .map_err(|error| Refusal::Rejected(error.to_string()))?;
    let analyst = if value(form, "analyze") == "on" {
        let registry = registry::load().map_err(|error| Refusal::Failed(error.to_string()))?;
        Some(agent_choice(&registry, form, ANALYST_PREFIX)?)
    } else {
        None
    };

    tournament::create(
        name,
        game,
        limit,
        combats,
        analyst,
        analyst_seconds(form, ANALYST_PREFIX)?,
    )
    .map_err(|error| Refusal::Rejected(error.to_string()))?;

    Ok(Done {
        note: format!("{name} is open, seat the agents"),
        landing: Some(format!("/tournament/{name}")),
    })
}

/// Seat the agent chosen in the form in the named tournament.
fn seat(name: &str, form: &[(String, String)]) -> Result<Done, Refusal> {
    let registry = registry::load().map_err(|error| Refusal::Failed(error.to_string()))?;
    let agent = agent_choice(&registry, form, "")?;
    let label = agent.label();

    tournament::add_seat(name, &agent).map_err(|error| Refusal::Rejected(error.to_string()))?;

    Ok(Done::note(format!("seated {label}")))
}

/// Remove the seat named in the form from the named tournament.
fn unseat(name: &str, form: &[(String, String)]) -> Result<Done, Refusal> {
    let seat: usize = value(form, "seat")
        .parse()
        .map_err(|_| Refusal::Rejected("the seat is a number".to_string()))?;

    tournament::remove_seat(name, seat).map_err(|error| Refusal::Rejected(error.to_string()))?;

    Ok(Done::note(format!("removed seat {}", seat + 1)))
}

/// Play one round of the named tournament in a thread of its own, with at
/// most the submitted number of runs at once.
fn play_round(name: &str, form: &[(String, String)]) -> Result<Done, Refusal> {
    let record = tournament::load(name).map_err(|error| Refusal::Rejected(error.to_string()))?;
    let parallel = match value(form, "parallel") {
        "" => None,
        count => Some(
            count
                .parse::<usize>()
                .ok()
                .filter(|count| *count > 0)
                .ok_or_else(|| {
                    Refusal::Rejected("the parallel count is a number above zero".to_string())
                })?,
        ),
    };
    if tournament::playing(name) {
        return Err(Refusal::Rejected(format!(
            "{name} is playing a round already"
        )));
    }
    if record.seats.is_empty() {
        return Err(Refusal::Rejected(format!(
            "{name} has no seats, seat an agent first"
        )));
    }

    let round = record.rounds.len() + 1;
    let name = name.to_string();
    let note = format!("playing round {round} of {name}");
    log::info!("{note}");

    std::thread::spawn(
        move || match tournament::play_round(&name, false, parallel) {
            Ok(code) => log::info!("round {round} of {name} finished with code {code}"),
            Err(error) => log::error!("round {round} of {name} failed: {error}"),
        },
    );

    Ok(Done::note(note))
}

/// The submitted form fields, urldecoded.
fn form(request: &mut tiny_http::Request) -> Vec<(String, String)> {
    let mut body = String::new();
    let _ = request
        .as_reader()
        .take(MAX_FORM_BYTES)
        .read_to_string(&mut body);

    body.split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(key, value)| (urldecode(key), urldecode(value)))
        .collect()
}

/// Encode `text` for one urlencoded form or query token.
fn urlencode(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());

    for byte in text.bytes() {
        match byte {
            b' ' => encoded.push('+'),
            byte if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) => {
                encoded.push(byte as char)
            }
            byte => encoded.push_str(&format!("%{byte:02X}")),
        }
    }

    encoded
}

/// The urldecoded value `key` holds in the query string, if it is there.
fn query_value(query: Option<&str>, key: &str) -> Option<String> {
    query?
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(name, _)| *name == key)
        .map(|(_, value)| urldecode(value))
}

/// The first value submitted under `key`, or the empty string.
fn value<'a>(form: &'a [(String, String)], key: &str) -> &'a str {
    form.iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
        .unwrap_or("")
}

/// Decode one urlencoded form token.
fn urldecode(token: &str) -> String {
    let mut bytes = token.bytes();
    let mut decoded = Vec::with_capacity(token.len());

    while let Some(byte) = bytes.next() {
        match byte {
            b'+' => decoded.push(b' '),
            b'%' => {
                let high = bytes.next().unwrap_or(b'0');
                let low = bytes.next().unwrap_or(b'0');
                let pair = [high, low];
                let escaped = std::str::from_utf8(&pair)
                    .ok()
                    .and_then(|hex| u8::from_str_radix(hex, 16).ok());
                decoded.push(escaped.unwrap_or(b'?'));
            }
            plain => decoded.push(plain),
        }
    }

    String::from_utf8_lossy(&decoded).into_owned()
}

fn html_response(status: u16, page: &str) -> Answer {
    tiny_http::Response::from_string(page)
        .with_status_code(status)
        .with_header(content_type(HTML_CONTENT_TYPE))
}

fn plain_response(status: u16, message: &str) -> Answer {
    tiny_http::Response::from_string(message).with_status_code(status)
}

/// See the action through in the browser by loading `location` fresh.
fn redirect(location: &str) -> Answer {
    tiny_http::Response::from_data(Vec::new())
        .with_status_code(303)
        .with_header(
            tiny_http::Header::from_bytes("Location", location).expect("a plain path parses"),
        )
}

fn content_type(value: &str) -> tiny_http::Header {
    tiny_http::Header::from_bytes("Content-Type", value).expect("a fixed header parses")
}
