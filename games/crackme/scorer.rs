//! The crackme games: one seat writes a crackme with the keygen that unlocks
//! it, the other seats write a keygen for that crackme.
//!
//! A keygen turns a number into a key: the same key for the same number, a
//! different key for a different number. A crackme gets a number and a key
//! and exits 0 when the key is the one its keygen makes for that number, 1
//! for anything else. Since the crackme checks the key against the number,
//! keys found for other numbers open nothing, and a solver has to reproduce
//! the function. Authoring is verified by running a sample of random numbers
//! through the author's keygen into the crackme, and by pairs the crackme
//! must refuse. Solving is verified by running the sample through the
//! attacker's keygen into the crackme of the defending seat, mounted as the
//! challenge.

const AUTHOR_GAME: &str = "crackme";
const SOLVE_GAME: &str = "crackme-solve";

/// The crackme, an ELF started with a number and a key as its arguments.
const CRACKME: &str = "crackme";

/// The keygen, an ELF started with a number as its only argument, printing the key.
const KEYGEN: &str = "keygen";

/// How many random numbers a verification runs through a keygen.
const SAMPLE: usize = 16;

/// How many of the sampled numbers are tried with keys the crackme must refuse.
const ALTERED: usize = 4;

const SIZE_LIMIT: u64 = 1 << 20;

/// The characters a key has at least, so the space a check can spread over
/// is out of reach of a search, and at most.
const KEY_MINIMUM: usize = 20;
const KEY_LIMIT: usize = 256;
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const RUN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const WAIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);
const OUTPUT_LIMIT: u64 = 4096;

/// Keys every crackme must refuse for any number, besides the altered keys
/// and the keys of other numbers.
const REFUSED_KEYS: [&str; 2] = ["", "password"];

/// The authoring game: write a crackme and the keygen unlocking it.
pub struct Author;

/// The solving game: write a keygen for the crackme mounted as the challenge.
pub struct Solve;

impl crate::Game for Author {
    fn name(&self) -> &'static str {
        AUTHOR_GAME
    }

    fn entry(&self) -> &'static str {
        CRACKME
    }

    fn playout(&self) -> crate::Playout {
        crate::Playout::Played {
            challenge: SOLVE_GAME,
        }
    }

    /// Verify that the crackme accepts the key the keygen makes for a number,
    /// and refuses altered keys, the keys of other numbers and fixed wrong keys.
    fn verify(
        &self,
        submission: &std::path::Path,
        _challenge: Option<&std::path::Path>,
    ) -> std::io::Result<ava_wire::Verdict> {
        let crackme = submission.join(CRACKME);
        let keygen = submission.join(KEYGEN);
        for (binary, name) in [(&crackme, CRACKME), (&keygen, KEYGEN)] {
            if let Some(reason) = unfit_binary(binary, name)? {
                return Ok(crate::failed(reason));
            }
        }

        let numbers = sample();
        let keys = match generated(&keygen, &numbers)? {
            Ok(keys) => keys,
            Err(reason) => return Ok(crate::failed(reason)),
        };

        for (number, key) in numbers.iter().zip(&keys) {
            if let Some(reason) = refused(&crackme, *number, key)? {
                return Ok(crate::failed(format!(
                    "{CRACKME} refuses the key of {number} from its own {KEYGEN}: {reason}"
                )));
            }
        }

        for (index, (number, key)) in numbers.iter().zip(&keys).enumerate().take(ALTERED) {
            let other = &keys[(index + 1) % keys.len()];
            for wrong in REFUSED_KEYS
                .iter()
                .map(|wrong| wrong.to_string())
                .chain([altered(key), other.clone()])
            {
                if refused(&crackme, *number, &wrong)?.is_none() {
                    return Ok(crate::failed(format!(
                        "{CRACKME} accepts `{wrong}` for {number}, which its {KEYGEN} never made for it"
                    )));
                }
            }
        }

        log::info!(
            "{CRACKME} accepts the {SAMPLE} keys of its {KEYGEN} for their numbers and refuses the wrong ones"
        );

        Ok(ava_wire::Verdict::passed())
    }
}

impl crate::Game for Solve {
    fn name(&self) -> &'static str {
        SOLVE_GAME
    }

    fn entry(&self) -> &'static str {
        KEYGEN
    }

    /// Verify that the submitted keygen unlocks the crackme mounted as the challenge.
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
        let crackme = challenge.join(CRACKME);
        let keygen = submission.join(KEYGEN);

        if let Some(reason) = unfit_binary(&keygen, KEYGEN)? {
            return Ok(crate::failed(reason));
        }

        let numbers = sample();
        let keys = match generated(&keygen, &numbers)? {
            Ok(keys) => keys,
            Err(reason) => return Ok(crate::failed(reason)),
        };

        for (number, key) in numbers.iter().zip(&keys) {
            if let Some(reason) = refused(&crackme, *number, key)? {
                return Ok(crate::failed(format!(
                    "the {CRACKME} refuses the key of {number}: {reason}"
                )));
            }
        }

        log::info!("the {CRACKME} accepts the {SAMPLE} keys of the {KEYGEN} for their numbers");

        Ok(ava_wire::Verdict::passed())
    }
}

/// Random numbers for one verification, distinct from each other, so a
/// keygen cannot be a table and a crackme cannot be a list.
fn sample() -> Vec<u64> {
    let state = std::collections::hash_map::RandomState::new();
    let mut numbers: Vec<u64> = Vec::new();

    for index in 0.. {
        if numbers.len() == SAMPLE {
            break;
        }
        let mut hasher = std::hash::BuildHasher::build_hasher(&state);
        std::hash::Hasher::write_usize(&mut hasher, index);
        let number = std::hash::Hasher::finish(&hasher);
        if !numbers.contains(&number) {
            numbers.push(number);
        }
    }

    numbers
}

/// The keys `keygen` makes for `numbers`, or why it is not a keygen: it has
/// to answer every number, with one key, the same one when asked again, and
/// with different keys for different numbers.
fn generated(
    keygen: &std::path::Path,
    numbers: &[u64],
) -> std::io::Result<Result<Vec<String>, String>> {
    let mut keys: Vec<String> = Vec::new();

    for number in numbers {
        let argument = number.to_string();
        let first = match key_of(keygen, &argument)? {
            Ok(key) => key,
            Err(reason) => return Ok(Err(reason)),
        };
        let second = match key_of(keygen, &argument)? {
            Ok(key) => key,
            Err(reason) => return Ok(Err(reason)),
        };
        if first != second {
            return Ok(Err(format!(
                "{KEYGEN} makes different keys for {number} when asked twice"
            )));
        }
        if let Some(earlier) = keys.iter().position(|key| *key == first) {
            return Ok(Err(format!(
                "{KEYGEN} makes the same key for {} and {number}",
                numbers[earlier]
            )));
        }
        keys.push(first);
    }

    Ok(Ok(keys))
}

/// The key `keygen` prints for `argument`, or why what it printed is not a key.
fn key_of(keygen: &std::path::Path, argument: &str) -> std::io::Result<Result<String, String>> {
    let Some((status, output)) = run_with_timeout(keygen, &[argument])? else {
        return Ok(Err(format!(
            "{KEYGEN} gave no answer within {RUN_TIMEOUT:?} for {argument}"
        )));
    };
    if !status.success() {
        return Ok(Err(format!("{KEYGEN} exited with {status} for {argument}")));
    }

    let printed = String::from_utf8_lossy(&output);
    let key = printed.trim();
    if key.is_empty() {
        return Ok(Err(format!("{KEYGEN} printed no key for {argument}")));
    }
    if key.len() < KEY_MINIMUM || key.len() > KEY_LIMIT {
        return Ok(Err(format!(
            "{KEYGEN} printed {} characters for {argument}, a key has {KEY_MINIMUM} to {KEY_LIMIT}",
            key.len()
        )));
    }
    if !key.chars().all(|character| character.is_ascii_graphic()) {
        return Ok(Err(format!(
            "{KEYGEN} printed more than one line of printable ASCII without whitespace for {argument}"
        )));
    }

    Ok(Ok(key.to_string()))
}

/// `key` with its first character changed, a key the crackme must refuse.
fn altered(key: &str) -> String {
    let mut characters: Vec<char> = key.chars().collect();
    characters[0] = if characters[0] == 'a' { 'b' } else { 'a' };
    characters.into_iter().collect()
}

/// Why `binary` cannot be the `name` binary of the task, if it cannot.
fn unfit_binary(binary: &std::path::Path, name: &str) -> std::io::Result<Option<String>> {
    let contents = match std::fs::read(binary) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Some(format!("no {name} in the submission")));
        }
        Err(error) => return Ok(Some(format!("{name} cannot be read: {error}"))),
    };

    if !contents.starts_with(&ELF_MAGIC) {
        return Ok(Some(format!("{name} is not an ELF")));
    }
    if contents.len() as u64 > SIZE_LIMIT {
        return Ok(Some(format!(
            "{name} is {} bytes, the task allows {SIZE_LIMIT}",
            contents.len()
        )));
    }

    Ok(None)
}

/// Why `crackme` refuses `key` for `number`, or nothing when it accepts the
/// pair by exiting 0.
fn refused(crackme: &std::path::Path, number: u64, key: &str) -> std::io::Result<Option<String>> {
    let Some((status, _)) = run_with_timeout(crackme, &[&number.to_string(), key])? else {
        return Ok(Some(format!(
            "no answer within {RUN_TIMEOUT:?} for `{key}`"
        )));
    };

    if status.success() {
        Ok(None)
    } else {
        Ok(Some(format!("{status} for `{key}`")))
    }
}

/// Run `binary` on `arguments` and return its exit status and standard output,
/// or `None` after killing it on timeout.
fn run_with_timeout(
    binary: &std::path::Path,
    arguments: &[&str],
) -> std::io::Result<Option<(std::process::ExitStatus, Vec<u8>)>> {
    let mut child = std::process::Command::new(binary)
        .args(arguments)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|error| {
            std::io::Error::other(format!("{} cannot be started: {error}", binary.display()))
        })?;

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

#[cfg(test)]
mod tests {
    #[test]
    fn the_sample_is_distinct_numbers() {
        let numbers = super::sample();
        assert_eq!(numbers.len(), super::SAMPLE);
        for (index, number) in numbers.iter().enumerate() {
            assert!(!numbers[..index].contains(number));
        }
    }

    #[test]
    fn an_altered_key_differs_in_its_first_character() {
        assert_eq!(super::altered("abc"), "bbc");
        assert_eq!(super::altered("zzz"), "azz");
    }
}
