//! The games benchmark submissions are verified and ranked by.

#[path = "../crackme/scorer.rs"]
pub mod crackme;
#[path = "../fib-golf/scorer.rs"]
pub mod fib_golf;
#[path = "../r2wars-x86-32/scorer.rs"]
pub mod r2wars;
#[path = "../sanity-check/scorer.rs"]
pub mod sanity_check;
pub mod scoring;

/// Every game a benchmark run can play.
pub const GAMES: [&dyn Game; 5] = [
    &crackme::Crackme,
    &fib_golf::FibGolf,
    &r2wars::GAMES[0],
    &r2wars::GAMES[1],
    &sanity_check::SanityCheck,
];

/// The architecture the tasks ask binaries for, whatever the host runs on.
pub const TASK_ARCHITECTURE: &str = "x86_64";

/// The user mode emulator running a task binary on a host of another architecture.
const EMULATOR: &str = "qemu-x86_64";

/// Where the libraries of the task architecture live on such a host, for the
/// emulator to load a dynamically linked binary from.
const EMULATOR_LIBRARIES: &str = "/usr/x86_64-linux-gnu";
const EMULATOR_LIBRARIES_OPTION: &str = "-L";

/// The command running the task binary at `binary`: the binary itself on a
/// host of the task architecture, the emulator over it anywhere else, so a
/// verdict does not depend on the host.
pub(crate) fn binary_command(binary: &std::path::Path) -> std::process::Command {
    if std::env::consts::ARCH == TASK_ARCHITECTURE {
        return std::process::Command::new(binary);
    }

    let mut command = std::process::Command::new(EMULATOR);
    if std::path::Path::new(EMULATOR_LIBRARIES).is_dir() {
        command.args([EMULATOR_LIBRARIES_OPTION, EMULATOR_LIBRARIES]);
    }
    command.arg(binary);
    command
}

/// The points scale every game ranks in.
///
/// An entry ranks within 0 and this maximum regardless of the game, which is
/// what makes runs of different games comparable. A game with nothing to rank
/// beyond passing ranks nothing.
pub const MAXIMUM_POINTS: u64 = 10_000;

/// The folder holding the task of a game with one turn.
pub const TASK_FOLDER: &str = "task";

/// One turn of a game: the task the seats play and the file it asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Turn {
    /// The folder under the game holding the task.
    pub task: &'static str,
    /// The file the task asks for, kept as the entry of every passing push.
    pub entry: &'static str,
}

/// The one turn of a game with a single task, asking for `entry`.
pub const fn single_turn(entry: &'static str) -> Turn {
    Turn {
        task: TASK_FOLDER,
        entry,
    }
}

/// An entry of an earlier turn a seat gets before playing a turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    /// The seat whose entry it is.
    pub seat: usize,
    /// The turn the entry was kept in.
    pub turn: usize,
    /// The name the entry appears under in the workspace and the inputs directory.
    pub name: String,
}

/// An entry a run kept, ranked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kept {
    pub path: std::path::PathBuf,
    pub points: Option<u64>,
}

/// One turn as a seat played it: the entry of record it kept, if any, and
/// every verdict its pushes got.
#[derive(Clone, Debug, Default)]
pub struct Played {
    pub entry: Option<Kept>,
    pub attempts: Vec<ava_wire::Attempt>,
}

/// How a pairing came out from the view of its first seat, with the reason
/// when the tally is not the outcome of play.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub tally: ava_wire::Tally,
    pub reason: Option<String>,
}

/// A benchmark task able to verify the submission an agent left and to rank
/// the entry it kept.
///
/// The verifier runs in the scoring container on every push and records a
/// fact. Ranking runs wherever standings are shown and reads the entry alone,
/// so it never executes anything and its knobs can change without a re-run.
pub trait Game {
    /// The name identifying the game on the command line and under the games directory.
    fn name(&self) -> &'static str;

    /// The folder under the games directory whose Dockerfile layers the software
    /// of the game over the harness image and the scorer image, if the game needs
    /// any beyond the base.
    fn image(&self) -> Option<&'static str> {
        None
    }

    /// The turns of the game in order, at least one.
    fn turns(&self) -> &[Turn];

    /// What a seat gets before playing `turn` from the `opponents` it meets:
    /// entries of earlier turns under the names the task text uses. Nothing
    /// unless the game says otherwise.
    fn inputs(&self, turn: usize, opponents: &[usize]) -> Vec<Input> {
        let _ = (turn, opponents);
        Vec::new()
    }

    /// Whether the contents of the `submission` directory do what the task of
    /// `turn` asks, with the inputs of the turn under `inputs` by name.
    fn verify(
        &self,
        turn: usize,
        submission: &std::path::Path,
        inputs: &std::path::Path,
    ) -> std::io::Result<ava_wire::Verdict>;

    /// The points a passing `entry` ranks at, within 0 and [`MAXIMUM_POINTS`],
    /// or nothing for a game with nothing to rank beyond passing.
    fn points(&self, entry: &std::path::Path) -> std::io::Result<Option<u64>> {
        let _ = entry;
        Ok(None)
    }

    /// How `first` came out against `second` over the turns both played,
    /// read from what was recorded, or nothing when it takes a fight. Unless
    /// the game says otherwise the entries of the last turn are compared by
    /// their points: more points win, equal points draw, a missing entry
    /// forfeits.
    fn outcome(&self, first: (usize, &[Played]), second: (usize, &[Played])) -> Option<Outcome> {
        let points = |played: &[Played]| {
            played
                .last()
                .and_then(|turn| turn.entry.as_ref())
                .map(|kept| kept.points)
        };

        Some(compared(
            first.0,
            points(first.1),
            second.0,
            points(second.1),
        ))
    }

    /// Fight the entry at `first` against the entry at `second` over `combats`
    /// combats and report the rounds from the view of `first`, for a game
    /// whose pairings take a fight.
    fn fight(
        &self,
        first: &std::path::Path,
        second: &std::path::Path,
        combats: u64,
    ) -> std::io::Result<ava_wire::Tally> {
        let _ = (first, second, combats);
        Err(std::io::Error::other(format!(
            "{} is not fought by the scorer",
            self.name()
        )))
    }
}

/// The outcome of comparing the entries of two seats by their points, either
/// of which may be missing. The points themselves are nothing for a game
/// ranking nothing, which draws.
pub fn compared(
    first: usize,
    first_points: Option<Option<u64>>,
    second: usize,
    second_points: Option<Option<u64>>,
) -> Outcome {
    match (first_points, second_points) {
        (Some(first_points), Some(second_points)) => {
            let ordering = first_points.cmp(&second_points);
            Outcome {
                tally: ava_wire::Tally {
                    won: u64::from(ordering.is_gt()),
                    drawn: u64::from(ordering.is_eq()),
                    lost: u64::from(ordering.is_lt()),
                },
                reason: None,
            }
        }
        (first_points, second_points) => forfeit(
            first,
            first_points.is_some(),
            second,
            second_points.is_some(),
        ),
    }
}

/// The reason of a pairing neither seat left an entry for.
const NEITHER_ENTRY: &str = "neither seat left a passing entry";

/// The outcome of a pairing at least one seat left no entry for: one round
/// to the seat that did, or no round at all.
pub fn forfeit(first: usize, first_present: bool, second: usize, second_present: bool) -> Outcome {
    let (tally, reason) = match (first_present, second_present) {
        (true, false) => (ava_wire::Tally::FIRST_WON, no_entry(second)),
        (false, true) => (ava_wire::Tally::SECOND_WON, no_entry(first)),
        _ => (ava_wire::Tally::default(), NEITHER_ENTRY.to_string()),
    };

    Outcome {
        tally,
        reason: Some(reason),
    }
}

/// The reason a seat forfeits, the seat counted from one the way the
/// tournament page counts.
pub fn no_entry(seat: usize) -> String {
    format!("seat {} left no passing entry", seat + 1)
}

/// Look up the game registered under `name`.
pub fn find(name: &str) -> Option<&'static dyn Game> {
    GAMES.into_iter().find(|game| game.name() == name)
}

/// The contents of the file at `path`, or nothing when it holds more than
/// `limit` bytes, read no further than that so an endless file cannot stall a
/// verification.
pub(crate) fn read_at_most(path: &std::path::Path, limit: u64) -> std::io::Result<Option<Vec<u8>>> {
    let file = std::fs::File::open(path)?;
    let mut contents = Vec::new();
    std::io::Read::read_to_end(&mut std::io::Read::take(file, limit + 1), &mut contents)?;

    Ok((contents.len() as u64 <= limit).then_some(contents))
}

/// The verdict on a submission failing the task, logged as it is reached.
pub(crate) fn failed(reason: String) -> ava_wire::Verdict {
    log::info!("{reason}");

    ava_wire::Verdict::failed(reason)
}

#[cfg(test)]
mod tests {
    #[test]
    fn points_compare_and_missing_entries_forfeit() {
        let won = super::compared(0, Some(Some(10)), 1, Some(Some(5)));
        assert_eq!(won.tally, ava_wire::Tally::FIRST_WON);
        assert_eq!(won.reason, None);

        let drawn = super::compared(0, Some(None), 1, Some(None));
        assert_eq!(drawn.tally.drawn, 1);

        let forfeited = super::compared(0, None, 1, Some(Some(5)));
        assert_eq!(forfeited.tally, ava_wire::Tally::SECOND_WON);
        assert_eq!(
            forfeited.reason.as_deref(),
            Some("seat 1 left no passing entry")
        );

        let neither = super::forfeit(0, false, 1, false);
        assert_eq!(neither.tally.rounds(), 0);
    }

    #[test]
    fn a_read_stops_at_the_limit() {
        let path = std::env::temp_dir().join(format!("ava-read-at-most-{}", std::process::id()));
        std::fs::write(&path, b"palindrome").unwrap();

        let whole = super::read_at_most(&path, 10).unwrap();
        let cut = super::read_at_most(&path, 9).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(whole.as_deref(), Some(&b"palindrome"[..]));
        assert_eq!(cut, None);
    }
}
