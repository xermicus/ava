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

/// Who an entry is up against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The entry stands alone against the ceiling of the game.
    SinglePlayer,
    /// The entries of the seats of a tournament meet: fought by the scorer,
    /// or attacked by the other agents when the game has an attack task.
    Multiplayer,
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

    /// The file the task asks for, kept as the entry of every passing attempt.
    fn entry(&self) -> &'static str;

    /// Who an entry of the game is up against.
    fn mode(&self) -> Mode {
        Mode::SinglePlayer
    }

    /// The file the attack task asks for, kept as the entry of a passing
    /// attack, for a multiplayer game whose entries agents attack.
    fn attack_entry(&self) -> &'static str {
        self.entry()
    }

    /// Whether the contents of the `submission` directory do what the task asks.
    ///
    /// An attack on the entry of another run gets that entry as the file under
    /// `challenge`, the way it was mounted into the scoring container.
    fn verify(
        &self,
        submission: &std::path::Path,
        challenge: Option<&std::path::Path>,
    ) -> std::io::Result<ava_wire::Verdict>;

    /// The points a passing `entry` ranks at, within 0 and [`MAXIMUM_POINTS`],
    /// or nothing for a game with nothing to rank beyond passing.
    fn points(&self, entry: &std::path::Path) -> std::io::Result<Option<u64>> {
        let _ = entry;
        Ok(None)
    }

    /// Fight the entry at `first` against the entry at `second` over `combats`
    /// combats and report the rounds from the view of `first`, for a
    /// multiplayer game the scorer fights.
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
