pub mod assembler;
pub mod board;
pub mod field;
pub mod machine;
pub mod playout;

const GAME_NAME: &str = "chess-vm";
const SUBMISSION_FILE: &str = "bot.cvm";

const SOURCE_LIMIT: u64 = 64 * 1024;

const FIGHT_SEED: u64 = 1;

const FIGHT_GAMES_PER_COMBAT: u64 = 2;

pub struct ChessVm;

const TURNS: [crate::Turn; 1] = [crate::single_turn(SUBMISSION_FILE)];

impl crate::Game for ChessVm {
    fn name(&self) -> &'static str {
        GAME_NAME
    }

    fn turns(&self) -> &[crate::Turn] {
        &TURNS
    }

    fn prepare(&self) -> std::io::Result<()> {
        let field = field::Field::load();

        log::info!(
            "the field has {} opponents with ratings {:.0} to {:.0}",
            field.anchors.len(),
            field.lowest,
            field.highest
        );

        Ok(())
    }

    fn verify(
        &self,
        _turn: usize,
        submission: &std::path::Path,
        _inputs: &std::path::Path,
    ) -> std::io::Result<ava_wire::Verdict> {
        let path = submission.join(SUBMISSION_FILE);

        let source = match read_source(&path) {
            Ok(source) => source,
            Err(reason) => return Ok(crate::failed(reason)),
        };

        let program = match assembler::assemble(&source, assembler::Author::Agent) {
            Ok(program) => program,
            Err(error) => return Ok(crate::failed(format!("{SUBMISSION_FILE} {error}"))),
        };

        log::info!(
            "{SUBMISSION_FILE} has {} instructions and {} data words, playing it against the field",
            program.code().len(),
            program.data().len()
        );

        let field = field::Field::load();
        let report = field::grade_submission(&program, &field);
        log_standings(&report);

        if !report.failures.is_empty() {
            return Ok(crate::failed(faulted(&report)));
        }

        Ok(passing_verdict(&report, &field))
    }

    /// The rating the push measured, as points: the best rated push is the
    /// entry of record.
    fn points(
        &self,
        _entry: &std::path::Path,
        verdict: &ava_wire::Verdict,
    ) -> std::io::Result<Option<u64>> {
        Ok(verdict
            .rating
            .map(|rating| rating.round().clamp(0.0, crate::MAXIMUM_POINTS as f64) as u64))
    }

    fn outcome(
        &self,
        _first: (usize, &[crate::Played]),
        _second: (usize, &[crate::Played]),
    ) -> Option<crate::Outcome> {
        None
    }

    fn fight(
        &self,
        first: &std::path::Path,
        second: &std::path::Path,
        combats: u64,
    ) -> std::io::Result<ava_wire::Tally> {
        let first_program = entry_program(first)?;
        let second_program = entry_program(second)?;

        let games: Vec<u64> = (0..combats * FIGHT_GAMES_PER_COMBAT).collect();
        let play = |game: &u64| -> playout::Outcome {
            let first_is_white = game.is_multiple_of(FIGHT_GAMES_PER_COMBAT);
            let (white, black) = if first_is_white {
                (&first_program, &second_program)
            } else {
                (&second_program, &first_program)
            };
            let played = playout::play(
                white,
                black,
                playout::derive_seed(FIGHT_SEED, *game),
                field::GRADING_PLY_CAP,
            );

            playout::outcome(if first_is_white {
                played.white_points
            } else {
                1.0 - played.white_points
            })
        };

        let mut tally = ava_wire::Tally::default();
        for outcome in playout::run_in_parallel(&games, &play) {
            match outcome {
                playout::Outcome::Won => tally.won += 1,
                playout::Outcome::Drawn => tally.drawn += 1,
                playout::Outcome::Lost => tally.lost += 1,
            }
        }

        Ok(tally)
    }
}

fn read_source(path: &std::path::Path) -> Result<String, String> {
    let metadata = std::fs::metadata(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => format!(
            "the submission has no {SUBMISSION_FILE}, its files are: {}",
            file_names(path.parent().unwrap_or(path))
        ),
        _ => format!("{SUBMISSION_FILE} cannot be read: {error}"),
    })?;

    if metadata.len() > SOURCE_LIMIT {
        return Err(format!(
            "{SUBMISSION_FILE} is {} bytes, the limit is {SOURCE_LIMIT} bytes",
            metadata.len()
        ));
    }

    std::fs::read_to_string(path)
        .map_err(|error| format!("{SUBMISSION_FILE} cannot be read: {error}"))
}

fn entry_program(entry: &std::path::Path) -> std::io::Result<assembler::Program> {
    let source = std::fs::read_to_string(entry)?;

    assembler::assemble(&source, assembler::Author::Agent)
        .map_err(|error| std::io::Error::other(format!("{}: {error}", entry.display())))
}

fn file_names(submission: &std::path::Path) -> String {
    let Ok(entries) = std::fs::read_dir(submission) else {
        return "unreadable".to_string();
    };

    let names: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();

    if names.is_empty() {
        return "none".to_string();
    }

    names.join(", ")
}

fn faulted(report: &field::Report) -> String {
    let failures = &report.failures;
    let cause = match &failures.first_message {
        Some(message) => format!("the first fault was: {message}"),
        None => "the cycle budget ran out before the program set a score".to_string(),
    };

    format!(
        "{SUBMISSION_FILE} failed {} decisions with a fault and {} by exceeding the cycle budget; the engine played the first legal move for each of them; {cause}",
        failures.faults, failures.overruns
    )
}

fn passing_verdict(report: &field::Report, field: &field::Field) -> ava_wire::Verdict {
    ava_wire::Verdict {
        passed: true,
        reason: Some(format!(
            "{SUBMISSION_FILE} has rating {:.0}, the field ranges from {:.0} to {:.0}",
            report.rating, field.lowest, field.highest
        )),
        defeated: Vec::new(),
        rating: Some(report.rating),
        measurements: std::collections::BTreeMap::new(),
    }
}

fn log_standings(report: &field::Report) {
    for standing in &report.standings {
        log::info!(
            "{:<16}{:>4} wins {:>4} draws {:>4} losses",
            standing.name,
            standing.wins,
            standing.draws,
            standing.losses
        );
    }

    log::info!(
        "{SUBMISSION_FILE} has rating {:.0}, {} faults, {} cycle budget overruns",
        report.rating,
        report.failures.faults,
        report.failures.overruns
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Game;

    const STARTER: &str = include_str!("task/bot.cvm");
    const REFERENCE: &str = include_str!("reference.cvm");

    fn submission(name: &str, contents: Option<&str>) -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "ava-chess-vm-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));

        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the temporary directory is writable");
        if let Some(contents) = contents {
            std::fs::write(directory.join(SUBMISSION_FILE), contents)
                .expect("the temporary directory is writable");
        }

        directory
    }

    fn failure_reason(name: &str, contents: Option<&str>) -> String {
        let directory = submission(name, contents);
        let verdict = ChessVm
            .verify(0, &directory, &directory)
            .expect("verify reads the submission");
        std::fs::remove_dir_all(&directory).expect("the temporary directory is removable");

        assert!(!verdict.passed);

        verdict.reason.expect("a failed verdict has a reason")
    }

    #[test]
    fn the_rating_of_the_push_ranks_the_entry() {
        let rated = |rating: Option<f64>| {
            let verdict = ava_wire::Verdict {
                rating,
                ..ava_wire::Verdict::passed()
            };
            ChessVm
                .points(std::path::Path::new(SUBMISSION_FILE), &verdict)
                .expect("points read the verdict")
        };

        assert_eq!(rated(Some(912.4)), Some(912));
        assert_eq!(rated(Some(-3.0)), Some(0));
        assert_eq!(rated(Some(20_000.0)), Some(crate::MAXIMUM_POINTS));
        assert_eq!(rated(None), None);
    }

    #[test]
    fn a_submission_without_a_program_fails() {
        let reason = failure_reason("empty", None);

        assert!(
            reason.starts_with(&format!("the submission has no {SUBMISSION_FILE}")),
            "{reason}"
        );
        assert!(reason.ends_with("its files are: none"), "{reason}");
    }

    #[test]
    fn a_program_that_does_not_assemble_fails() {
        let reason = failure_reason("broken", Some("    frobnicate r0\n    halt\n"));

        assert!(
            reason.starts_with(&format!("{SUBMISSION_FILE} line 1: unknown instruction")),
            "{reason}"
        );
    }

    #[test]
    fn a_program_over_the_size_limit_fails() {
        let long = "a".repeat(SOURCE_LIMIT as usize + 1);
        let reason = failure_reason("huge", Some(&long));

        assert!(reason.contains("the limit is"), "{reason}");
    }

    #[test]
    fn a_fight_plays_both_colours_and_tallies_for_the_first_entry() {
        let combats = 2;
        let heuristic = submission("fight-reference", Some(REFERENCE));
        let random = submission("fight-starter", Some(STARTER));

        let tally = ChessVm
            .fight(
                &heuristic.join(SUBMISSION_FILE),
                &random.join(SUBMISSION_FILE),
                combats,
            )
            .expect("both entries assemble");

        std::fs::remove_dir_all(&heuristic).expect("the temporary directory is removable");
        std::fs::remove_dir_all(&random).expect("the temporary directory is removable");

        assert_eq!(tally.rounds(), combats * FIGHT_GAMES_PER_COMBAT);
        assert!(
            tally.won > tally.lost,
            "the reference should beat the random mover, the tally is {tally:?}"
        );
    }

    #[test]
    #[ignore]
    fn the_reference_submission_passes_with_its_expected_rating() {
        let directory = submission("reference", Some(REFERENCE));

        let verdict = ChessVm
            .verify(0, &directory, &directory)
            .expect("verify reads the submission");
        std::fs::remove_dir_all(&directory).expect("the temporary directory is removable");

        assert!(verdict.passed, "{:?}", verdict.reason);

        let reason = verdict.reason.expect("a passing verdict has a reason");
        assert!(
            reason.starts_with(&format!("{SUBMISSION_FILE} has rating 9")),
            "unexpected rating: {reason}"
        );
    }
}
