//! The fib-golf game, scored against the size of the submitted ELF.

const GAME_NAME: &str = "fib-golf";
const SUBMISSION_BINARY: &str = "fibonacci";
const SEPARATOR: char = ' ';

/// The first 47 Fibonacci numbers, the largest N the task asks for.
///
/// The length is the bound the scorer tests up to, so it is the bound the task
/// states. Every value here fits in a signed 32 bit integer, which leaves the
/// width of the arithmetic a choice the submission makes rather than one the
/// scorer forces.
const FIBONACCI: [u64; 47] = [
    0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987, 1597, 2584, 4181, 6765,
    10946, 17711, 28657, 46368, 75025, 121393, 196418, 317811, 514229, 832040, 1346269, 2178309,
    3524578, 5702887, 9227465, 14930352, 24157817, 39088169, 63245986, 102334155, 165580141,
    267914296, 433494437, 701408733, 1134903170, 1836311903,
];

/// The size the ELF of a solving submission is scored against: every halving
/// of the size below the ceiling earns the same share of the points, so a
/// smaller ELF scores higher across the whole range.
const SCORE_CEILING: u64 = 1 << 15;

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const RUN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const WAIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);

/// Just some random number made up by claude.
const OUTPUT_LIMIT: u64 = 1 << 20;

/// The fib-golf game, scored against the size of the submitted ELF.
pub struct FibGolf;

impl crate::Game for FibGolf {
    fn name(&self) -> &'static str {
        GAME_NAME
    }

    /// Score the ELF submitted as `fibonacci`.
    fn score(&self, submission: &std::path::Path) -> std::io::Result<crate::Score> {
        let binary = submission.join(SUBMISSION_BINARY);

        let contents = match std::fs::read(&binary) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(unsolved(&format!(
                    "no {SUBMISSION_BINARY} in the tarball, which holds {}",
                    listing(submission)
                )));
            }
            Err(error) => {
                return Ok(unsolved(&format!(
                    "{SUBMISSION_BINARY} cannot be read: {error}"
                )));
            }
        };

        let bytes = contents.len() as u64;

        if !contents.starts_with(&ELF_MAGIC) {
            return Ok(unsolved(&format!(
                "{SUBMISSION_BINARY} is {bytes} bytes but not an ELF, it starts with {:02x?}",
                &contents[..contents.len().min(ELF_MAGIC.len())]
            )));
        }

        log::info!("{SUBMISSION_BINARY} is an ELF of {bytes} bytes, running it");

        for n in 0..=FIBONACCI.len() as u64 {
            let Ok(Some((status, output))) = run_with_timeout(&binary, n) else {
                return Ok(unsolved(&format!(
                    "{SUBMISSION_BINARY} did not finish within {RUN_TIMEOUT:?} at N={n}"
                )));
            };

            let expected = expected_output(n);
            let correct = output.trim_ascii_end() == expected.as_bytes().trim_ascii_end();

            if !status.success() {
                return Ok(unsolved(&format!(
                    "{SUBMISSION_BINARY} exited with {status} at N={n}"
                )));
            }

            if !correct {
                return Ok(unsolved(&format!(
                    "wrong output at N={n}, first difference at byte {}: printed {} bytes where {} were expected",
                    difference(&output, expected.as_bytes()),
                    output.len(),
                    expected.len()
                )));
            }
        }

        let points = earned_points(bytes);
        if points == 0 {
            return Ok(unsolved(&format!(
                "{SUBMISSION_BINARY} works but earns nothing at {bytes} bytes, the ceiling is {SCORE_CEILING}"
            )));
        }

        Ok(crate::Score {
            game: GAME_NAME,
            solved: true,
            points,
            reason: None,
        })
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

/// The points granted for a solving ELF of `bytes`, log scaled so that every
/// halving of the size earns the same share of the points.
fn earned_points(bytes: u64) -> u64 {
    if bytes >= SCORE_CEILING {
        return 0;
    }

    let total_halvings = (SCORE_CEILING as f64).log2();
    let earned_halvings = (SCORE_CEILING as f64 / bytes as f64).log2();

    (crate::MAXIMUM_POINTS as f64 * earned_halvings / total_halvings).round() as u64
}

/// The score of a submission which does not solve the task.
fn unsolved(reason: &str) -> crate::Score {
    log::info!("{reason}");

    crate::Score {
        game: GAME_NAME,
        solved: false,
        points: 0,
        reason: Some(reason.to_string()),
    }
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
