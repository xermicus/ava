//! The `.env` file at the repository root, holding the keys of the backends.

/// The file holding one `NAME=value` line per variable.
const ENVIRONMENT_FILE: &str = ".env";
const COMMENT: char = '#';
const QUOTES: [char; 2] = ['"', '\''];

/// Merge the file under the process environment: a variable the environment
/// already carries keeps its value, and a missing file is an empty one.
///
/// Runs before any thread exists, which is what makes writing the environment
/// sound.
pub(crate) fn load() -> std::io::Result<()> {
    let contents = match std::fs::read_to_string(ENVIRONMENT_FILE) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(std::io::Error::other(format!(
                "{ENVIRONMENT_FILE}: {error}"
            )));
        }
    };

    for line in contents.lines().map(str::trim) {
        if line.is_empty() || line.starts_with(COMMENT) {
            continue;
        }

        let Some((name, value)) = line.split_once('=') else {
            return Err(std::io::Error::other(format!(
                "{ENVIRONMENT_FILE}: `{line}` is not a NAME=value line"
            )));
        };
        let name = name.trim();

        if std::env::var_os(name).is_none() {
            unsafe { std::env::set_var(name, unquoted(value.trim())) };
        }
    }

    Ok(())
}

/// `value` without one pair of matching quotes around it.
fn unquoted(value: &str) -> &str {
    for quote in QUOTES {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner;
        }
    }

    value
}
