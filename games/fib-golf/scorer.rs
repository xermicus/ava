//! The fib-golf game, verified by running the submitted ELF and ranked by its size.

const GAME_NAME: &str = "fib-golf";
const SUBMISSION_BINARY: &str = "fibonacci";
const SEPARATOR: char = ' ';

/// The first 47 Fibonacci numbers, the largest N the task asks for.
///
/// The length is the bound the verifier tests up to, so it is the bound the
/// task states. Every value here fits in a signed 32 bit integer, which leaves
/// the width of the arithmetic a choice the submission makes rather than one
/// the verifier forces.
const FIBONACCI: [u64; 47] = [
    0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987, 1597, 2584, 4181, 6765,
    10946, 17711, 28657, 46368, 75025, 121393, 196418, 317811, 514229, 832040, 1346269, 2178309,
    3524578, 5702887, 9227465, 14930352, 24157817, 39088169, 63245986, 102334155, 165580141,
    267914296, 433494437, 701408733, 1134903170, 1836311903,
];

/// The size the task allows, which is the size an entry earns nothing at.
const SIZE_LIMIT: u64 = 1 << 14;

/// The size an entry earns everything at.
const SCORE_FLOOR: u64 = 1 << 7;

/// The points decay exponentially with the size, by e over this many bytes.
const SCORE_DECAY_BYTES: f64 = 1500.0;

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const RUN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const WAIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);

/// Just some random number made up by claude.
const OUTPUT_LIMIT: u64 = 1 << 20;

/// The fib-golf game, verified by running the submitted ELF and ranked by its size.
pub struct FibGolf;

impl crate::Game for FibGolf {
    fn name(&self) -> &'static str {
        GAME_NAME
    }

    fn entry(&self) -> &'static str {
        SUBMISSION_BINARY
    }

    /// Verify the ELF submitted as `fibonacci` by running it for every N.
    fn verify(
        &self,
        submission: &std::path::Path,
        _challenge: Option<&std::path::Path>,
    ) -> std::io::Result<ava_wire::Verdict> {
        let binary = submission.join(SUBMISSION_BINARY);

        let contents = match std::fs::read(&binary) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(crate::failed(format!(
                    "no {SUBMISSION_BINARY} in the submission, which holds {}",
                    listing(submission)
                )));
            }
            Err(error) => {
                return Ok(crate::failed(format!(
                    "{SUBMISSION_BINARY} cannot be read: {error}"
                )));
            }
        };

        let bytes = contents.len() as u64;

        if !contents.starts_with(&ELF_MAGIC) {
            return Ok(crate::failed(format!(
                "{SUBMISSION_BINARY} is {bytes} bytes but not an ELF, it starts with {:02x?}",
                &contents[..contents.len().min(ELF_MAGIC.len())]
            )));
        }

        log::info!("{SUBMISSION_BINARY} is an ELF of {bytes} bytes, running it");

        for n in 0..=FIBONACCI.len() as u64 {
            let Ok(Some((status, output))) = run_with_timeout(&binary, n) else {
                return Ok(crate::failed(format!(
                    "{SUBMISSION_BINARY} did not finish within {RUN_TIMEOUT:?} at N={n}"
                )));
            };

            let expected = expected_output(n);
            let correct = output.trim_ascii_end() == expected.as_bytes().trim_ascii_end();

            if !status.success() {
                return Ok(crate::failed(format!(
                    "{SUBMISSION_BINARY} exited with {status} at N={n}"
                )));
            }

            if !correct {
                return Ok(crate::failed(format!(
                    "wrong output at N={n}, first difference at byte {}: printed {} bytes where {} were expected",
                    difference(&output, expected.as_bytes()),
                    output.len(),
                    expected.len()
                )));
            }
        }

        if bytes > SIZE_LIMIT {
            return Ok(crate::failed(format!(
                "{SUBMISSION_BINARY} prints what it should but is {bytes} bytes, past the {SIZE_LIMIT} the task allows"
            )));
        }

        Ok(ava_wire::Verdict::passed())
    }

    /// The points of an entry by its size: they fall off as
    /// `e^(-(bytes - 128) / 1500)`, scaled so that 128 bytes earn everything
    /// and the size limit earns nothing.
    fn points(&self, entry: &std::path::Path) -> std::io::Result<u64> {
        Ok(earned_points(std::fs::metadata(entry)?.len()))
    }
}

/// What the submission folder holds.
fn listing(submission: &std::path::Path) -> String {
    let Ok(entries) = std::fs::read_dir(submission) else {
        return "nothing readable".to_string();
    };

    let names: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();

    if names.is_empty() {
        return "no files".to_string();
    }

    names.join(", ")
}

/// The byte where `output` starts to differ from `expected`.
fn difference(output: &[u8], expected: &[u8]) -> usize {
    output
        .iter()
        .zip(expected)
        .position(|(printed, wanted)| printed != wanted)
        .unwrap_or(output.len().min(expected.len()))
}

/// The points for a passing ELF of `bytes`.
fn earned_points(bytes: u64) -> u64 {
    let decay = |bytes: u64| (-((bytes - SCORE_FLOOR) as f64) / SCORE_DECAY_BYTES).exp();
    let at_limit = decay(SIZE_LIMIT);
    let share = (decay(bytes.clamp(SCORE_FLOOR, SIZE_LIMIT)) - at_limit) / (1.0 - at_limit);

    (crate::MAXIMUM_POINTS as f64 * share).round() as u64
}

/// The first `n` fibonacci numbers, space separated.
fn expected_output(n: u64) -> String {
    FIBONACCI[..n as usize]
        .iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(&SEPARATOR.to_string())
}

/// Run `binary` on `n` and return its exit status and standard output, or
/// `None` after killing it on timeout.
fn run_with_timeout(
    binary: &std::path::Path,
    n: u64,
) -> std::io::Result<Option<(std::process::ExitStatus, Vec<u8>)>> {
    let mut child = std::process::Command::new(binary)
        .arg(n.to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()?;

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
    fn the_curve_spans_the_scale() {
        assert_eq!(
            super::earned_points(super::SCORE_FLOOR),
            crate::MAXIMUM_POINTS
        );
        assert_eq!(super::earned_points(1), crate::MAXIMUM_POINTS);
        assert_eq!(super::earned_points(super::SIZE_LIMIT), 0);
        assert!(super::earned_points(1000) > super::earned_points(2000));
    }
}
