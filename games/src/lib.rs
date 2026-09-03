//! The games scoring benchmark submissions.

#[path = "../fib-golf/scorer.rs"]
pub mod fib_golf;
#[path = "../sanity-check/scorer.rs"]
pub mod sanity_check;
pub mod scoring;

/// Every game a benchmark run can play.
pub const GAMES: [&dyn Game; 2] = [&fib_golf::FibGolf, &sanity_check::SanityCheck];

/// The points scale every game scores in.
///
/// A submission scores within 0 and this maximum regardless of the game,
/// which is what makes runs of different games comparable.
pub const MAXIMUM_POINTS: u64 = 10_000;

/// The verdict a game reaches over one submission.
#[derive(Debug, serde::Serialize)]
pub struct Score {
    /// The game which produced this score.
    pub game: &'static str,
    /// Whether the submission solves the task.
    pub solved: bool,
    /// The measure the game optimizes for, within 0 and [`MAXIMUM_POINTS`];
    /// higher is better.
    pub points: u64,
    /// Why the submission did not solve the task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A benchmark task able to score the submission an agent left.
pub trait Game {
    /// The name identifying the game on the command line and under the games directory.
    fn name(&self) -> &'static str;

    /// Score the contents of the `submission` directory.
    fn score(&self, submission: &std::path::Path) -> std::io::Result<Score>;
}

/// Look up the game registered under `name`.
pub fn find(name: &str) -> Option<&'static dyn Game> {
    GAMES.into_iter().find(|game| game.name() == name)
}
