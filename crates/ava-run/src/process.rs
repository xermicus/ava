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

/// Log what `program` wrote to its standard error, which is where it logs.
fn relay(program: &str, stderr: &[u8]) {
    let written = String::from_utf8_lossy(stderr);

    for line in written.lines().filter(|line| !line.trim().is_empty()) {
        log::info!("{program}: {line}");
    }
}
