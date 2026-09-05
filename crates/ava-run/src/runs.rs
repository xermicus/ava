//! The runs on disk: the record of each, the entries it kept and the entry of
//! record among them.

/// The report a run from before the wire format was completed by, read for
/// the metrics and the end it holds.
const LEGACY_REPORT_FILE: &str = "score.json";

#[derive(Default, serde::Deserialize)]
struct LegacyReport {
    #[serde(default)]
    metrics: Option<ava_wire::Metrics>,
}

/// The record of the run in `directory`.
///
/// A run from before the wire format left its metrics and its end in a
/// separate report, which is read in their place when the record has neither.
pub fn read(directory: &std::path::Path) -> std::io::Result<ava_wire::Run> {
    let path = directory.join(crate::docker::RUN_FILE);
    let contents = std::fs::read_to_string(&path)
        .map_err(|error| std::io::Error::other(format!("{}: {error}", path.display())))?;
    let mut run: ava_wire::Run = serde_json::from_str(&contents)
        .map_err(|error| std::io::Error::other(format!("{}: {error}", path.display())))?;

    if run.finished_seconds.is_none() {
        let legacy = directory.join(LEGACY_REPORT_FILE);
        if let Ok(contents) = std::fs::read_to_string(&legacy) {
            let report: LegacyReport = serde_json::from_str(&contents).unwrap_or_default();
            run.metrics = report.metrics;
            run.finished_seconds = modified_seconds(&legacy);
        }
    }

    Ok(run)
}

/// The analysis record of the run in `directory`, if any.
pub fn analysis(directory: &std::path::Path) -> std::io::Result<Option<ava_wire::Analysis>> {
    let path = directory.join(crate::docker::ANALYSIS_FILE);
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(std::io::Error::other(format!(
                "{}: {error}",
                path.display()
            )));
        }
    };

    serde_json::from_str(&contents)
        .map(Some)
        .map_err(|error| std::io::Error::other(format!("{}: {error}", path.display())))
}

/// Every run on disk with a readable record, together with its directory, in
/// no particular order.
pub fn all() -> std::io::Result<Vec<(std::path::PathBuf, ava_wire::Run)>> {
    let mut runs = Vec::new();

    for directory in crate::docker::run_directories()? {
        if let Ok(run) = read(&directory) {
            runs.push((directory, run));
        }
    }

    Ok(runs)
}

/// An entry a run kept: the file of a passing attempt, ranked.
#[derive(Clone, Debug)]
pub struct Entry {
    /// The seconds of the attempt the entry came from.
    pub seconds: u64,
    pub path: std::path::PathBuf,
    pub bytes: u64,
    /// The points the game ranks the entry at, or nothing for a game with
    /// nothing to rank beyond passing.
    pub points: Option<u64>,
}

/// Every entry the run in `directory` kept, oldest first.
pub fn entries(
    game: &dyn ava_game::Game,
    directory: &std::path::Path,
) -> std::io::Result<Vec<Entry>> {
    let kept = directory.join(crate::docker::ENTRIES_DIRECTORY);
    let mut entries = Vec::new();

    let Ok(attempts) = std::fs::read_dir(&kept) else {
        return Ok(entries);
    };

    for attempt in attempts {
        let attempt = attempt?;
        let Some(seconds) = attempt
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u64>().ok())
        else {
            continue;
        };
        let path = attempt.path().join(game.entry());
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };

        entries.push(Entry {
            seconds,
            points: game.points(&path)?,
            path,
            bytes: metadata.len(),
        });
    }

    entries.sort_by_key(|entry| entry.seconds);

    Ok(entries)
}

/// The entry of record of the run in `directory`: the one ranking highest,
/// the newest one on ties.
pub fn entry_of_record(
    game: &dyn ava_game::Game,
    directory: &std::path::Path,
) -> std::io::Result<Option<Entry>> {
    Ok(entries(game, directory)?
        .into_iter()
        .max_by_key(|entry| (entry.points, entry.seconds)))
}

/// The epoch second the file at `path` was last written, if it is there.
pub fn modified_seconds(path: &std::path::Path) -> Option<u64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(
        modified
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs(),
    )
}
