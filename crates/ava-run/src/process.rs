//! Helpers for driving external programs.

/// The signal that asks whether a process is there and nothing else.
const NO_SIGNAL: i32 = 0;

unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}

/// Whether the process `pid` is running.
pub fn alive(pid: i32) -> bool {
    unsafe { kill(pid, NO_SIGNAL) == 0 }
}

/// Run `program` with `arguments` and return its trimmed standard output.
///
/// Anything other than a successful exit becomes an error carrying the trimmed
/// standard error output of the program.
pub fn run_and_assume_success(program: &str, arguments: &[&str]) -> std::io::Result<String> {
    let output = std::process::Command::new(program)
        .args(arguments)
        .output()?;

    if !output.status.success() {
        return Err(std::io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    relay(program, &output.stderr);

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// How often a wait for a program that may never answer looks at it.
const WAIT_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);

/// Run `program` with `arguments` and return its trimmed standard output,
/// giving up once `timeout` has passed.
pub fn run_within(
    program: &str,
    arguments: &[&str],
    timeout: std::time::Duration,
) -> std::io::Result<String> {
    let mut child = std::process::Command::new(program)
        .args(arguments)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait()? {
            Some(status) if !status.success() => {
                return Err(std::io::Error::other(format!("{program}: {status}")));
            }
            Some(_) => break,
            None if std::time::Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(std::io::Error::other(format!(
                    "{program} did not answer within {} seconds",
                    timeout.as_secs()
                )));
            }
            None => std::thread::sleep(WAIT_POLL_INTERVAL),
        }
    }

    let mut written = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        std::io::Read::read_to_string(&mut stdout, &mut written)?;
    }

    Ok(written.trim().to_string())
}

/// Log what `program` wrote to its standard error, which is where it logs.
fn relay(program: &str, stderr: &[u8]) {
    let written = String::from_utf8_lossy(stderr);

    for line in written.lines().filter(|line| !line.trim().is_empty()) {
        log::info!("{program}: {line}");
    }
}
