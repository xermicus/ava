//! The crackme games: one seat authors a crackme, the others recover its key.
//!
//! Authoring is verified by running the crackme with the key its author
//! submitted and with keys it must refuse. Solving is verified by running the
//! crackme of the defending seat, mounted as the challenge, with the key the
//! attacker submitted.

const AUTHOR_GAME: &str = "crackme";
const SOLVE_GAME: &str = "crackme-solve";

/// The crackme, an ELF started with the key as its only argument.
const BINARY: &str = "crackme";

/// The key, one line beside the crackme when authoring, the submission when solving.
const KEY_FILE: &str = "key";

/// What the crackme prints for the right key.
const ACCEPTED: &str = "OK";

const SIZE_LIMIT: u64 = 1 << 20;
const KEY_LIMIT: usize = 256;
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const RUN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const WAIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);
const OUTPUT_LIMIT: u64 = 4096;

/// A key every crackme must refuse besides the corrupted secret.
const WRONG_KEYS: [&str; 2] = ["", "password"];

/// The authoring game: write a crackme and submit its key.
pub struct Author;

/// The solving game: recover the key of the crackme mounted as the challenge.
pub struct Solve;

impl crate::Game for Author {
    fn name(&self) -> &'static str {
        AUTHOR_GAME
    }

    fn entry(&self) -> &'static str {
        BINARY
    }

    fn playout(&self) -> crate::Playout {
        crate::Playout::Played {
            challenge: SOLVE_GAME,
        }
    }

    /// Verify that the crackme accepts the submitted key and refuses others.
    fn verify(
        &self,
        submission: &std::path::Path,
        _challenge: Option<&std::path::Path>,
    ) -> std::io::Result<ava_wire::Verdict> {
        let binary = submission.join(BINARY);
        if let Some(reason) = unfit_binary(&binary)? {
            return Ok(crate::failed(reason));
        }

        let key = match read_key(&submission.join(KEY_FILE))? {
            Ok(key) => key,
            Err(reason) => return Ok(crate::failed(reason)),
        };

        if let Some(reason) = refused(&binary, &key)? {
            return Ok(crate::failed(format!(
                "{BINARY} refuses its own {KEY_FILE}: {reason}"
            )));
        }

        let mut corrupted = key.clone();
        let first = corrupted.remove(0);
        corrupted.insert(0, if first == 'a' { 'b' } else { 'a' });
        for wrong in WRONG_KEYS.iter().copied().chain([corrupted.as_str()]) {
            if refused(&binary, wrong)?.is_none() {
                return Ok(crate::failed(format!(
                    "{BINARY} accepts `{wrong}` as well as its {KEY_FILE}"
                )));
            }
        }

        log::info!("{BINARY} accepts its {KEY_FILE} and refuses the wrong ones");

        Ok(ava_wire::Verdict::passed())
    }
}

impl crate::Game for Solve {
    fn name(&self) -> &'static str {
        SOLVE_GAME
    }

    fn entry(&self) -> &'static str {
        KEY_FILE
    }

    /// Verify the submitted key against the crackme mounted as the challenge.
    fn verify(
        &self,
        submission: &std::path::Path,
        challenge: Option<&std::path::Path>,
    ) -> std::io::Result<ava_wire::Verdict> {
        let Some(challenge) = challenge else {
            return Err(std::io::Error::other(format!(
                "{SOLVE_GAME} verifies against a challenge and none was mounted"
            )));
        };

        let key = match read_key(&submission.join(KEY_FILE))? {
            Ok(key) => key,
            Err(reason) => return Ok(crate::failed(reason)),
        };

        match refused(&challenge.join(BINARY), &key)? {
            Some(reason) => Ok(crate::failed(format!(
                "the {BINARY} refuses the {KEY_FILE}: {reason}"
            ))),
            None => {
                log::info!("the {BINARY} accepts the {KEY_FILE}");
                Ok(ava_wire::Verdict::passed())
            }
        }
    }
}

/// Why `binary` cannot be a crackme, if it cannot.
fn unfit_binary(binary: &std::path::Path) -> std::io::Result<Option<String>> {
    let contents = match std::fs::read(binary) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Some(format!("no {BINARY} in the submission")));
        }
        Err(error) => return Ok(Some(format!("{BINARY} cannot be read: {error}"))),
    };

    if !contents.starts_with(&ELF_MAGIC) {
        return Ok(Some(format!("{BINARY} is not an ELF")));
    }
    if contents.len() as u64 > SIZE_LIMIT {
        return Ok(Some(format!(
            "{BINARY} is {} bytes, the task allows {SIZE_LIMIT}",
            contents.len()
        )));
    }

    Ok(None)
}

/// The key in the file at `path`, or why it is not one.
fn read_key(path: &std::path::Path) -> std::io::Result<Result<String, String>> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Err(format!("no {KEY_FILE} in the submission")));
        }
        Err(error) => return Ok(Err(format!("{KEY_FILE} cannot be read: {error}"))),
    };

    let key = contents.trim();
    if key.is_empty() {
        return Ok(Err(format!("{KEY_FILE} is empty")));
    }
    if key.len() > KEY_LIMIT {
        return Ok(Err(format!(
            "{KEY_FILE} is {} characters, the task allows {KEY_LIMIT}",
            key.len()
        )));
    }
    if !key.chars().all(|character| character.is_ascii_graphic()) {
        return Ok(Err(format!(
            "{KEY_FILE} is not one line of printable ASCII without whitespace"
        )));
    }

    Ok(Ok(key.to_string()))
}

/// Why `binary` refuses `key`, or nothing when it accepts it: exits 0 and
/// prints the accepting word.
fn refused(binary: &std::path::Path, key: &str) -> std::io::Result<Option<String>> {
    let Some((status, output)) = run_with_timeout(binary, key)? else {
        return Ok(Some(format!(
            "no answer within {RUN_TIMEOUT:?} for `{key}`"
        )));
    };

    if !status.success() {
        return Ok(Some(format!(
            "exit {} for `{key}`",
            status.code().unwrap_or(-1)
        )));
    }

    let printed = String::from_utf8_lossy(&output);
    if printed.trim() != ACCEPTED {
        return Ok(Some(format!(
            "exit 0 without printing {ACCEPTED} for `{key}`"
        )));
    }

    Ok(None)
}

/// Run `binary` on `key` and return its exit status and standard output, or
/// `None` after killing it on timeout.
fn run_with_timeout(
    binary: &std::path::Path,
    key: &str,
) -> std::io::Result<Option<(std::process::ExitStatus, Vec<u8>)>> {
    let mut child = match std::process::Command::new(binary)
        .arg(key)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return Err(std::io::Error::other(format!(
                "{BINARY} cannot be started: {error}"
            )));
        }
    };

    let stdout = child.stdout.take().expect("stdout was requested piped");
    let reader = std::thread::spawn(move || {
        let mut output = Vec::new();
        let mut capped = std::io::Read::take(stdout, OUTPUT_LIMIT);
        std::io::Read::read_to_end(&mut capped, &mut output).map(|_| output)
    });

    let deadline = std::time::Instant::now() + RUN_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            child.kill()?;
            child.wait()?;
            return Ok(None);
        }
        std::thread::sleep(WAIT_INTERVAL);
    };

    let output = reader.join().expect("the reader thread does not panic")?;
    Ok(Some((status, output)))
}
