//! The `remote` sub command: the git remote of one run, served on the score socket.
//!
//! A thin CGI shim around `git http-backend`: the sandbox pushes and fetches
//! over plain HTTP through the proxy, and the verifying happens in the receive
//! hooks of the repository, not here. The `score` host verifies a posted tar
//! without recording it. Requests are answered one at a time, which makes one
//! answered request the proof that a scoring in flight has finished.

use std::io::{Read, Write};

/// The git remote command, run inside the scoring container.
#[derive(Debug, Default)]
pub struct Remote {
    /// Where to listen instead of the score socket.
    pub socket: Option<String>,
    /// The directory holding the repository instead of the agent home.
    pub root: Option<String>,
}

/// Where the shim listens unless the command names another socket.
const SOCKET_PATH: &str = "/run/ava/score.sock";

/// The directory holding the bare repository, unless the command names another one.
const PROJECT_ROOT: &str = "/home/agent";

/// The identity everything past binding the socket runs under.
const SANDBOX_UID: u32 = 1000;
const SANDBOX_GID: u32 = 1000;

/// The sandbox connects as the sandbox user, which the root owned socket must admit.
const SOCKET_MODE: u32 = 0o666;

const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;
const BACKEND_TIMEOUT_SECONDS: &str = "180";

/// The same bound the receive hook puts on a scoring.
const SCORE_TIMEOUT_SECONDS: &str = "90";

/// How coreutils `timeout` reports an expired deadline.
const TIMEOUT_STATUS: i32 = 124;

/// The host a posted tar is scored under.
const SCORER_HOST: &str = "score";

/// Written by the scorer entrypoint next to the repository.
const GAME_FILE: &str = "game";

/// Mounted next to the repository when the run attacks the entry of another.
const CHALLENGE_DIRECTORY: &str = "challenge";

/// One scratch directory is enough, requests are answered one at a time.
const SCRATCH_PREFIX: &str = "ava-score-";
const TARBALL_FILE: &str = "submission.tar";

const STATUS_HEADER: &str = "status";
const CONTENT_LENGTH_HEADER: &str = "content-length";
const CONTENT_TYPE_HEADER: &str = "Content-Type";
const HOST_HEADER: &str = "Host";
const JSON_CONTENT_TYPE: &str = "application/json";

unsafe extern "C" {
    fn getuid() -> u32;
    fn setgroups(count: usize, groups: *const u32) -> i32;
    fn setgid(gid: u32) -> i32;
    fn setuid(uid: u32) -> i32;
}

/// One buffered answer, the only response shape this shim sends.
type Answer = tiny_http::Response<std::io::Cursor<Vec<u8>>>;

/// Bind the socket, drop to the sandbox user and answer forever.
pub fn run(command: &Remote) -> std::io::Result<i32> {
    let socket = command.socket.as_deref().unwrap_or(SOCKET_PATH);
    let root = command.root.as_deref().unwrap_or(PROJECT_ROOT);

    match std::fs::remove_file(socket) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error),
        _ => {}
    }

    let server = tiny_http::Server::http_unix(std::path::Path::new(socket))
        .map_err(|error| std::io::Error::other(format!("{socket}: {error}")))?;

    std::fs::set_permissions(
        socket,
        std::os::unix::fs::PermissionsExt::from_mode(SOCKET_MODE),
    )?;
    drop_privileges()?;

    log::info!("serving {root} on {socket}");

    loop {
        let mut request = server.recv()?;

        let answer = if scoring(&request) {
            score(&mut request, root)
                .unwrap_or_else(|error| refuse(500, &format!("scoring failed: {error}")))
        } else {
            backend(&mut request, root)
                .unwrap_or_else(|error| refuse(500, &format!("the git backend failed: {error}")))
        };

        if let Err(error) = request.respond(answer) {
            log::warn!("answering the sandbox failed: {error}");
        }
    }
}

/// Whether `request` came in under the scorer host.
fn scoring(request: &tiny_http::Request) -> bool {
    header(request, HOST_HEADER).is_some_and(|host| host == SCORER_HOST)
}

/// The `name` header of `request`.
fn header(request: &tiny_http::Request, name: &'static str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|header| header.field.equiv(name))
        .map(|header| header.value.to_string())
}

/// The request body, refused past the cap.
fn body(request: &mut tiny_http::Request) -> Result<Vec<u8>, Answer> {
    let mut body = Vec::new();
    request
        .as_reader()
        .take(MAX_BODY_BYTES as u64 + 1)
        .read_to_end(&mut body)
        .map_err(|error| refuse(500, &format!("reading the body failed: {error}")))?;

    if body.len() > MAX_BODY_BYTES {
        return Err(refuse(
            413,
            &format!("the body exceeds {MAX_BODY_BYTES} bytes"),
        ));
    }

    Ok(body)
}

/// Score a posted tar the way a push is scored, without recording an attempt.
fn score(request: &mut tiny_http::Request, root: &str) -> std::io::Result<Answer> {
    if *request.method() != tiny_http::Method::Post {
        return Ok(refuse(405, "post a tar archive of the tree to score"));
    }

    let tarball = match body(request) {
        Ok(tarball) => tarball,
        Err(answer) => return Ok(answer),
    };

    let game = std::fs::read_to_string(std::path::Path::new(root).join(GAME_FILE))?;

    let scratch = std::env::temp_dir().join(format!("{SCRATCH_PREFIX}{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(scratch.join(crate::score::SUBMISSION_DIRECTORY))?;

    let challenge = std::path::Path::new(root).join(CHALLENGE_DIRECTORY);
    let answer = unpack_and_score(
        &scratch,
        &tarball,
        game.trim(),
        challenge.is_dir().then_some(challenge.as_path()),
    );
    let _ = std::fs::remove_dir_all(&scratch);

    answer
}

/// Unpack `tarball` under `scratch` and run the verifier over it, against the
/// `challenge` when the run attacks one.
fn unpack_and_score(
    scratch: &std::path::Path,
    tarball: &[u8],
    game: &str,
    challenge: Option<&std::path::Path>,
) -> std::io::Result<Answer> {
    let archive = scratch.join(TARBALL_FILE);
    std::fs::write(&archive, tarball)?;

    let unpacked = std::process::Command::new("tar")
        .arg("-xf")
        .arg(&archive)
        .arg("-C")
        .arg(scratch.join(crate::score::SUBMISSION_DIRECTORY))
        .output()?;
    if !unpacked.status.success() {
        return Ok(refuse(
            400,
            &format!(
                "the body is not a tar archive: {}",
                String::from_utf8_lossy(&unpacked.stderr).trim()
            ),
        ));
    }

    let mut verifier = std::process::Command::new("timeout");
    verifier
        .arg(SCORE_TIMEOUT_SECONDS)
        .arg(std::env::current_exe()?)
        .args(["score", "--game", game]);
    if let Some(challenge) = challenge {
        verifier.arg("--challenge").arg(challenge);
    }
    let scored = verifier.current_dir(scratch).output()?;

    let _ = std::io::stderr().write_all(&scored.stderr);

    if scored.status.code() == Some(TIMEOUT_STATUS) {
        return Ok(refuse(500, "scoring timed out"));
    }
    if !scored.status.success() {
        return Ok(refuse(
            500,
            &format!(
                "scoring failed: {}",
                String::from_utf8_lossy(&scored.stderr).trim()
            ),
        ));
    }

    Ok(tiny_http::Response::from_data(scored.stdout).with_header(
        tiny_http::Header::from_bytes(CONTENT_TYPE_HEADER, JSON_CONTENT_TYPE)
            .expect("a fixed header parses"),
    ))
}

/// Answer one request with `git http-backend`, speaking CGI on its behalf.
fn backend(request: &mut tiny_http::Request, root: &str) -> std::io::Result<Answer> {
    let method = match request.method() {
        tiny_http::Method::Get => "GET",
        tiny_http::Method::Post => "POST",
        _ => return Ok(refuse(405, "only GET and POST are served")),
    };

    let (path, query) = match request.url().split_once('?') {
        Some((path, query)) => (path.to_string(), query.to_string()),
        None => (request.url().to_string(), String::new()),
    };

    let content_type = header(request, CONTENT_TYPE_HEADER).unwrap_or_default();

    let body = match body(request) {
        Ok(body) => body,
        Err(answer) => return Ok(answer),
    };

    let mut child = std::process::Command::new("timeout")
        .args([BACKEND_TIMEOUT_SECONDS, "git", "http-backend"])
        .env("GIT_PROJECT_ROOT", root)
        .env("GIT_HTTP_EXPORT_ALL", "1")
        .env("REQUEST_METHOD", method)
        .env("PATH_INFO", path)
        .env("QUERY_STRING", query)
        .env("CONTENT_TYPE", content_type)
        .env("CONTENT_LENGTH", body.len().to_string())
        .env("REMOTE_ADDR", "sandbox")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().expect("stdin is piped");
    let mut stdout_pipe = child.stdout.take().expect("stdout is piped");
    let mut stderr_pipe = child.stderr.take().expect("stderr is piped");

    // The pipes are pumped concurrently: a push is larger than a pipe buffer
    // and a clone answer is too, so feeding before reading could deadlock.
    let (stdout, stderr) = std::thread::scope(|scope| {
        scope.spawn(move || {
            let _ = stdin.write_all(&body);
        });
        let errors = scope.spawn(move || {
            let mut collected = Vec::new();
            let _ = stderr_pipe.read_to_end(&mut collected);
            collected
        });

        let mut collected = Vec::new();
        let _ = stdout_pipe.read_to_end(&mut collected);

        (
            collected,
            errors.join().expect("reading stderr does not panic"),
        )
    });

    let status = child.wait()?;
    if !status.success() {
        if status.code() == Some(TIMEOUT_STATUS) {
            return Ok(refuse(500, "the git backend timed out"));
        }
        let detail = String::from_utf8_lossy(&stderr);
        return Ok(refuse(
            500,
            &format!("the git backend failed: {}", detail.trim()),
        ));
    }

    Ok(relay(&stdout))
}

/// The CGI output of the backend as a response, honoring its Status header.
fn relay(output: &[u8]) -> Answer {
    let boundary = output.windows(4).position(|window| window == b"\r\n\r\n");
    let (raw_headers, payload) = match boundary {
        Some(position) => (&output[..position], &output[position + 4..]),
        None => (output, &[][..]),
    };

    let mut status = 200u16;
    let mut answer = tiny_http::Response::from_data(payload.to_vec());

    for line in String::from_utf8_lossy(raw_headers).lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let (name, value) = (name.trim(), value.trim());

        if name.eq_ignore_ascii_case(STATUS_HEADER) {
            status = value
                .split_whitespace()
                .next()
                .and_then(|code| code.parse().ok())
                .unwrap_or(status);
        } else if !name.eq_ignore_ascii_case(CONTENT_LENGTH_HEADER)
            && let Ok(header) = tiny_http::Header::from_bytes(name.as_bytes(), value.as_bytes())
        {
            answer = answer.with_header(header);
        }
    }

    answer.with_status_code(status)
}

fn refuse(status: u16, reason: &str) -> Answer {
    tiny_http::Response::from_string(format!("{reason}\n")).with_status_code(status)
}

/// Root binds the socket on the root owned volume; nothing else may run as
/// root, least of all the hooks scoring the submissions.
fn drop_privileges() -> std::io::Result<()> {
    if unsafe { getuid() } != 0 {
        return Ok(());
    }

    if unsafe { setgroups(0, std::ptr::null()) } != 0
        || unsafe { setgid(SANDBOX_GID) } != 0
        || unsafe { setuid(SANDBOX_UID) } != 0
    {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}
