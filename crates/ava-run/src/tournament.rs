//! Tournaments: a lobby of seats and the rounds they play.
//!
//! A round is every seat playing a run of every turn of the game, each turn
//! seeded with the entries of the other seats the game asks for, then every
//! pairing settled: fought in the scorer image where the game needs a fight,
//! read from the records otherwise. The record under the tournament folder
//! holds the facts, and the standings are derived from it wherever they are
//! shown.

use crate::docker;

/// Where the tournaments are kept, one folder each.
pub const TOURNAMENT_DIRECTORY: &str = "tournaments";

/// The record of a tournament in its folder.
pub const RECORD_FILE: &str = "tournament.json";

/// The console of the fights of one round, `round-<number>.log` in the folder.
const ROUND_LOG_PREFIX: &str = "round-";
const ROUND_LOG_SUFFIX: &str = ".log";

/// The marker the process playing a round leaves in the folder, holding its
/// pid, so another process knows the round is going on.
pub const PLAYING_FILE: &str = "playing";

/// The signal number that probes a process without signaling it.
const NO_SIGNAL: i32 = 0;

unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}

/// What separates the harness, the model and the thinking level of a seat on
/// the command line.
const SEAT_SEPARATOR: char = '/';
const SEAT_PARTS: usize = 3;

/// The characters a tournament name is made of, besides letters and digits.
const NAME_PUNCTUATION: [char; 3] = ['-', '_', '.'];

/// The combats every fight of a tournament plays unless chosen otherwise:
/// one combat is best of three rounds on random load positions, too few to
/// tell a win share from a coin flip.
pub const DEFAULT_COMBATS: u64 = 5;

/// `combats` as the combats a fight may play: at least one.
pub fn checked_combats(combats: u64) -> std::io::Result<u64> {
    if combats == 0 {
        return Err(std::io::Error::other("a fight plays at least one combat"));
    }

    Ok(combats)
}

/// The tournament command: create the named tournament when it does not
/// exist, seat the agents given, and play one round.
#[derive(Debug, Default)]
pub struct Tournament {
    /// The tournament, naming a folder under `tournaments`.
    pub name: String,
    /// The game, needed to create the tournament.
    pub game: String,
    /// The seats to add, each `harness/model` or `harness/model/thinking`.
    pub seats: Vec<String>,
    /// The seconds every run is given, taken when the tournament is created.
    pub limit: Option<u64>,
    /// The combats every fight plays, taken when the tournament is created.
    pub combats: Option<u64>,
    /// The agent analyzing every run, `harness/model` or
    /// `harness/model/thinking`, taken when the tournament is created.
    pub analyst: Option<String>,
    /// The seconds that analyst is given, taken when the tournament is created.
    pub analyst_seconds: Option<u64>,
    /// Whether the docker images are rebuilt instead of reused.
    pub force_build_images: bool,
    /// The most runs a round starts at once, all of them without a cap.
    pub parallel: Option<usize>,
}

/// Where a run sits in a tournament.
#[derive(Clone, Debug)]
pub struct Placement {
    pub tournament: String,
    /// The round, counted from zero.
    pub round: usize,
    /// The seat, counted from zero.
    pub seat: usize,
    /// The turn, counted from zero.
    pub turn: usize,
}

/// Serializes every change to the records, so two actions on one tournament
/// cannot lose each other's writes.
static RECORDS: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The turn the attacks of a record from before the turns count as.
const LEGACY_ATTACK_TURN: usize = 1;

/// The tournaments a round is being played in.
static PLAYING: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Marks a tournament as playing for as long as it lives: in this process,
/// and through the marker file for every other one.
struct Playing(String);

impl Playing {
    /// Mark the named tournament, under the records lock so no seat change
    /// slips in between the check and the mark.
    fn begin(name: &str) -> std::io::Result<Self> {
        let _records = RECORDS.lock().expect("the records lock is not poisoned");
        if playing(name) {
            return Err(std::io::Error::other(format!(
                "{name} is playing a round already"
            )));
        }

        std::fs::write(
            directory(name).join(PLAYING_FILE),
            std::process::id().to_string(),
        )?;
        PLAYING
            .lock()
            .expect("the playing list is not poisoned")
            .push(name.to_string());

        Ok(Self(name.to_string()))
    }
}

impl Drop for Playing {
    fn drop(&mut self) {
        PLAYING
            .lock()
            .expect("the playing list is not poisoned")
            .retain(|tournament| *tournament != self.0);
        let _ = std::fs::remove_file(directory(&self.0).join(PLAYING_FILE));
    }
}

/// Whether a round of the named tournament is being played, by this process
/// or by another one still alive. A marker left by a process that died reads
/// as a round that broke off.
pub fn playing(name: &str) -> bool {
    if PLAYING
        .lock()
        .expect("the playing list is not poisoned")
        .iter()
        .any(|tournament| tournament == name)
    {
        return true;
    }

    let Some(pid) = std::fs::read_to_string(directory(name).join(PLAYING_FILE))
        .ok()
        .and_then(|marker| marker.trim().parse::<i32>().ok())
    else {
        return false;
    };

    pid != std::process::id() as i32 && unsafe { kill(pid, NO_SIGNAL) } == 0
}

/// Run the tournament command.
pub fn run(command: &Tournament) -> std::io::Result<i32> {
    let name = command.name.as_str();
    checked_name(name)?;

    if directory(name).join(RECORD_FILE).is_file() {
        if !command.game.is_empty() {
            return Err(std::io::Error::other(format!(
                "{name} exists, its game is fixed"
            )));
        }
        if command.limit.is_some() {
            return Err(std::io::Error::other(format!(
                "{name} exists, its seconds are fixed"
            )));
        }
        if command.combats.is_some() {
            return Err(std::io::Error::other(format!(
                "{name} exists, its combats are fixed"
            )));
        }
        if command.analyst.is_some() || command.analyst_seconds.is_some() {
            return Err(std::io::Error::other(format!(
                "{name} exists, its analyst is fixed"
            )));
        }
    } else {
        if command.game.is_empty() {
            return Err(std::io::Error::other(format!(
                "{name} does not exist, pass a game with -g to create it"
            )));
        }
        let analyst = match &command.analyst {
            Some(analyst) => Some(parse_seat(analyst)?),
            None => None,
        };
        create(
            name,
            &command.game,
            command
                .limit
                .unwrap_or(docker::Agent::DEFAULT_LIMIT_SECONDS),
            command.combats.unwrap_or(DEFAULT_COMBATS),
            analyst,
            command
                .analyst_seconds
                .unwrap_or(docker::Analyst::DEFAULT_LIMIT_SECONDS),
        )?;
    }

    for seat in &command.seats {
        add_seat(name, &parse_seat(seat)?)?;
    }

    play_round(name, command.force_build_images, command.parallel)
}

/// The agent a `harness/model` or `harness/model/thinking` seat names.
fn parse_seat(seat: &str) -> std::io::Result<ava_wire::Agent> {
    let parts: Vec<&str> = seat.splitn(SEAT_PARTS, SEAT_SEPARATOR).collect();
    let (harness, model, thinking) = match parts.as_slice() {
        [harness, model] => (harness, model, None),
        [harness, model, thinking] => (harness, model, Some(thinking.to_string())),
        _ => {
            return Err(std::io::Error::other(format!(
                "`{seat}`: a seat is harness{SEAT_SEPARATOR}model or harness{SEAT_SEPARATOR}model{SEAT_SEPARATOR}thinking"
            )));
        }
    };

    if let Some(level) = &thinking
        && !crate::registry::THINKING_LEVELS.contains(&level.as_str())
    {
        return Err(std::io::Error::other(format!(
            "`{seat}`: unknown thinking level `{level}`, known are: {}",
            crate::registry::THINKING_LEVELS.join(", ")
        )));
    }

    Ok(ava_wire::Agent {
        harness: harness.to_string(),
        model: model.to_string(),
        thinking,
    })
}

/// The folder of the named tournament.
pub fn directory(name: &str) -> std::path::PathBuf {
    std::path::Path::new(TOURNAMENT_DIRECTORY).join(name)
}

/// Refuse a name that is not a plain folder name.
fn checked_name(name: &str) -> std::io::Result<()> {
    let plain = !name.is_empty()
        && !name.starts_with('.')
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || NAME_PUNCTUATION.contains(&character)
        });

    if !plain {
        return Err(std::io::Error::other(format!(
            "`{name}`: a tournament name is letters, digits, dashes, underscores and dots"
        )));
    }

    Ok(())
}

/// Create the named tournament of `game`, every run given `limit` seconds
/// and every fight playing `combats` combats.
pub fn create(
    name: &str,
    game: &str,
    limit: u64,
    combats: u64,
    analyst: Option<ava_wire::Agent>,
    analyst_seconds: u64,
) -> std::io::Result<ava_wire::Tournament> {
    checked_name(name)?;

    ava_game::find(game).ok_or_else(|| {
        crate::registry::unknown(game, "game", ava_game::GAMES.iter().map(|game| game.name()))
    })?;
    docker::Agent::checked_limit(limit)?;
    checked_combats(combats)?;
    docker::Analyst::checked_limit(analyst_seconds)?;

    let _records = RECORDS.lock().expect("the records lock is not poisoned");
    let folder = directory(name);
    if folder.join(RECORD_FILE).is_file() {
        return Err(std::io::Error::other(format!("{name} exists already")));
    }
    std::fs::create_dir_all(&folder)?;

    let record = ava_wire::Tournament {
        version: ava_wire::VERSION,
        name: name.to_string(),
        game: game.to_string(),
        game_version: docker::game_version(game),
        pairing: ava_wire::ROUND_ROBIN.to_string(),
        limit_seconds: limit,
        combats,
        analyst,
        analyst_seconds,
        created_seconds: crate::usage::epoch_now(),
        seats: Vec::new(),
        rounds: Vec::new(),
    };
    write(&record)?;
    log::info!("created the {name} tournament playing {game}");

    Ok(record)
}

/// Seat `agent` in the named tournament, which no round has fixed yet,
/// checking that the pairing can play.
pub fn add_seat(name: &str, agent: &ava_wire::Agent) -> std::io::Result<()> {
    crate::registry::load()?.invocation(
        &agent.harness,
        &agent.model,
        "",
        agent.thinking.as_deref(),
        crate::registry::Start::Task,
    )?;

    modify(name, |record| {
        if playing(name) {
            return Err(std::io::Error::other(format!("{name} is playing a round")));
        }
        if record.played() {
            return Err(std::io::Error::other(format!(
                "{name} played a round, its seats are fixed"
            )));
        }
        record.seats.push(agent.clone());
        log::info!("{name}: seat {} is {}", record.seats.len(), agent.label());
        Ok(())
    })
}

/// Remove the seat at `seat` from the named tournament, which no round has fixed yet.
pub fn remove_seat(name: &str, seat: usize) -> std::io::Result<()> {
    modify(name, |record| {
        if playing(name) {
            return Err(std::io::Error::other(format!("{name} is playing a round")));
        }
        if record.played() {
            return Err(std::io::Error::other(format!(
                "{name} played a round, its seats are fixed"
            )));
        }
        if seat >= record.seats.len() {
            return Err(std::io::Error::other(format!(
                "{name} has no seat {}",
                seat + 1
            )));
        }
        record.seats.remove(seat);
        Ok(())
    })
}

/// The record of the named tournament.
pub fn load(name: &str) -> std::io::Result<ava_wire::Tournament> {
    checked_name(name)?;
    let path = directory(name).join(RECORD_FILE);
    let contents = std::fs::read_to_string(&path)
        .map_err(|error| std::io::Error::other(format!("{name}: {error}")))?;

    serde_json::from_str(&contents)
        .map_err(|error| std::io::Error::other(format!("{}: {error}", path.display())))
}

/// Every tournament on disk, newest first.
pub fn list() -> std::io::Result<Vec<ava_wire::Tournament>> {
    let folders = match std::fs::read_dir(TOURNAMENT_DIRECTORY) {
        Ok(folders) => folders,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(std::io::Error::new(
                error.kind(),
                format!("{TOURNAMENT_DIRECTORY}: {error}"),
            ));
        }
    };

    let mut tournaments: Vec<ava_wire::Tournament> = folders
        .filter_map(Result::ok)
        .filter_map(|folder| folder.file_name().into_string().ok())
        .filter_map(|name| load(&name).ok())
        .collect();
    tournaments.sort_by_key(|tournament| std::cmp::Reverse(tournament.created_seconds));

    Ok(tournaments)
}

/// Where every run a tournament placed sits, by run name.
pub fn placements() -> std::io::Result<std::collections::HashMap<String, Placement>> {
    let mut placements = std::collections::HashMap::new();

    for tournament in list()? {
        for (round, played) in tournament.rounds.iter().enumerate() {
            for entry in &played.entries {
                placements.insert(
                    entry.run.clone(),
                    Placement {
                        tournament: tournament.name.clone(),
                        round,
                        seat: entry.seat,
                        turn: entry.turn,
                    },
                );
            }
            // The attacks of a record from before the turns played the second turn.
            for pairing in &played.pairings {
                if let Some(run) = &pairing.run {
                    placements.insert(
                        run.clone(),
                        Placement {
                            tournament: tournament.name.clone(),
                            round,
                            seat: pairing.first,
                            turn: LEGACY_ATTACK_TURN,
                        },
                    );
                }
            }
        }
    }

    Ok(placements)
}

/// The pairings of `round` as the standings see them: the ones recorded,
/// the fights and the attacks of records from before the turns, and for every
/// other pair of seats what the game reads out of the records, derived when
/// asked so a changed curve changes who won.
pub fn pairings(
    record: &ava_wire::Tournament,
    round: &ava_wire::Round,
) -> std::io::Result<Vec<ava_wire::Pairing>> {
    let game = find(&record.game)?;
    let seats = record.seats.len();
    let played = played_round(game, round, seats)?;
    let seconds = round.finished_seconds.unwrap_or(round.started_seconds);

    let mut pairings = Vec::new();
    for (first, second) in ava_game::scoring::round_robin(seats) {
        let recorded: Vec<ava_wire::Pairing> = round
            .pairings
            .iter()
            .filter(|pairing| {
                (pairing.first, pairing.second) == (first, second)
                    || (pairing.first, pairing.second) == (second, first)
            })
            .cloned()
            .collect();
        if !recorded.is_empty() {
            pairings.extend(recorded);
            continue;
        }

        if let Some(outcome) = game.outcome((first, &played[first]), (second, &played[second])) {
            pairings.push(ava_wire::Pairing {
                first,
                second,
                seconds,
                tally: outcome.tally,
                reason: outcome.reason,
                run: None,
            });
        }
    }

    Ok(pairings)
}

/// The turns of `round` as every seat played them, by seat.
fn played_round(
    game: &dyn ava_game::Game,
    round: &ava_wire::Round,
    seats: usize,
) -> std::io::Result<Vec<Vec<ava_game::Played>>> {
    (0..seats)
        .map(|seat| played_turns(game, round, seat))
        .collect()
}

/// The turns of `round` as `seat` played them: per turn the entry of record it
/// kept and the verdicts of its pushes, or nothing for a turn it did not play.
fn played_turns(
    game: &dyn ava_game::Game,
    round: &ava_wire::Round,
    seat: usize,
) -> std::io::Result<Vec<ava_game::Played>> {
    let mut played = Vec::new();
    for (turn, task) in game.turns().iter().enumerate() {
        let Some(entry) = round
            .entries
            .iter()
            .find(|entry| entry.seat == seat && entry.turn == turn)
        else {
            played.push(ava_game::Played::default());
            continue;
        };
        let directory = std::path::Path::new(docker::RUN_DIRECTORY).join(&entry.run);
        let attempts = crate::runs::read(&directory)
            .map(|run| run.attempts)
            .unwrap_or_default();
        let kept = match entry.attempt {
            Some(attempt) => crate::runs::entries(game, &directory, task.entry)?
                .into_iter()
                .find(|kept| kept.seconds == attempt)
                .map(|kept| ava_game::Kept {
                    path: kept.path,
                    points: kept.points,
                }),
            None => None,
        };
        played.push(ava_game::Played {
            entry: kept,
            attempts,
        });
    }

    Ok(played)
}

/// The matches of the finished rounds of `record`, in play order.
pub fn matches(record: &ava_wire::Tournament) -> std::io::Result<Vec<ava_game::scoring::Match>> {
    let mut played = Vec::new();
    for round in record.finished_rounds() {
        played.extend(pairings(record, round)?);
    }

    Ok(ava_game::scoring::matches(&record.seats, &played))
}

/// The game of a tournament, or the error naming the known ones.
fn find(game: &str) -> std::io::Result<&'static dyn ava_game::Game> {
    ava_game::find(game).ok_or_else(|| {
        crate::registry::unknown(game, "game", ava_game::GAMES.iter().map(|game| game.name()))
    })
}

/// Write `record` as the record of its tournament.
fn write(record: &ava_wire::Tournament) -> std::io::Result<()> {
    std::fs::write(
        directory(&record.name).join(RECORD_FILE),
        format!(
            "{}\n",
            serde_json::to_string_pretty(record).map_err(std::io::Error::other)?
        ),
    )
}

/// Change the record of the named tournament under the records lock.
fn modify(
    name: &str,
    change: impl FnOnce(&mut ava_wire::Tournament) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let _records = RECORDS.lock().expect("the records lock is not poisoned");
    let mut record = load(name)?;
    change(&mut record)?;
    write(&record)
}

/// Play one round of the named tournament: turn by turn a run per seat, all
/// at once, each seeded with the entries of the other seats the game asks
/// for, then the pairings settled.
///
/// The round is written the moment the runs of a turn are named, so the
/// record links the runs while they play, and again after every entry of
/// record and every fight, so a round that breaks off leaves what it had.
/// Only a finished round counts for the standings.
pub fn play_round(
    name: &str,
    force_build_images: bool,
    parallel: Option<usize>,
) -> std::io::Result<i32> {
    let _playing = Playing::begin(name)?;
    let record = load(name)?;
    if record.seats.is_empty() {
        return Err(std::io::Error::other(format!("{name} has no seats")));
    }
    let game = find(&record.game)?;
    let seats = record.seats.len();
    let round = record.rounds.len() + 1;

    modify(name, |record| {
        record.rounds.push(ava_wire::Round {
            started_seconds: crate::usage::epoch_now(),
            finished_seconds: None,
            entries: Vec::new(),
            pairings: Vec::new(),
        });
        Ok(())
    })?;
    log::info!(
        "{name}: round {round} starts, {seats} seats play {} over {} turns",
        record.game,
        game.turns().len()
    );

    let analyses = Analyses::new(
        name,
        record.analyst.clone(),
        record.analyst_seconds,
        parallel,
    );
    let mut code = 0;

    for (turn, task) in game.turns().iter().enumerate() {
        if crate::interrupt::interrupted() {
            break;
        }

        // Every run is resolved before the turn is written, so the record only
        // ever names runs that are about to start.
        let played = load(name)?;
        let played = played.rounds.last().expect("the round was written");
        let mut launches = Vec::new();
        for (seat, agent) in record.seats.iter().enumerate() {
            let opponents: Vec<usize> = (0..seats).filter(|other| *other != seat).collect();
            launches.push(docker::prepare(&docker::Agent {
                name: agent.harness.clone(),
                model: agent.model.clone(),
                game: record.game.clone(),
                limit: record.limit_seconds,
                parallel: 1,
                thinking: agent.thinking.clone(),
                force_build_images,
                analyst: None,
                turn,
                inputs: resolve_inputs(name, game, played, &game.inputs(turn, &opponents)),
            })?);
        }
        let runs: Vec<String> = record
            .seats
            .iter()
            .map(|seat| docker::run_name(&seat.harness))
            .collect();

        modify(name, |record| {
            let played = record.rounds.last_mut().expect("the round was written");
            played
                .entries
                .extend(runs.iter().enumerate().map(|(seat, run)| ava_wire::Entry {
                    seat,
                    turn,
                    run: run.clone(),
                    attempt: None,
                }));
            Ok(())
        })?;
        log::info!(
            "{name}: round {round}, turn {} of {}: {seats} seats play {}",
            turn + 1,
            game.turns().len(),
            task.task
        );

        let outcomes = bounded(runs.len(), parallel, |index| {
            let outcome = docker::play(&launches[index], &runs[index]);
            analyses.start(&runs[index]);
            outcome
        });
        for (run, outcome) in runs.iter().zip(outcomes) {
            match outcome {
                Ok(finished) if code == 0 => code = finished,
                Ok(_) => {}
                Err(error) => {
                    log::error!("{name}: the run {run} failed: {error}");
                    code = 1;
                }
            }
        }

        let kept: Vec<Option<u64>> = runs
            .iter()
            .map(|run| {
                crate::runs::entry_of_record(
                    game,
                    &std::path::Path::new(docker::RUN_DIRECTORY).join(run),
                    task.entry,
                )
                .unwrap_or_else(|error| {
                    log::warn!("{name}: the entries of {run} cannot be read: {error}");
                    None
                })
                .map(|kept| kept.seconds)
            })
            .collect();
        modify(name, |record| {
            let played = record.rounds.last_mut().expect("the round was written");
            for entry in played.entries.iter_mut().filter(|entry| entry.turn == turn) {
                entry.attempt = kept[entry.seat];
            }
            Ok(())
        })?;
    }

    if !crate::interrupt::interrupted() {
        settle(name, &record, round, game, &mut code)?;
    }

    // An interrupted round stays unfinished, so its forfeits never reach the
    // standings.
    if crate::interrupt::interrupted() {
        log::warn!("{name}: round {round} was interrupted and stays unfinished");
        analyses.finish();
        return Ok(code.max(1));
    }

    modify(name, |record| {
        record
            .rounds
            .last_mut()
            .expect("the round was written")
            .finished_seconds = Some(crate::usage::epoch_now());
        Ok(())
    })?;
    log::info!("{name}: round {round} is over");
    analyses.finish();

    Ok(code)
}

/// The files behind the `inputs` a turn asks for: the entries of record the
/// earlier turns of `round` kept. An input whose seat kept nothing is left
/// out, and the verifier of the turn says what that means.
fn resolve_inputs(
    name: &str,
    game: &dyn ava_game::Game,
    round: &ava_wire::Round,
    inputs: &[ava_game::Input],
) -> Vec<docker::InputFile> {
    inputs
        .iter()
        .filter_map(|input| {
            let entry = round
                .entries
                .iter()
                .find(|entry| entry.seat == input.seat && entry.turn == input.turn)?;
            let attempt = entry.attempt?;
            let path = std::path::Path::new(docker::RUN_DIRECTORY)
                .join(&entry.run)
                .join(docker::ENTRIES_DIRECTORY)
                .join(attempt.to_string())
                .join(crate::runs::turn_entry(game, input.turn));
            if !path.is_file() {
                log::warn!(
                    "{name}: the entry of seat {} from turn {} is not at {}",
                    input.seat + 1,
                    input.turn + 1,
                    path.display()
                );
                return None;
            }

            Some(docker::InputFile {
                path,
                record: ava_wire::Input {
                    run: entry.run.clone(),
                    attempt,
                    name: input.name.clone(),
                },
            })
        })
        .collect()
}

/// Settle the pairings of the round the game cannot read from the records:
/// every such pair of seats fights in the scorer image, one fight after the
/// other, each recorded as it ends. A seat without an entry of the last turn
/// forfeits its fights.
fn settle(
    name: &str,
    record: &ava_wire::Tournament,
    round: usize,
    game: &dyn ava_game::Game,
    code: &mut i32,
) -> std::io::Result<()> {
    let current = load(name)?;
    let played = played_round(
        game,
        current.rounds.last().expect("the round was written"),
        record.seats.len(),
    )?;
    let console = directory(name).join(format!("{ROUND_LOG_PREFIX}{round}{ROUND_LOG_SUFFIX}"));

    for (first, second) in ava_game::scoring::round_robin(record.seats.len()) {
        if game
            .outcome((first, &played[first]), (second, &played[second]))
            .is_some()
        {
            continue;
        }
        if crate::interrupt::interrupted() {
            log::warn!("{name}: round {round} was interrupted before every pairing fought");
            *code = 1;
            return Ok(());
        }

        let entry = |seat: usize| played[seat].last().and_then(|turn| turn.entry.as_ref());
        let (tally, reason) = match (entry(first), entry(second)) {
            (Some(kept_first), Some(kept_second)) => {
                log::info!("{name}: seat {} fights seat {}", first + 1, second + 1);
                match docker::fight(
                    &record.game,
                    &kept_first.path,
                    &kept_second.path,
                    record.combats,
                    &console,
                ) {
                    Ok(tally) => (tally, None),
                    Err(error) => (ava_wire::Tally::default(), Some(error.to_string())),
                }
            }
            (kept_first, kept_second) => {
                let forfeited =
                    ava_game::forfeit(first, kept_first.is_some(), second, kept_second.is_some());
                (forfeited.tally, forfeited.reason)
            }
        };
        log::info!(
            "{name}: seat {} against seat {}: {} won, {} drawn, {} lost{}",
            first + 1,
            second + 1,
            tally.won,
            tally.drawn,
            tally.lost,
            reason
                .as_deref()
                .map(|reason| format!(", {reason}"))
                .unwrap_or_default()
        );
        record_pairing(
            name,
            ava_wire::Pairing {
                first,
                second,
                seconds: crate::usage::epoch_now(),
                tally,
                reason,
                run: None,
            },
        )?;
    }

    Ok(())
}

/// The analyses of a round: every run is analyzed the moment it is over, in
/// parallel with whatever the round still plays, under the same cap as the
/// runs. A failed analysis is logged and fails nothing, the run page offers
/// it again.
struct Analyses {
    name: String,
    analyst: Option<ava_wire::Agent>,
    /// The seconds every analysis is given.
    seconds: u64,
    /// The queue the capped workers take runs from.
    queue: Option<std::sync::Mutex<std::sync::mpsc::Sender<String>>>,
    handles: std::sync::Mutex<Vec<std::thread::JoinHandle<()>>>,
}

impl Analyses {
    /// Ready to analyze the runs of the named tournament with `analyst`, at
    /// most `parallel` at a time, or as many as end without a cap.
    fn new(
        name: &str,
        analyst: Option<ava_wire::Agent>,
        seconds: u64,
        parallel: Option<usize>,
    ) -> Self {
        let mut analyses = Self {
            name: name.to_string(),
            analyst,
            seconds,
            queue: None,
            handles: std::sync::Mutex::new(Vec::new()),
        };

        if let (Some(analyst), Some(workers)) = (&analyses.analyst, parallel) {
            let (sender, receiver) = std::sync::mpsc::channel::<String>();
            let receiver = std::sync::Arc::new(std::sync::Mutex::new(receiver));
            let mut handles = Vec::new();
            for _ in 0..workers.max(1) {
                let receiver = receiver.clone();
                let name = analyses.name.clone();
                let analyst = analyst.clone();
                let seconds = analyses.seconds;
                handles.push(std::thread::spawn(move || {
                    loop {
                        let next = receiver.lock().expect("the queue is not poisoned").recv();
                        let Ok(run) = next else {
                            return;
                        };
                        analyze(&name, &analyst, seconds, &run);
                    }
                }));
            }
            analyses.queue = Some(std::sync::Mutex::new(sender));
            analyses.handles = std::sync::Mutex::new(handles);
        }

        analyses
    }

    /// Analyze `run`, unless the tournament has no analyst.
    fn start(&self, run: &str) {
        let Some(analyst) = &self.analyst else {
            return;
        };

        if let Some(queue) = &self.queue {
            let _ = queue
                .lock()
                .expect("the queue is not poisoned")
                .send(run.to_string());
            return;
        }

        let name = self.name.clone();
        let analyst = analyst.clone();
        let seconds = self.seconds;
        let run = run.to_string();
        self.handles
            .lock()
            .expect("the handles are not poisoned")
            .push(std::thread::spawn(move || {
                analyze(&name, &analyst, seconds, &run)
            }));
    }

    /// Wait for every analysis started.
    fn finish(self) {
        drop(self.queue);
        for handle in self
            .handles
            .into_inner()
            .expect("the handles are not poisoned")
        {
            let _ = handle.join();
        }
    }
}

/// Analyze `run` of the named tournament with `analyst`, logging a failure.
fn analyze(name: &str, analyst: &ava_wire::Agent, seconds: u64, run: &str) {
    log::info!("{name}: analyzing {run} with {}", analyst.label());
    let outcome = docker::analyze(&docker::Analyze {
        run: run.to_string(),
        analyst: docker::Analyst {
            name: analyst.harness.clone(),
            model: analyst.model.clone(),
            thinking: analyst.thinking.clone(),
            limit: seconds,
        },
    });
    if let Err(error) = outcome {
        log::error!("{name}: the analysis of {run} failed: {error}");
    }
}

/// Play `runs` with at most `parallel` sandboxes at a time, or all at once
/// without a cap, calling `finished` on each run as it ends. The outcomes come
/// back in the order the runs were given.
fn bounded(
    count: usize,
    parallel: Option<usize>,
    job: impl Fn(usize) -> std::io::Result<i32> + Sync,
) -> Vec<std::io::Result<i32>> {
    let workers = parallel.unwrap_or(count).clamp(1, count.max(1));
    let queue = std::sync::Mutex::new((0..count).collect::<std::collections::VecDeque<_>>());
    let outcomes = std::sync::Mutex::new(Vec::new());

    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let next = queue.lock().expect("the queue is not poisoned").pop_front();
                    let Some(index) = next else {
                        return;
                    };
                    let outcome = job(index);
                    outcomes
                        .lock()
                        .expect("the outcomes are not poisoned")
                        .push((index, outcome));
                }
            });
        }
    });

    let mut outcomes = outcomes
        .into_inner()
        .expect("the outcomes are not poisoned");
    outcomes.sort_by_key(|(index, _)| *index);
    outcomes.into_iter().map(|(_, outcome)| outcome).collect()
}

/// Record `pairing` on the last round of the named tournament.
fn record_pairing(name: &str, pairing: ava_wire::Pairing) -> std::io::Result<()> {
    modify(name, |record| {
        record
            .rounds
            .last_mut()
            .expect("the round was written")
            .pairings
            .push(pairing);
        Ok(())
    })
}
