//! The tcc-opt game: the submission is a tinycc tree with a Makefile. The
//! verifier builds it, then builds and tests zlib, sqlite, libpng and tinycc
//! itself with the compiler it left, without and with `TCC_OPT_SIZE=1`, and
//! records the code size of every program both ways next to the code size the
//! unmodified compiler gives it. An entry ranks by how much smaller the
//! optimizing build leaves the code.

const GAME_NAME: &str = "tcc-opt";
const IMAGE: &str = "tcc-opt";

/// The entry kept of a passing push: the diff of the submitted tinycc tree
/// against the tree the task handed out.
const ENTRY: &str = "tinycc.patch";

/// The tinycc tree in the submission, and the copy the task handed out.
const TREE: &str = "tinycc";
const SCAFFOLD: &str = "/opt/scaffold";

/// What the image lays down for the game: the bootstrap compiler the
/// submission is built with, the sources of the suites, and the check script.
const ROOT: &str = "/opt/tcc-opt";
const BOOTSTRAP: &str = "bootstrap";
const CHECK: &str = "check";

/// The check script skips the tests in this mode.
const BUILD_ONLY: &str = "build";

/// What the verifier runs in the submission, and what it takes from the
/// directory the target installs into.
const MAKEFILE: &str = "Makefile";
const BUILD_TARGET: &str = "build";
const BUILD_DIRECTORY: &str = "build";
const TOOLCHAIN_PARTS: [&str; 2] = ["bin/tcc", "lib/tcc"];

/// The variable switching the optimizations on.
const SWITCH: &str = "TCC_OPT_SIZE";
const SWITCH_ON: &str = "1";

/// The suites, as the check script names them.
const SUITES: [&str; 4] = ["zlib", "sqlite", "libpng", "tinycc"];

/// The measurements of a suite, by the key the verdict records them under.
const OPTIMIZED: &str = "optimized";
const PLAIN: &str = "plain";
const REFERENCE: &str = "reference";

/// The share of the unmodified compiler's code the optimizing build has to
/// shave off to earn everything.
const FULL_REDUCTION: f64 = 0.5;

const BUILD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);
const CHECK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(900);

/// The build, the slowest check and a margin: the checks run in parallel.
const SCORING_SECONDS: u64 = 1260;

const WAIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);
const OUTPUT_LIMIT: u64 = 4 << 20;

/// How much of the output of a failing step the reason carries.
const REASON_LINES: usize = 12;
const REASON_BYTES: usize = 1500;

/// The path a step gets, cleared of everything else so the only compiler it
/// can reach is the one the submission built, and the compilers that must not
/// be on it.
const PATH: &str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";
const COMPILERS: [&str; 6] = ["cc", "gcc", "clang", "c++", "g++", "tcc"];

const SCRATCH_PREFIX: &str = "ava-tcc-opt-";
const SIGKILL: std::ffi::c_int = 9;

unsafe extern "C" {
    fn kill(pid: std::ffi::c_int, signal: std::ffi::c_int) -> std::ffi::c_int;
}

/// The tcc-opt game.
pub struct TccOpt;

const TURNS: [crate::Turn; 1] = [crate::single_turn(ENTRY)];

impl crate::Game for TccOpt {
    fn name(&self) -> &'static str {
        GAME_NAME
    }

    fn image(&self) -> Option<&'static str> {
        Some(IMAGE)
    }

    fn turns(&self) -> &[crate::Turn] {
        &TURNS
    }

    fn scoring_seconds(&self) -> u64 {
        SCORING_SECONDS
    }

    /// Measure the code the unmodified compiler gives every suite, once.
    fn prepare(&self) -> std::io::Result<()> {
        no_other_compiler()?;
        let scratch = Scratch::new()?;
        let sizes = reference::load(&scratch.path)?;

        for (suite, bytes) in SUITES.iter().zip(sizes) {
            log::info!("the unmodified compiler gives {suite} {bytes} bytes of code");
        }

        Ok(())
    }

    fn verify(
        &self,
        _turn: usize,
        submission: &std::path::Path,
        _inputs: &std::path::Path,
    ) -> std::io::Result<ava_wire::Verdict> {
        no_other_compiler()?;

        let submission = submission.canonicalize()?;
        if !submission.join(MAKEFILE).is_file() {
            return Ok(crate::failed(format!(
                "no {MAKEFILE} in the submission, the CI builds with `make {BUILD_TARGET}`"
            )));
        }
        if !submission.join(TREE).is_dir() {
            return Ok(crate::failed(format!(
                "no {TREE} directory in the submission"
            )));
        }

        let scratch = Scratch::new()?;
        write_patch(&submission, &scratch.path)?;

        if let Err(reason) = build(&submission)? {
            return Ok(crate::failed(reason));
        }
        let toolchain = match extract(&submission, &scratch.path)? {
            Ok(toolchain) => toolchain,
            Err(reason) => return Ok(crate::failed(reason)),
        };

        let cached = reference::read();
        let (checks, reference) = std::thread::scope(|scope| {
            let checks: Vec<_> = SUITES
                .iter()
                .flat_map(|suite| [(suite, false), (suite, true)])
                .map(|(suite, optimized)| {
                    let (toolchain, scratch) = (&toolchain, &scratch.path);
                    scope.spawn(move || check(toolchain, suite, optimized, scratch))
                })
                .collect();
            let reference = match cached {
                Some(sizes) => Ok(sizes),
                None => reference::load(&scratch.path),
            };
            let checks: std::io::Result<Vec<Checked>> = checks
                .into_iter()
                .map(|handle| handle.join().expect("a check does not panic"))
                .collect();
            (checks, reference)
        });
        let checks = checks?;
        let reference = reference?;

        for optimized in [false, true] {
            let failures: Vec<&Checked> = checks
                .iter()
                .filter(|checked| checked.optimized == optimized && checked.bytes.is_err())
                .collect();
            if let Some(failed) = failures.first() {
                let reason = failed.bytes.as_ref().expect_err("filtered for failures");
                return Ok(crate::failed(format!(
                    "{} {SWITCH}{}: {reason}",
                    failed.suite,
                    if optimized {
                        format!("={SWITCH_ON}")
                    } else {
                        " unset".to_string()
                    }
                )));
            }
        }

        let mut measurements = std::collections::BTreeMap::new();
        for checked in &checks {
            let bytes = *checked.bytes.as_ref().expect("failures returned above");
            let kind = if checked.optimized { OPTIMIZED } else { PLAIN };
            measurements.insert(key(checked.suite, kind), bytes);
        }
        for (suite, bytes) in SUITES.iter().zip(reference) {
            measurements.insert(key(suite, REFERENCE), bytes);
        }

        let reason = summary(&measurements);
        log::info!("{reason}");

        Ok(ava_wire::Verdict {
            reason: Some(reason),
            measurements,
            ..ava_wire::Verdict::passed()
        })
    }

    /// The points of the entry from the sizes the verdict recorded.
    fn points(
        &self,
        _entry: &std::path::Path,
        verdict: &ava_wire::Verdict,
    ) -> std::io::Result<Option<u64>> {
        Ok(earned_points(&verdict.measurements))
    }
}

/// One suite checked one way: the bytes of code of its program, or why it failed.
struct Checked {
    suite: &'static str,
    optimized: bool,
    bytes: Result<u64, String>,
}

/// The key a measurement of `suite` is recorded under.
fn key(suite: &str, kind: &str) -> String {
    format!("{suite}.{kind}")
}

/// The share of the unmodified compiler's code the optimizing build leaves,
/// averaged over the suites, or nothing when a measurement is missing.
fn relative_size(measurements: &std::collections::BTreeMap<String, u64>) -> Option<f64> {
    let mut total = 0.0;
    for suite in SUITES {
        let optimized = *measurements.get(&key(suite, OPTIMIZED))?;
        let reference = *measurements.get(&key(suite, REFERENCE))?;
        if reference == 0 {
            return None;
        }
        total += optimized as f64 / reference as f64;
    }

    Some(total / SUITES.len() as f64)
}

/// The points of an entry: everything for shaving [`FULL_REDUCTION`] off the
/// unmodified compiler's code on average, nothing for code no smaller, and a
/// proportional share in between.
fn earned_points(measurements: &std::collections::BTreeMap<String, u64>) -> Option<u64> {
    let reduction = 1.0 - relative_size(measurements)?;
    let share = (reduction / FULL_REDUCTION).clamp(0.0, 1.0);

    Some((crate::MAXIMUM_POINTS as f64 * share).round() as u64)
}

/// The reason of a passing verdict: every program's code with the switch on
/// against the unmodified compiler's, and the average reduction.
fn summary(measurements: &std::collections::BTreeMap<String, u64>) -> String {
    let sizes: Vec<String> = SUITES
        .iter()
        .map(|suite| {
            format!(
                "{suite} {}/{}",
                measurements.get(&key(suite, OPTIMIZED)).unwrap_or(&0),
                measurements.get(&key(suite, REFERENCE)).unwrap_or(&0)
            )
        })
        .collect();
    let reduction = 100.0 * (1.0 - relative_size(measurements).unwrap_or(1.0));

    format!(
        "bytes of code with {SWITCH}={SWITCH_ON} against the unmodified compiler: {}, {reduction:.1}% smaller on average",
        sizes.join(", ")
    )
}

/// The scratch directory of one verification, removed with it.
struct Scratch {
    path: std::path::PathBuf,
}

impl Scratch {
    fn new() -> std::io::Result<Self> {
        let path = std::env::temp_dir().join(format!("{SCRATCH_PREFIX}{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path)?;

        Ok(Self { path })
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Fail where a compiler other than the submission's is on the path, since
/// the test programs have to be compiled by the submission or not at all.
fn no_other_compiler() -> std::io::Result<()> {
    for directory in PATH.split(':') {
        for compiler in COMPILERS {
            let path = std::path::Path::new(directory).join(compiler);
            if path.exists() {
                return Err(std::io::Error::other(format!(
                    "{} is a C compiler the submission could hand its work to, the scoring image must have none",
                    path.display()
                )));
            }
        }
    }

    Ok(())
}

/// Write the entry: the diff of the submitted tree against the tree the task
/// handed out, under git style names.
fn write_patch(submission: &std::path::Path, scratch: &std::path::Path) -> std::io::Result<()> {
    let staged = scratch.join("patch");
    std::fs::create_dir_all(&staged)?;
    std::os::unix::fs::symlink(std::path::Path::new(SCAFFOLD).join(TREE), staged.join("a"))?;
    std::os::unix::fs::symlink(submission.join(TREE), staged.join("b"))?;

    let patch = std::fs::File::create(submission.join(ENTRY))?;
    let status = std::process::Command::new("diff")
        .args(["-ruN", "a", "b"])
        .current_dir(&staged)
        .stdin(std::process::Stdio::null())
        .stdout(patch)
        .stderr(std::process::Stdio::inherit())
        .status()?;

    // diff exits 1 on differences and 2 on trouble.
    match status.code() {
        Some(0 | 1) => Ok(()),
        _ => Err(std::io::Error::other(format!(
            "diffing the submitted {TREE} against {SCAFFOLD} failed with {status}"
        ))),
    }
}

/// Run `make build` in the submission and say why it failed if it did.
fn build(submission: &std::path::Path) -> std::io::Result<Result<(), String>> {
    log::info!("running make {BUILD_TARGET} in the submission");

    let mut command = std::process::Command::new("make");
    command.arg(BUILD_TARGET).current_dir(submission);
    environment(&mut command, submission, false);

    let ran = run(&mut command, BUILD_TIMEOUT)?;
    Ok(match ran {
        Ran::TimedOut { stderr } => Err(format!(
            "make {BUILD_TARGET} did not finish within {BUILD_TIMEOUT:?}: {}",
            tail(&stderr)
        )),
        Ran::Finished { status, stderr, .. } if !status.success() => Err(format!(
            "make {BUILD_TARGET} failed with {status}: {}",
            tail(&stderr)
        )),
        Ran::Finished { .. } => Ok(()),
    })
}

/// Copy the compiler and its runtime out of the build directory of the
/// submission, so the checks run nothing else of it.
fn extract(
    submission: &std::path::Path,
    scratch: &std::path::Path,
) -> std::io::Result<Result<std::path::PathBuf, String>> {
    let toolchain = scratch.join("toolchain");
    let built = submission.join(BUILD_DIRECTORY);

    for part in TOOLCHAIN_PARTS {
        let source = built.join(part);
        if !source.exists() {
            return Ok(Err(format!(
                "make {BUILD_TARGET} left no {BUILD_DIRECTORY}/{part}, which the CI takes from the build"
            )));
        }
        let target = toolchain.join(part);
        std::fs::create_dir_all(target.parent().expect("the parts have a parent"))?;
        let copied = std::process::Command::new("cp")
            .arg("-r")
            .arg(&source)
            .arg(&target)
            .status()?;
        if !copied.success() {
            return Err(std::io::Error::other(format!(
                "copying {} out of the build failed with {copied}",
                source.display()
            )));
        }
    }

    Ok(Ok(toolchain))
}

/// Build and test one suite with the compiler under `toolchain`, the switch
/// set when `optimized`, in a scratch directory of its own.
fn check(
    toolchain: &std::path::Path,
    suite: &'static str,
    optimized: bool,
    scratch: &std::path::Path,
) -> std::io::Result<Checked> {
    let bytes = measure(toolchain, suite, optimized, false, scratch)?;
    Ok(Checked {
        suite,
        optimized,
        bytes,
    })
}

/// Run the check script on `suite` with the compiler under `toolchain`, in a
/// scratch directory of its own, and read the bytes of code of the program it
/// built, or why it failed.
fn measure(
    toolchain: &std::path::Path,
    suite: &str,
    optimized: bool,
    build_only: bool,
    scratch: &std::path::Path,
) -> std::io::Result<Result<u64, String>> {
    let root = std::path::Path::new(ROOT);
    let directory = scratch.join(format!(
        "{suite}-{}",
        match (build_only, optimized) {
            (true, _) => REFERENCE,
            (false, true) => OPTIMIZED,
            (false, false) => PLAIN,
        }
    ));
    std::fs::create_dir_all(&directory)?;

    let mut command = std::process::Command::new(root.join(CHECK));
    command.arg(toolchain).arg(suite).current_dir(&directory);
    if build_only {
        command.arg(BUILD_ONLY);
    }
    environment(&mut command, &directory, optimized);

    log::info!("checking {suite} {}", directory.display());
    let ran = run(&mut command, CHECK_TIMEOUT)?;
    let _ = std::fs::remove_dir_all(&directory);

    Ok(match ran {
        Ran::TimedOut { stderr } => Err(format!(
            "did not finish within {CHECK_TIMEOUT:?}: {}",
            tail(&stderr)
        )),
        Ran::Finished { status, stderr, .. } if !status.success() => {
            Err(format!("failed with {status}: {}", tail(&stderr)))
        }
        Ran::Finished { stdout, stderr, .. } => size_line(&stdout, suite).ok_or_else(|| {
            format!(
                "printed no size for {suite}: {}",
                tail(&[stdout, stderr].concat())
            )
        }),
    })
}

/// The bytes the check script printed for `suite`, in its `suite program bytes` line.
fn size_line(stdout: &[u8], suite: &str) -> Option<u64> {
    String::from_utf8_lossy(stdout).lines().find_map(|line| {
        let mut words = line.split_whitespace();
        (words.next()? == suite).then(|| words.nth(1)?.parse().ok())?
    })
}

/// The environment of a step: cleared to a fixed path, its scratch directory
/// as home and temporary directory, and the switch when `optimized`.
fn environment(command: &mut std::process::Command, home: &std::path::Path, optimized: bool) {
    command
        .env_clear()
        .env("PATH", PATH)
        .env("HOME", home)
        .env("TMPDIR", home);
    if optimized {
        command.env(SWITCH, SWITCH_ON);
    }
}

/// How a command came out.
enum Ran {
    Finished {
        status: std::process::ExitStatus,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    },
    TimedOut {
        stderr: Vec<u8>,
    },
}

/// Run `command` in a process group of its own with its output captured, kill
/// the group when it is over or `timeout` is, so nothing it started outlives
/// it.
fn run(command: &mut std::process::Command, timeout: std::time::Duration) -> std::io::Result<Ran> {
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    std::os::unix::process::CommandExt::process_group(command, 0);

    let mut child = command.spawn().map_err(|error| {
        std::io::Error::other(format!(
            "{:?} cannot be started: {error}",
            command.get_program()
        ))
    })?;
    let group = child.id() as std::ffi::c_int;

    let stdout = collect(child.stdout.take().expect("stdout was requested piped"));
    let stderr = collect(child.stderr.take().expect("stderr was requested piped"));

    let deadline = std::time::Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if std::time::Instant::now() >= deadline {
            break None;
        }
        std::thread::sleep(WAIT_INTERVAL);
    };

    unsafe {
        kill(-group, SIGKILL);
    }
    if status.is_none() {
        child.wait()?;
    }
    let stdout = stdout.join().expect("the reader thread does not panic");
    let stderr = stderr.join().expect("the reader thread does not panic");

    Ok(match status {
        Some(status) => Ran::Finished {
            status,
            stdout,
            stderr,
        },
        None => Ran::TimedOut { stderr },
    })
}

/// Drain `pipe` to its end on a thread of its own, keeping only its last
/// [`OUTPUT_LIMIT`] bytes.
///
/// The pipe is always read to the end, so a step writing more than the limit,
/// as the sqlite tests do, does not fill the pipe and take an `EPIPE` on its
/// next write. Only the tail is kept, which bounds the memory and holds the
/// size line and the reason of a failure, both at the end of the output.
fn collect<R: std::io::Read + Send + 'static>(pipe: R) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut pipe = pipe;
        let mut output = Vec::new();
        let mut chunk = [0u8; 64 * 1024];
        while let Ok(read) = std::io::Read::read(&mut pipe, &mut chunk) {
            if read == 0 {
                break;
            }
            output.extend_from_slice(&chunk[..read]);
            if output.len() > 2 * OUTPUT_LIMIT as usize {
                output.drain(..output.len() - OUTPUT_LIMIT as usize);
            }
        }
        if output.len() > OUTPUT_LIMIT as usize {
            output.drain(..output.len() - OUTPUT_LIMIT as usize);
        }
        output
    })
}

/// The last lines of an output, as much as a reason carries.
fn tail(output: &[u8]) -> String {
    let text = String::from_utf8_lossy(output);
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let kept = lines[lines.len().saturating_sub(REASON_LINES)..].join("\n");
    let start = kept.len().saturating_sub(REASON_BYTES);
    let start = (start..kept.len())
        .find(|index| kept.is_char_boundary(*index))
        .unwrap_or(kept.len());

    kept[start..].trim().to_string()
}

/// The code sizes the unmodified compiler gives the suites, measured by
/// building them with the bootstrap compiler and cached in the temporary
/// directory under a name keyed on that compiler and the check script.
mod reference {
    use super::{BOOTSTRAP, CHECK, ROOT, SUITES};

    const FILE_PREFIX: &str = "ava-tcc-opt-reference-";

    const HASH_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
    const HASH_PRIME: u64 = 0x0000_0100_0000_01B3;

    fn hashed(fingerprint: &[u8]) -> u64 {
        let mut hash = HASH_OFFSET;

        for byte in fingerprint {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(HASH_PRIME);
        }

        hash
    }

    fn fingerprint() -> Vec<u8> {
        let root = std::path::Path::new(ROOT);
        let mut bytes = Vec::new();

        for file in [root.join(BOOTSTRAP).join("bin/tcc"), root.join(CHECK)] {
            if let Ok(metadata) = std::fs::metadata(&file) {
                bytes.extend_from_slice(
                    format!(
                        "{}{:?}{}",
                        file.display(),
                        metadata.modified(),
                        metadata.len()
                    )
                    .as_bytes(),
                );
            }
        }

        bytes
    }

    fn path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("{FILE_PREFIX}{:016x}", hashed(&fingerprint())))
    }

    /// The cached sizes in the order of the suites, if every suite has one.
    pub(super) fn read() -> Option<Vec<u64>> {
        let contents = std::fs::read_to_string(path()).ok()?;
        let sizes: Vec<u64> = contents
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?;

        (sizes.len() == SUITES.len()).then_some(sizes)
    }

    /// The sizes, measured in `scratch` unless cached.
    ///
    /// The cache is written best effort: `prepare` runs as a different user
    /// than a push, so its file may already be there and unwritable, which is
    /// no reason to fail a verification the measurement already answered.
    pub(super) fn load(scratch: &std::path::Path) -> std::io::Result<Vec<u64>> {
        if let Some(sizes) = read() {
            return Ok(sizes);
        }

        let sizes = measure(scratch)?;
        let written: Vec<String> = sizes.iter().map(u64::to_string).collect();
        let _ = std::fs::write(path(), written.join(" "));

        Ok(sizes)
    }

    /// Build every suite with the bootstrap compiler, in parallel under
    /// `scratch`, and take the code size of its program.
    fn measure(scratch: &std::path::Path) -> std::io::Result<Vec<u64>> {
        let bootstrap = std::path::Path::new(ROOT).join(BOOTSTRAP);

        let measured: Vec<std::io::Result<Result<u64, String>>> = std::thread::scope(|scope| {
            let handles: Vec<_> = SUITES
                .iter()
                .map(|suite| {
                    let bootstrap = &bootstrap;
                    scope.spawn(move || super::measure(bootstrap, suite, false, true, scratch))
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("a measurement does not panic"))
                .collect()
        });

        measured
            .into_iter()
            .zip(SUITES)
            .map(|(bytes, suite)| {
                bytes?.map_err(|reason| {
                    std::io::Error::other(format!(
                        "the unmodified compiler fails to build {suite}: {reason}"
                    ))
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::Game;

    fn measured(sizes: &[(&str, u64, u64)]) -> std::collections::BTreeMap<String, u64> {
        let mut measurements = std::collections::BTreeMap::new();
        for (suite, optimized, reference) in sizes {
            measurements.insert(super::key(suite, super::OPTIMIZED), *optimized);
            measurements.insert(super::key(suite, super::PLAIN), *reference);
            measurements.insert(super::key(suite, super::REFERENCE), *reference);
        }
        measurements
    }

    fn unchanged() -> Vec<(&'static str, u64, u64)> {
        super::SUITES
            .iter()
            .map(|suite| (*suite, 1000, 1000))
            .collect()
    }

    #[test]
    fn the_points_follow_the_average_reduction() {
        assert_eq!(super::earned_points(&measured(&unchanged())), Some(0));

        let mut halved = unchanged();
        for (_, optimized, _) in &mut halved {
            *optimized = 500;
        }
        assert_eq!(
            super::earned_points(&measured(&halved)),
            Some(crate::MAXIMUM_POINTS)
        );

        let mut one_suite = unchanged();
        one_suite[0].1 = 600;
        assert_eq!(super::earned_points(&measured(&one_suite)), Some(2000));

        let mut bigger = unchanged();
        bigger[1].1 = 1500;
        assert_eq!(super::earned_points(&measured(&bigger)), Some(0));

        assert_eq!(
            super::earned_points(&measured(&unchanged()[..2])),
            None,
            "a missing suite ranks nothing"
        );
    }

    #[test]
    fn points_read_the_verdict_alone() {
        let verdict = ava_wire::Verdict {
            measurements: measured(&unchanged()),
            ..ava_wire::Verdict::passed()
        };

        assert_eq!(
            super::TccOpt
                .points(std::path::Path::new(super::ENTRY), &verdict)
                .unwrap(),
            Some(0)
        );
        assert_eq!(
            super::TccOpt
                .points(
                    std::path::Path::new(super::ENTRY),
                    &ava_wire::Verdict::passed()
                )
                .unwrap(),
            None
        );
    }

    #[test]
    fn the_summary_names_every_suite() {
        let summary = super::summary(&measured(&unchanged()));

        for suite in super::SUITES {
            assert!(summary.contains(&format!("{suite} 1000/1000")), "{summary}");
        }
        assert!(summary.ends_with("0.0% smaller on average"), "{summary}");
    }

    #[test]
    fn the_size_line_is_the_suites_own() {
        let printed = b"configure: creating Makefile\nzlib minigzip 103478\n";

        assert_eq!(super::size_line(printed, "zlib"), Some(103478));
        assert_eq!(super::size_line(printed, "libpng"), None);
        assert_eq!(super::size_line(b"zlib minigzip many\n", "zlib"), None);
    }

    #[test]
    fn the_tail_keeps_the_last_lines_within_bounds() {
        let lines: Vec<String> = (0..40).map(|line| format!("line {line}")).collect();
        let tail = super::tail(lines.join("\n").as_bytes());

        assert!(tail.starts_with("line 28"), "{tail}");
        assert!(tail.ends_with("line 39"), "{tail}");

        let long = "x".repeat(4000);
        assert_eq!(super::tail(long.as_bytes()).len(), super::REASON_BYTES);
        assert_eq!(super::tail(b"\n\n"), "");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_environment_is_cleared_to_a_fixed_path() {
        let scratch = super::Scratch::new().unwrap();
        let mut command = std::process::Command::new("sh");
        command
            .arg("-c")
            .arg("test \"$PATH\" = \"$1\" && test -z \"$SECRET\"")
            .arg("sh")
            .arg(super::PATH);
        // SAFETY: the test is single threaded around this environment change.
        unsafe { std::env::set_var("SECRET", "leaked") };
        super::environment(&mut command, &scratch.path, false);

        let ran = super::run(&mut command, std::time::Duration::from_secs(10)).unwrap();
        assert!(matches!(
            ran,
            super::Ran::Finished { status, .. } if status.success()
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_command_past_its_timeout_is_killed_with_its_group() {
        let scratch = super::Scratch::new().unwrap();
        let mut command = std::process::Command::new("sh");
        command.arg("-c").arg("sleep 30 & sleep 30");
        super::environment(&mut command, &scratch.path, false);

        let started = std::time::Instant::now();
        let ran = super::run(&mut command, std::time::Duration::from_millis(200)).unwrap();

        assert!(matches!(ran, super::Ran::TimedOut { .. }));
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
    }
}
