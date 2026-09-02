//! The `serve` sub command: the http layer of the web interface.
//!
//! Reading views render from the run artifacts on disk; the two actions
//! start a run in a background thread and end a live one through its done
//! marker. The one vendored asset is tailwind, compiling the utility
//! classes in the browser.

use std::io::Read;

use ava_run::{docker, process, registry};

use crate::views;

const BIND_ADDRESS: &str = "127.0.0.1";

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
const JAVASCRIPT_CONTENT_TYPE: &str = "text/javascript";
const FONT_CONTENT_TYPE: &str = "font/woff2";

/// A form submission larger than this is not one of ours.
const MAX_FORM_BYTES: u64 = 16 * 1024;

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

    log::info!("serving http://{address}");

    loop {
        let mut request = server.recv()?;

        if ava_run::interrupt::interrupted() {
            return Ok(0);
        }

        let response = respond(&mut request);
        if let Err(error) = request.respond(response) {
            log::warn!("answering the browser failed: {error}");
        }
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
        agent: query_value(query, "agent"),
        model: query_value(query, "model"),
        game: query_value(query, "game"),
        thinking: query_value(query, "thinking"),
        limit: query_value(query, "limit"),
        parallel: query_value(query, "parallel"),
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
fn action(segments: &[&str], form: &[(String, String)]) -> Answer {
    let (origin, carried, outcome) = match segments {
        ["start"] => ("/".to_string(), preserved(form), start_run(form)),
        ["run", name, "stop"] => (format!("/run/{name}"), String::new(), stop_run(name)),
        _ => return plain_response(404, "no such action\n"),
    };

    match outcome {
        Ok(note) => redirect(&format!("{origin}?started={}{carried}", urlencode(&note))),
        Err(refusal) => {
            refusal.report();
            redirect(&format!(
                "{origin}?refused={}{carried}",
                urlencode(refusal.reason())
            ))
        }
    }
}

/// The submitted start fields as query parameters, so the form the browser
/// returns to holds what was submitted instead of resetting.
fn preserved(form: &[(String, String)]) -> String {
    ["agent", "model", "game", "thinking", "limit", "parallel"]
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
fn start_run(form: &[(String, String)]) -> Result<String, Refusal> {
    let registry = registry::load().map_err(|error| Refusal::Failed(error.to_string()))?;

    let name = value(form, "agent");
    if !registry
        .harnesses
        .iter()
        .any(|harness| harness.name == name)
    {
        return Err(Refusal::Rejected(format!("unknown harness `{name}`")));
    }

    let model = value(form, "model");
    if !registry.models.iter().any(|known| known.name == model) {
        return Err(Refusal::Rejected(format!("unknown model `{model}`")));
    }

    let game = value(form, "game");
    let games = views::games().map_err(|error| Refusal::Failed(error.to_string()))?;
    if !games.iter().any(|known| known == game) {
        return Err(Refusal::Rejected(format!("unknown game `{game}`")));
    }

    let thinking = match value(form, "thinking") {
        "" => None,
        level if registry::THINKING_LEVELS.contains(&level) => Some(level.to_string()),
        level => {
            return Err(Refusal::Rejected(format!(
                "unknown thinking level `{level}`"
            )));
        }
    };

    let limit: u64 = value(form, "limit")
        .parse()
        .map_err(|_| Refusal::Rejected("the seconds are a number".to_string()))?;
    let parallel: u64 = value(form, "parallel")
        .parse()
        .map_err(|_| Refusal::Rejected("the parallel count is a number".to_string()))?;
    if limit == 0 || parallel == 0 {
        return Err(Refusal::Rejected(
            "the seconds and the parallel count are at least 1".to_string(),
        ));
    }

    // A credential the host never set is the operator's to fix, unlike a
    // pairing this harness cannot serve, which is the form's to correct.
    registry
        .invocation(name, model, "", thinking.as_deref(), registry::Start::Task)
        .map_err(|error| {
            let reason = error.to_string();
            if registry::is_missing_credential(&error) {
                Refusal::Failed(reason)
            } else {
                Refusal::Rejected(reason)
            }
        })?;

    let command = docker::Agent {
        name: name.to_string(),
        model: model.to_string(),
        game: game.to_string(),
        limit,
        parallel,
        thinking,
        force_build_images: value(form, "force") == "on",
    };

    log::info!(
        "starting {name} on {model}: game {game}, thinking {}, {limit}s, {parallel} parallel{}",
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
                started: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|now| now.as_secs())
                    .unwrap_or(0),
            },
        ));

    let note = format!("starting {name} on {model}");
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

    Ok(note)
}

/// End a live run early by leaving its done marker, as a release tag would.
fn stop_run(name: &str) -> Result<String, Refusal> {
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
    .map(|_| format!("stopping {name}"))
    .map_err(|error| Refusal::Failed(format!("stopping {name} failed: {error}")))
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
