//! Tournaments: a lobby of seats and the rounds they play.
//!
//! A round is every seat playing a run of every turn of the game, each turn
//! seeded with the entries of the other seats the game asks for, then every
//! pairing settled: fought in the scorer image where the game needs a fight,
//! read from the records otherwise. A swiss round pairs each seat once and
//! plays a turn facing other seats once per pair and direction. The record
//! under the tournament folder holds the facts, and the standings are
//! derived from it wherever they are shown.

use crate::docker;

/// Where the tournaments are kept, one folder each.
pub const TOURNAMENT_DIRECTORY: &str = "tournaments";

/// The record of a tournament in its folder.
pub const RECORD_FILE: &str = "tournament.json";

/// The name a record is staged under before it replaces the record.
const RECORD_STAGING_FILE: &str = "tournament.json.tmp";

/// The console of the fights of one round, `round-<number>.log` in the folder.
const ROUND_LOG_PREFIX: &str = "round-";
const ROUND_LOG_SUFFIX: &str = ".log";

/// The marker the process playing a round leaves in the folder, holding its
/// pid and the rounds it plays, so another process knows the round is going
/// on and which one it is. A marker from before the rounds were named holds
/// the pid alone.
pub const PLAYING_FILE: &str = "playing";

/// What separates the rounds of the playing marker.
const ROUND_SEPARATOR: char = ',';

/// A seat on the command line.
const SEAT_SHAPE: &str = "agent[/thinking], the agent a name of the registry or harness/model";

/// The characters a tournament name is made of, besides letters and digits.
const NAME_PUNCTUATION: [char; 3] = ['-', '_', '.'];

/// The combats every fight of a tournament plays unless chosen otherwise:
/// one combat is best of three rounds on random load positions, too few to
/// tell a win share from a coin flip.
pub const DEFAULT_COMBATS: u64 = 5;

/// The runs a round starts at once unless a count is chosen.
pub const DEFAULT_PARALLEL: usize = 8;

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
    /// The seats to add, each `agent[/thinking]`.
    pub seats: Vec<String>,
    /// The seconds every run is given, taken when the tournament is created.
    pub limit: Option<u64>,
    /// The combats every fight plays, taken when the tournament is created.
    pub combats: Option<u64>,
    /// One of [`ava_wire::PAIRINGS`], taken at creation, else [`default_pairing`].
    pub pairing: Option<String>,
    /// The agent analyzing every run, `agent[/thinking]`, taken when the
    /// tournament is created.
    pub analyst: Option<String>,
    /// The seconds that analyst is given, taken when the tournament is created.
    pub analyst_seconds: Option<u64>,
    /// Whether the docker images are rebuilt instead of reused.
    pub force_build_images: bool,
    /// The most runs a round starts at once, [`DEFAULT_PARALLEL`] without one.
    pub parallel: Option<usize>,
    /// Whether the rounds the seats joined after are backfilled instead of a
    /// round being played.
    pub backfill: bool,
    /// The round to resume and the way to resume it, instead of a round being
    /// played, the round counted from zero.
    pub resume: Option<(usize, Resume)>,
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

/// The turn a game of one turn plays, the only turn a seat joining a
/// tournament that has played rounds backfills.
const ONLY_TURN: usize = 0;

const LATE_JOIN_REFUSAL: &str = "a multi turn round robin takes no seats after its first round";

/// Any seat, to ask a turn whether it takes inputs from opponents.
const PROBED_OPPONENT: usize = 1;

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

    /// Name the rounds this play took in the marker, which is what tells a
    /// round in flight from one that broke off before it.
    fn plays_rounds(&self, rounds: &[usize]) -> std::io::Result<()> {
        let rounds: Vec<String> = rounds.iter().map(usize::to_string).collect();

        std::fs::write(
            directory(&self.0).join(PLAYING_FILE),
            format!(
                "{} {}",
                std::process::id(),
                rounds.join(&ROUND_SEPARATOR.to_string())
            ),
        )
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

    let Some(pid) = marker(name).and_then(|marker| {
        marker
            .split_whitespace()
            .next()
            .and_then(|pid| pid.parse::<i32>().ok())
    }) else {
        return false;
    };

    pid != std::process::id() as i32 && crate::process::alive(pid)
}

/// What the marker of the named tournament holds, if it is there.
fn marker(name: &str) -> Option<String> {
    std::fs::read_to_string(directory(name).join(PLAYING_FILE)).ok()
}

/// The rounds the play going on in the named tournament took, counted from
/// zero, empty when nothing plays or the marker names none.
pub fn playing_rounds(name: &str) -> Vec<usize> {
    marker(name)
        .map(|marker| marked_rounds(&marker))
        .unwrap_or_default()
}

/// The rounds a playing marker names after its pid, none for a marker from
/// before they were named.
fn marked_rounds(marker: &str) -> Vec<usize> {
    let Some(rounds) = marker.split_whitespace().nth(1) else {
        return Vec::new();
    };

    rounds
        .split(ROUND_SEPARATOR)
        .filter_map(|round| round.parse::<usize>().ok())
        .collect()
}

/// Run the tournament command.
pub fn run(command: &Tournament) -> std::io::Result<i32> {
    let name = command.name.as_str();
    checked_name(name)?;
    let registry = crate::registry::load()?;
    let seats = command
        .seats
        .iter()
        .map(|seat| parse_seat(&registry, seat))
        .collect::<std::io::Result<Vec<ava_wire::Setup>>>()?;

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
        if command.pairing.is_some() {
            return Err(std::io::Error::other(format!(
                "{name} exists, its pairing is fixed"
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
            Some(analyst) => Some(parse_seat(&registry, analyst)?),
            None => None,
        };
        create(
            name,
            &command.game,
            command
                .limit
                .unwrap_or(docker::Agent::DEFAULT_LIMIT_SECONDS),
            command.combats.unwrap_or(DEFAULT_COMBATS),
            command.pairing.as_deref(),
            analyst,
            command
                .analyst_seconds
                .unwrap_or(docker::Analyst::DEFAULT_LIMIT_SECONDS),
        )?;
    }

    for seat in &seats {
        add_seat(name, seat)?;
    }

    if command.backfill {
        return backfill(name, command.force_build_images, command.parallel);
    }

    if let Some((index, resume)) = command.resume {
        return resume_round(
            name,
            index,
            resume,
            command.force_build_images,
            command.parallel,
        );
    }

    play_round(name, command.force_build_images, command.parallel)
}

/// The setup an `agent[/thinking]` seat names in `registry`.
fn parse_seat(
    registry: &crate::registry::Registry,
    seat: &str,
) -> std::io::Result<ava_wire::Setup> {
    let level = |part: &str| crate::registry::THINKING_LEVELS.contains(&part);
    let parts: Vec<&str> = seat.split(crate::registry::PAIRING_SEPARATOR).collect();
    let (name, model, thinking) = match parts.as_slice() {
        [name] => (name, None, None),
        [name, thinking] if level(thinking) || registry.alias(name).is_some() => {
            (name, None, Some(thinking))
        }
        [harness, model] => (harness, Some(model), None),
        [harness, model, thinking] => (harness, Some(model), Some(thinking)),
        _ => {
            return Err(std::io::Error::other(format!(
                "`{seat}`: a seat is {SEAT_SHAPE}"
            )));
        }
    };

    registry
        .setup(name, model.copied(), thinking.copied())
        .map_err(|error| std::io::Error::other(format!("`{seat}`: {error}")))
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

/// Whether `turn` of `game` faces the entries of other seats.
pub fn plays_against(game: &dyn ava_game::Game, turn: usize) -> bool {
    !game.inputs(turn, &[PROBED_OPPONENT]).is_empty()
}

/// Swiss for a game with a turn facing other seats, round robin otherwise.
pub fn default_pairing(game: &dyn ava_game::Game) -> &'static str {
    if (0..game.turns().len()).any(|turn| plays_against(game, turn)) {
        ava_wire::SWISS
    } else {
        ava_wire::ROUND_ROBIN
    }
}

/// Every pairing scheme, for the command line.
pub const PAIRINGS: [&str; 2] = ava_wire::PAIRINGS;

/// `pairing` as a known scheme.
pub fn checked_pairing(pairing: &str) -> std::io::Result<&'static str> {
    ava_wire::PAIRINGS
        .into_iter()
        .find(|known| *known == pairing)
        .ok_or_else(|| crate::registry::unknown(pairing, "pairing", ava_wire::PAIRINGS.into_iter()))
}

/// Create the named tournament of `game`, every run given `limit` seconds,
/// every fight playing `combats` combats, paired by `pairing` or by default.
pub fn create(
    name: &str,
    game: &str,
    limit: u64,
    combats: u64,
    pairing: Option<&str>,
    analyst: Option<ava_wire::Setup>,
    analyst_seconds: u64,
) -> std::io::Result<ava_wire::Tournament> {
    checked_name(name)?;

    let found = find(game)?;
    let pairing = match pairing {
        Some(pairing) => checked_pairing(pairing)?,
        None => default_pairing(found),
    };
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
        pairing: pairing.to_string(),
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

/// Whether a seat can join `record` now. A round robin of several turns takes
/// none after its first round, since later turns face every seat at once.
pub fn joins_late(record: &ava_wire::Tournament) -> std::io::Result<bool> {
    Ok(!record.played() || record.swiss() || find(&record.game)?.turns().len() == 1)
}

/// Seat `setup` in the named tournament, checking that it can play. A seat
/// joining after a round was played backfills the rounds it missed.
pub fn add_seat(name: &str, setup: &ava_wire::Setup) -> std::io::Result<()> {
    crate::registry::load()?.invocation(setup, "", crate::registry::Start::Task)?;

    modify(name, |record| {
        if playing(name) {
            return Err(std::io::Error::other(format!("{name} is playing a round")));
        }
        if !joins_late(record)? {
            return Err(std::io::Error::other(format!(
                "{name}: {LATE_JOIN_REFUSAL}"
            )));
        }
        record.seats.push(setup.clone());
        log::info!("{name}: seat {} is {}", record.seats.len(), setup.label());
        Ok(())
    })
}

/// Whether a round of `record` holds the seat at `seat`, which is what fixes
/// it in the lobby: the rounds reference a seat by its number, so one a round
/// recorded cannot leave without rewriting what the round says.
pub fn seat_is_held(record: &ava_wire::Tournament, seat: usize) -> bool {
    record.rounds.iter().any(|round| {
        seats_of(round).contains(&seat)
            || round
                .pairings
                .iter()
                .any(|pairing| pairing.first == seat || pairing.second == seat)
    })
}

/// Remove the seat at `seat` from the named tournament, which no round holds yet.
pub fn remove_seat(name: &str, seat: usize) -> std::io::Result<()> {
    modify(name, |record| {
        if playing(name) {
            return Err(std::io::Error::other(format!("{name} is playing a round")));
        }
        unseat(record, seat)?;
        log::info!("{name}: seat {} left", seat + 1);
        Ok(())
    })
}

/// Take the seat at `seat` out of `record` and renumber the seats behind it.
fn unseat(record: &mut ava_wire::Tournament, seat: usize) -> std::io::Result<()> {
    if seat >= record.seats.len() {
        return Err(std::io::Error::other(format!(
            "{} has no seat {}",
            record.name,
            seat + 1
        )));
    }
    if seat_is_held(record, seat) {
        return Err(std::io::Error::other(format!(
            "{}: seat {} played a round, it is fixed there",
            record.name,
            seat + 1
        )));
    }

    record.seats.remove(seat);
    // A round names its seats by number, so every seat behind the one that
    // left moves up one wherever a round holds it.
    let moved = |held: &mut usize| {
        if *held > seat {
            *held -= 1;
        }
    };
    for round in &mut record.rounds {
        for entry in &mut round.entries {
            moved(&mut entry.seat);
            if let Some(opponent) = &mut entry.opponent {
                moved(opponent);
            }
        }
        for pair in &mut round.pairs {
            moved(&mut pair.first);
            moved(&mut pair.second);
        }
        if let Some(bye) = &mut round.bye {
            moved(bye);
        }
        for pairing in &mut round.pairings {
            moved(&mut pairing.first);
            moved(&mut pairing.second);
        }
    }

    Ok(())
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

/// The seats that played `round`, the lobby as it stood then, the bye included.
pub fn seats_of(round: &ava_wire::Round) -> Vec<usize> {
    let mut seats: Vec<usize> = round.entries.iter().map(|entry| entry.seat).collect();
    for pair in &round.pairs {
        seats.extend([pair.first, pair.second]);
    }
    seats.extend(round.bye);
    seats.sort_unstable();
    seats.dedup();

    seats
}

/// The finished rounds a seat of the lobby has no entry in, which is what a
/// backfill plays. A round still playing or one that broke off is left alone.
pub fn unplayed_rounds(record: &ava_wire::Tournament) -> Vec<usize> {
    record
        .rounds
        .iter()
        .enumerate()
        .filter(|(_, round)| {
            round.finished_seconds.is_some() && seats_of(round).len() < record.seats.len()
        })
        .map(|(index, _)| index)
        .collect()
}

/// The pairs `round` settles, the lower seat first: its swiss pairs, or every
/// pair of the seats that played it.
pub fn round_pairs(round: &ava_wire::Round) -> Vec<(usize, usize)> {
    if round.paired() {
        return round
            .pairs
            .iter()
            .map(|pair| crate::swiss::ordered(pair.first, pair.second))
            .collect();
    }

    let seats = seats_of(round);
    let mut pairs = Vec::new();
    for (index, &first) in seats.iter().enumerate() {
        for &second in &seats[index + 1..] {
            pairs.push((first, second));
        }
    }

    pairs
}

/// A run a turn asks for: the seat, and the seat it faces alone if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub seat: usize,
    pub opponent: Option<usize>,
}

/// The runs `turn` of `round` asks for: one per seat of the lobby in a round
/// robin, what the pairs ask for in a swiss round.
pub fn slots(
    game: &dyn ava_game::Game,
    round: &ava_wire::Round,
    seats: usize,
    turn: usize,
) -> Vec<Slot> {
    if !round.paired() {
        return (0..seats)
            .map(|seat| Slot {
                seat,
                opponent: None,
            })
            .collect();
    }

    pair_slots(game, &round.pairs, turn)
}

/// The runs `turn` asks for from `pairs`: one per pair and direction for a
/// turn facing other seats, one per seat otherwise.
pub fn pair_slots(game: &dyn ava_game::Game, pairs: &[ava_wire::Pair], turn: usize) -> Vec<Slot> {
    if plays_against(game, turn) {
        return pairs
            .iter()
            .flat_map(|pair| {
                [
                    Slot {
                        seat: pair.first,
                        opponent: Some(pair.second),
                    },
                    Slot {
                        seat: pair.second,
                        opponent: Some(pair.first),
                    },
                ]
            })
            .collect();
    }

    let mut seats: Vec<usize> = pairs
        .iter()
        .flat_map(|pair| [pair.first, pair.second])
        .collect();
    seats.sort_unstable();
    seats.dedup();

    seats
        .into_iter()
        .map(|seat| Slot {
            seat,
            opponent: None,
        })
        .collect()
}

/// The entry `round` holds for `slot` of `turn`.
pub fn entry_of(round: &ava_wire::Round, slot: Slot, turn: usize) -> Option<&ava_wire::Entry> {
    round.entries.iter().find(|entry| {
        entry.seat == slot.seat && entry.turn == turn && entry.opponent == slot.opponent
    })
}

/// The entry playing `slot` of `turn`: the one it names, or the round robin
/// run of its seat, which covers an opponent that played the turn that way
/// too and none that joined later.
pub fn entry_for(round: &ava_wire::Round, slot: Slot, turn: usize) -> Option<&ava_wire::Entry> {
    let Some(opponent) = slot.opponent else {
        return entry_of(round, slot, turn);
    };
    let at_once = |seat: usize| {
        entry_of(
            round,
            Slot {
                seat,
                opponent: None,
            },
            turn,
        )
    };

    entry_of(round, slot, turn).or_else(|| at_once(opponent).and_then(|_| at_once(slot.seat)))
}

/// The entry `seat` played `turn` of `round` with against `opponent`.
fn entry_against<'a>(
    game: &dyn ava_game::Game,
    round: &'a ava_wire::Round,
    seat: usize,
    turn: usize,
    opponent: usize,
) -> Option<&'a ava_wire::Entry> {
    if round.paired() && plays_against(game, turn) {
        return entry_for(
            round,
            Slot {
                seat,
                opponent: Some(opponent),
            },
            turn,
        );
    }

    entry_of(
        round,
        Slot {
            seat,
            opponent: None,
        },
        turn,
    )
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
    let mut played = PlayedRound::new(game, round);
    let seconds = round.finished_seconds.unwrap_or(round.started_seconds);

    let mut pairings = Vec::new();
    for (first, second) in round_pairs(round) {
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

        let first_played = played.against(first, second)?;
        let second_played = played.against(second, first)?;
        if let Some(outcome) = game.outcome((first, &first_played), (second, &second_played)) {
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

/// What the runs of the last turn of a round spent, by seat and opponent,
/// which is what the weights of a rating weigh.
#[derive(Default)]
pub struct Spends(Vec<(usize, Option<usize>, ava_game::scoring::Spend)>);

impl Spends {
    /// What `seat` spent on its pairing with `opponent`, nothing when it banked
    /// no entry: it forfeits, so the pairing is not weighed.
    pub fn of(&self, seat: usize, opponent: usize) -> Option<ava_game::scoring::Spend> {
        let spent = |faced: Option<usize>| {
            self.0
                .iter()
                .find(|(spender, against, _)| *spender == seat && *against == faced)
                .map(|(_, _, spend)| *spend)
        };

        spent(Some(opponent)).or_else(|| spent(None))
    }
}

/// What the runs of the last turn of `round` spent.
pub fn spends(
    record: &ava_wire::Tournament,
    round: &ava_wire::Round,
    registry: &crate::registry::Registry,
) -> std::io::Result<Spends> {
    let last = find(&record.game)?.turns().len() - 1;
    let mut spent = Vec::new();

    for entry in round.entries.iter().filter(|entry| entry.turn == last) {
        let Some(banked) = entry.attempt else {
            continue;
        };
        let run = crate::runs::read(&std::path::Path::new(docker::RUN_DIRECTORY).join(&entry.run))?;
        spent.push((
            entry.seat,
            entry.opponent,
            ava_game::scoring::Spend {
                cost: run
                    .metrics
                    .as_ref()
                    .and_then(|metrics| registry.cost(&run.setup(), metrics))
                    .unwrap_or_default(),
                seconds: banked,
                limit: run.limit_seconds,
            },
        ));
    }

    Ok(Spends(spent))
}

/// The turns of a round as its seats played them, each set of runs read once.
struct PlayedRound<'a> {
    game: &'a dyn ava_game::Game,
    round: &'a ava_wire::Round,
    read: std::collections::HashMap<Vec<Option<&'a str>>, Vec<ava_game::Played>>,
}

impl<'a> PlayedRound<'a> {
    fn new(game: &'a dyn ava_game::Game, round: &'a ava_wire::Round) -> Self {
        Self {
            game,
            round,
            read: std::collections::HashMap::new(),
        }
    }

    /// The turns `seat` played in its pairing with `opponent`.
    fn against(&mut self, seat: usize, opponent: usize) -> std::io::Result<Vec<ava_game::Played>> {
        let (game, round) = (self.game, self.round);
        let picked: Vec<Option<&'a str>> = (0..game.turns().len())
            .map(|turn| {
                entry_against(game, round, seat, turn, opponent).map(|entry| entry.run.as_str())
            })
            .collect();
        if let Some(played) = self.read.get(&picked) {
            return Ok(played.clone());
        }

        let played = played_turns(game, round, seat, opponent)?;
        self.read.insert(picked, played.clone());

        Ok(played)
    }
}

/// The turns of `round` as `seat` played them against `opponent`: per turn
/// its entry of record and verdicts, or nothing.
fn played_turns(
    game: &dyn ava_game::Game,
    round: &ava_wire::Round,
    seat: usize,
    opponent: usize,
) -> std::io::Result<Vec<ava_game::Played>> {
    let mut played = Vec::new();
    for (turn, task) in game.turns().iter().enumerate() {
        let Some(entry) = entry_against(game, round, seat, turn, opponent) else {
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

/// The game of a tournament, or the error naming the known ones.
fn find(game: &str) -> std::io::Result<&'static dyn ava_game::Game> {
    ava_game::find(game).ok_or_else(|| {
        crate::registry::unknown(game, "game", ava_game::GAMES.iter().map(|game| game.name()))
    })
}

/// Write `record` as the record of its tournament, staged and renamed over
/// the record so a reader outside the records lock, the interface rendering a
/// page while a round settles its pairings, never reads a truncated file.
fn write(record: &ava_wire::Tournament) -> std::io::Result<()> {
    let folder = directory(&record.name);
    let staging = folder.join(RECORD_STAGING_FILE);
    std::fs::write(
        &staging,
        format!(
            "{}\n",
            serde_json::to_string_pretty(record).map_err(std::io::Error::other)?
        ),
    )?;

    std::fs::rename(staging, folder.join(RECORD_FILE))
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

/// What resuming a round that broke off does with its runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resume {
    /// Play the seats the round is missing a finished run of.
    Continue,
    /// Drop what the round holds and play every seat again.
    Restart,
    /// Play nothing and settle the round on what its runs left.
    Settle,
}

impl Resume {
    /// The words the command line and the interface name the modes by.
    pub const CONTINUE: &str = "continue";
    pub const RESTART: &str = "restart";
    pub const SETTLE: &str = "settle";

    /// The mode `word` names, if it names one.
    pub fn named(word: &str) -> Option<Self> {
        match word {
            Self::CONTINUE => Some(Self::Continue),
            Self::RESTART => Some(Self::Restart),
            Self::SETTLE => Some(Self::Settle),
            _ => None,
        }
    }

    /// The word this mode is named by.
    pub fn word(&self) -> &'static str {
        match self {
            Self::Continue => Self::CONTINUE,
            Self::Restart => Self::RESTART,
            Self::Settle => Self::SETTLE,
        }
    }

    /// The mode as what it is doing, for the notes and the log.
    pub fn doing(&self) -> &'static str {
        match self {
            Self::Continue => "continuing",
            Self::Restart => "restarting",
            Self::Settle => "settling",
        }
    }
}

/// Play one round of the named tournament: turn by turn a run per seat, all
/// at once, each seeded with the entries of the other seats the game asks
/// for, then the pairings settled.
///
/// A round a seat is missing from is backfilled first, so a new round never
/// stands beside a finished one the lobby did not all play.
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
    let playing = Playing::begin(name)?;
    let record = load(name)?;
    if record.seats.is_empty() {
        return Err(std::io::Error::other(format!("{name} has no seats")));
    }
    let owed = unplayed_rounds(&record);
    if !owed.is_empty() {
        let rounds: Vec<String> = owed.iter().map(|index| (index + 1).to_string()).collect();
        return Err(std::io::Error::other(format!(
            "{name}: backfill round {} before playing another round",
            rounds.join(", ")
        )));
    }
    let game = find(&record.game)?;
    let seats = record.seats.len();
    let index = record.rounds.len();
    let parallel = parallel.unwrap_or(DEFAULT_PARALLEL).max(1);

    let paired = if record.swiss() {
        let lobby: Vec<usize> = (0..seats).collect();
        crate::swiss::pair(
            &standings_before(&record, index, &lobby)?,
            &met_pairs(&record),
        )
    } else {
        crate::swiss::Paired {
            pairs: Vec::new(),
            bye: None,
        }
    };
    modify(name, |record| {
        record.rounds.push(ava_wire::Round {
            started_seconds: crate::usage::epoch_now(),
            finished_seconds: None,
            pairs: paired.pairs.clone(),
            bye: paired.bye,
            entries: Vec::new(),
            pairings: Vec::new(),
        });
        Ok(())
    })?;
    playing.plays_rounds(&[index])?;
    log::info!(
        "{name}: round {} starts, {seats} seats play {} over {} turns, {parallel} runs at once{}",
        index + 1,
        record.game,
        game.turns().len(),
        if record.swiss() {
            format!(", paired {}", described(&paired))
        } else {
            String::new()
        }
    );

    // A round just pushed is missing the run of every seat, so continuing it
    // plays them all.
    play_through(name, index, Resume::Continue, force_build_images, parallel)
}

/// The pairs and the bye for the log, seats counted from one.
fn described(paired: &crate::swiss::Paired) -> String {
    let mut parts: Vec<String> = paired
        .pairs
        .iter()
        .map(|pair| format!("{} against {}", pair.first + 1, pair.second + 1))
        .collect();
    if let Some(bye) = paired.bye {
        parts.push(format!("{} sits out", bye + 1));
    }

    parts.join(", ")
}

/// The standings `seats` pair by in the round at `index`, from the finished
/// rounds before it.
fn standings_before(
    record: &ava_wire::Tournament,
    index: usize,
    seats: &[usize],
) -> std::io::Result<Vec<crate::swiss::Standing>> {
    let before = &record.rounds[..index.min(record.rounds.len())];
    let mut won = vec![0.0; record.seats.len()];
    let mut counted = vec![0usize; record.seats.len()];
    for round in before
        .iter()
        .filter(|round| round.finished_seconds.is_some())
    {
        for pairing in pairings(record, round)? {
            let Some(score) = pairing.tally.score() else {
                continue;
            };
            for (seat, share) in [(pairing.first, score), (pairing.second, 1.0 - score)] {
                if let (Some(sum), Some(count)) = (won.get_mut(seat), counted.get_mut(seat)) {
                    *sum += share;
                    *count += 1;
                }
            }
        }
    }

    let random = std::collections::hash_map::RandomState::new();
    Ok(seats
        .iter()
        .map(|&seat| crate::swiss::Standing {
            seat,
            score: counted
                .get(seat)
                .filter(|count| **count > 0)
                .map(|count| won[seat] / *count as f64),
            byes: before
                .iter()
                .filter(|round| round.bye == Some(seat))
                .count(),
            tiebreak: std::hash::BuildHasher::hash_one(&random, seat),
        })
        .collect())
}

/// Every pair that met in a round of `record`.
fn met_pairs(record: &ava_wire::Tournament) -> std::collections::HashSet<(usize, usize)> {
    record.rounds.iter().flat_map(round_pairs).collect()
}

/// Resume the round at `index` of the named tournament, the way `resume` says,
/// and settle and finish it.
///
/// Only a round that broke off is resumed. A round the standings already
/// count is over, and playing it again is playing another round.
pub fn resume_round(
    name: &str,
    index: usize,
    resume: Resume,
    force_build_images: bool,
    parallel: Option<usize>,
) -> std::io::Result<i32> {
    let playing = Playing::begin(name)?;
    let record = load(name)?;
    if record.seats.is_empty() {
        return Err(std::io::Error::other(format!("{name} has no seats")));
    }
    let round = record
        .rounds
        .get(index)
        .ok_or_else(|| missing_round(name, index))?;
    if round.finished_seconds.is_some() {
        return Err(std::io::Error::other(format!(
            "{name}: round {} is over",
            index + 1
        )));
    }
    let parallel = parallel.unwrap_or(DEFAULT_PARALLEL).max(1);

    playing.plays_rounds(&[index])?;
    log::info!(
        "{name}: {} round {}, {parallel} runs at once",
        resume.doing(),
        index + 1
    );

    play_through(name, index, resume, force_build_images, parallel)
}

/// Play the round at `index` through: the runs `resume` asks for, turn by
/// turn, then the pairings and the second the round finished.
///
/// What the round already holds is banked before the turns, so a round that
/// broke off after its runs hands their entries to the turns that follow and
/// to the pairings.
fn play_through(
    name: &str,
    index: usize,
    resume: Resume,
    force_build_images: bool,
    parallel: usize,
) -> std::io::Result<i32> {
    let record = load(name)?;
    let game = find(&record.game)?;
    let seats = record.seats.len();
    let number = index + 1;
    let mut code = 0;

    if resume == Resume::Restart {
        modify(name, |record| {
            let round = record
                .rounds
                .get_mut(index)
                .ok_or_else(|| missing_round(name, index))?;
            round.entries.clear();
            round.pairings.clear();
            Ok(())
        })?;
        log::info!("{name}: round {number} drops what it held and plays again");
    }
    bank_entries(name, game, index)?;

    let analyses = Analyses::new(
        name,
        record.analyst.clone(),
        record.analyst_seconds,
        parallel,
    );

    for turn in 0..game.turns().len() {
        if resume == Resume::Settle || crate::interrupt::interrupted() {
            break;
        }

        let current = load(name)?;
        let round = current
            .rounds
            .get(index)
            .ok_or_else(|| missing_round(name, index))?;
        let work: Vec<(usize, Slot)> = slots_to_play(game, round, seats, turn)
            .into_iter()
            .map(|slot| (index, slot))
            .collect();
        let played = play_turn(
            name,
            &record,
            game,
            turn,
            &work,
            &analyses,
            force_build_images,
            parallel,
        )?;
        if code == 0 {
            code = played;
        }
    }

    if !crate::interrupt::interrupted() {
        settle(name, &record, index, game, &mut code)?;
    }

    // An interrupted round stays unfinished, so its forfeits never reach the
    // standings.
    if crate::interrupt::interrupted() {
        log::warn!("{name}: round {number} was interrupted and stays unfinished");
        analyses.finish();
        return Ok(code.max(1));
    }

    modify(name, |record| {
        record
            .rounds
            .get_mut(index)
            .ok_or_else(|| missing_round(name, index))?
            .finished_seconds = Some(crate::usage::epoch_now());
        Ok(())
    })?;
    log::info!("{name}: round {number} is over");
    analyses.finish();

    Ok(code)
}

/// Play `work`, rounds and slots of `turn`, at once under the cap. A run
/// whose opponents kept no entry is not started: it wins by forfeit anyway.
#[allow(clippy::too_many_arguments)]
fn play_turn(
    name: &str,
    record: &ava_wire::Tournament,
    game: &dyn ava_game::Game,
    turn: usize,
    work: &[(usize, Slot)],
    analyses: &Analyses,
    force_build_images: bool,
    parallel: usize,
) -> std::io::Result<i32> {
    let current = load(name)?;
    let seats = record.seats.len();
    let mut launches = Vec::new();
    let mut named: Vec<(usize, Slot, String)> = Vec::new();
    for &(index, slot) in work {
        let round = current
            .rounds
            .get(index)
            .ok_or_else(|| missing_round(name, index))?;
        let opponents: Vec<usize> = match slot.opponent {
            Some(opponent) => vec![opponent],
            None => (0..seats).filter(|other| *other != slot.seat).collect(),
        };
        let asked = game.inputs(turn, &opponents);
        let inputs = resolve_inputs(name, game, round, slot.seat, &asked);
        if !asked.is_empty() && inputs.is_empty() {
            log::info!(
                "{name}: round {}, turn {}: seat {} is not started, the seats it faces kept no entry",
                index + 1,
                turn + 1,
                slot.seat + 1
            );
            continue;
        }
        launches.push(launch(
            record,
            &record.seats[slot.seat],
            turn,
            inputs,
            force_build_images,
            parallel,
        )?);
        named.push((
            index,
            slot,
            docker::run_name(&record.seats[slot.seat].agent.harness),
        ));
    }
    if named.is_empty() {
        log::info!(
            "{name}: turn {} of {}: no run to play",
            turn + 1,
            game.turns().len()
        );
        return Ok(0);
    }

    // A slot played again drops the entry of its unfinished run.
    modify(name, |record| {
        for (index, slot, run) in &named {
            let round = record
                .rounds
                .get_mut(*index)
                .ok_or_else(|| missing_round(name, *index))?;
            round.entries.retain(|entry| {
                entry.seat != slot.seat || entry.turn != turn || entry.opponent != slot.opponent
            });
            round.entries.push(ava_wire::Entry {
                seat: slot.seat,
                turn,
                run: run.clone(),
                attempt: None,
                opponent: slot.opponent,
            });
        }
        Ok(())
    })?;
    let mut rounds: Vec<usize> = named.iter().map(|(index, _, _)| *index).collect();
    rounds.sort_unstable();
    rounds.dedup();
    let numbers: Vec<String> = rounds.iter().map(|index| (index + 1).to_string()).collect();
    log::info!(
        "{name}: turn {} of {}: {} runs play {} in round {}",
        turn + 1,
        game.turns().len(),
        named.len(),
        game.turns()[turn].task,
        numbers.join(", ")
    );

    let mut code = 0;
    let outcomes = bounded(named.len(), parallel, |at| {
        let run = &named[at].2;
        let outcome = docker::play(&launches[at], run);
        analyses.start(run);
        outcome
    });
    for ((_, _, run), outcome) in named.iter().zip(outcomes) {
        match outcome {
            Ok(finished) if code == 0 => code = finished,
            Ok(_) => {}
            Err(error) => {
                log::error!("{name}: the run {run} failed: {error}");
                code = 1;
            }
        }
    }

    for index in rounds {
        bank_entries(name, game, index)?;
    }

    Ok(code)
}

/// The runs `turn` of `round` still needs: slots with no finished run.
fn slots_to_play(
    game: &dyn ava_game::Game,
    round: &ava_wire::Round,
    seats: usize,
    turn: usize,
) -> Vec<Slot> {
    slots(game, round, seats, turn)
        .into_iter()
        .filter(|slot| match entry_for(round, *slot, turn) {
            Some(entry) => !finished_run(&entry.run),
            None => true,
        })
        .collect()
}

/// Whether the run has a record saying it is over. A run whose record cannot
/// be read never finished as far as the round is concerned.
fn finished_run(run: &str) -> bool {
    crate::runs::read(&std::path::Path::new(docker::RUN_DIRECTORY).join(run))
        .map(|record| record.finished_seconds.is_some())
        .unwrap_or(false)
}

/// Bank the entries of the round at `index` that have none: the entry of
/// record its run left on disk, which is what the turns that follow are
/// seeded with and what the pairings are settled on. An entry already banked
/// keeps the attempt it was recorded with, since that is what the standings
/// have counted.
fn bank_entries(name: &str, game: &dyn ava_game::Game, index: usize) -> std::io::Result<()> {
    let record = load(name)?;
    let round = record
        .rounds
        .get(index)
        .ok_or_else(|| missing_round(name, index))?;
    let unbanked: Vec<(String, usize)> = round
        .entries
        .iter()
        .filter(|entry| entry.attempt.is_none())
        .map(|entry| (entry.run.clone(), entry.turn))
        .collect();
    let kept: Vec<Option<u64>> = unbanked
        .iter()
        .map(|(run, turn)| banked(name, game, run, *turn).map(|kept| kept.seconds))
        .collect();

    modify(name, |record| {
        let round = record
            .rounds
            .get_mut(index)
            .ok_or_else(|| missing_round(name, index))?;
        for ((run, _), attempt) in unbanked.iter().zip(&kept) {
            if let Some(entry) = round
                .entries
                .iter_mut()
                .find(|entry| entry.run == *run && entry.attempt.is_none())
            {
                entry.attempt = *attempt;
            }
        }
        Ok(())
    })
}

/// Play the runs the seats that joined after a round are missing from it and
/// settle the pairings they add, every round at once under the same cap as a
/// round.
///
/// The entry is recorded once its run is over, so a backfill that broke off
/// leaves the round as it was and the next one plays the seat again.
pub fn backfill(
    name: &str,
    force_build_images: bool,
    parallel: Option<usize>,
) -> std::io::Result<i32> {
    let playing = Playing::begin(name)?;
    let record = load(name)?;
    let game = find(&record.game)?;
    let parallel = parallel.unwrap_or(DEFAULT_PARALLEL).max(1);
    if record.swiss() {
        return backfill_by_standing(name, &playing, force_build_images, parallel);
    }
    if game.turns().len() > 1 {
        return Err(std::io::Error::other(format!(
            "{name}: {LATE_JOIN_REFUSAL}"
        )));
    }

    let rounds = unplayed_rounds(&record);
    if rounds.is_empty() {
        log::info!("{name}: every seat played every finished round");
        return Ok(0);
    }
    let mut missing = Vec::new();
    for &index in &rounds {
        let played = seats_of(&record.rounds[index]);
        for seat in 0..record.seats.len() {
            if !played.contains(&seat) {
                missing.push((index, seat));
            }
        }
    }
    playing.plays_rounds(&rounds)?;
    log::info!(
        "{name}: {} runs backfill {} rounds, {parallel} runs at once",
        missing.len(),
        rounds.len()
    );

    let launches = missing
        .iter()
        .map(|(_, seat)| {
            launch(
                &record,
                &record.seats[*seat],
                ONLY_TURN,
                Vec::new(),
                force_build_images,
                parallel,
            )
        })
        .collect::<std::io::Result<Vec<docker::Launch>>>()?;
    let runs: Vec<String> = missing
        .iter()
        .map(|(_, seat)| docker::run_name(&record.seats[*seat].agent.harness))
        .collect();

    let analyses = Analyses::new(
        name,
        record.analyst.clone(),
        record.analyst_seconds,
        parallel,
    );
    let mut code = 0;
    if crate::interrupt::interrupted() {
        analyses.finish();
        return Ok(1);
    }

    // The rounds are written the moment their runs are named, so the graph of
    // each links its run while it plays, and they stay unfinished until the
    // backfill is over, so a round is out of the standings while it changes.
    let finished: Vec<(usize, Option<u64>)> = rounds
        .iter()
        .map(|index| (*index, record.rounds[*index].finished_seconds))
        .collect();
    modify(name, |record| {
        for ((index, seat), run) in missing.iter().zip(&runs) {
            let round = record
                .rounds
                .get_mut(*index)
                .ok_or_else(|| missing_round(name, *index))?;
            round.entries.push(ava_wire::Entry {
                seat: *seat,
                turn: ONLY_TURN,
                run: run.clone(),
                attempt: None,
                opponent: None,
            });
            round.finished_seconds = None;
        }
        Ok(())
    })?;

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
        .map(|run| banked(name, game, run, ONLY_TURN).map(|entry| entry.seconds))
        .collect();
    modify(name, |record| {
        for (((index, _), run), attempt) in missing.iter().zip(&runs).zip(&kept) {
            let round = record
                .rounds
                .get_mut(*index)
                .ok_or_else(|| missing_round(name, *index))?;
            if let Some(entry) = round.entries.iter_mut().find(|entry| entry.run == *run) {
                entry.attempt = *attempt;
            }
        }
        Ok(())
    })?;

    // An interrupted backfill leaves its rounds unfinished, so the seats it
    // did not play never forfeit them.
    if crate::interrupt::interrupted() {
        log::warn!("{name}: the backfill was interrupted and its rounds stay unfinished");
        analyses.finish();
        return Ok(code.max(1));
    }

    for &index in &rounds {
        settle(name, &record, index, game, &mut code)?;
    }
    modify(name, |record| {
        for (index, seconds) in &finished {
            record
                .rounds
                .get_mut(*index)
                .ok_or_else(|| missing_round(name, *index))?
                .finished_seconds = *seconds;
        }
        Ok(())
    })?;
    log::info!("{name}: the backfill is over");
    analyses.finish();

    Ok(code)
}

/// Backfill a swiss tournament: pair every round a seat missed up front, then
/// play the runs the new pairs need, turn by turn across all the rounds.
///
/// Nothing played before plays again, and the rounds keep their finish time,
/// so the ratings walk the matches in the same order.
fn backfill_by_standing(
    name: &str,
    playing: &Playing,
    force_build_images: bool,
    parallel: usize,
) -> std::io::Result<i32> {
    let record = load(name)?;
    let game = find(&record.game)?;
    let rounds = unplayed_rounds(&record);
    if rounds.is_empty() {
        log::info!("{name}: every seat played every finished round");
        return Ok(0);
    }
    playing.plays_rounds(&rounds)?;

    let plans = plan_late(&record, &rounds)?;
    for (index, paired) in &plans {
        log::info!(
            "{name}: round {} pairs the seats that joined late, {}",
            index + 1,
            described(paired)
        );
    }
    let finished: Vec<(usize, Option<u64>)> = rounds
        .iter()
        .map(|index| (*index, record.rounds[*index].finished_seconds))
        .collect();
    modify(name, |record| {
        for (index, paired) in &plans {
            let round = record
                .rounds
                .get_mut(*index)
                .ok_or_else(|| missing_round(name, *index))?;
            if !round.paired() {
                round.pairs = round_pairs(round)
                    .into_iter()
                    .map(|(first, second)| ava_wire::Pair { first, second })
                    .collect();
            }
            round.pairs.extend(paired.pairs.iter().copied());
            round.bye = paired.bye;
            round.finished_seconds = None;
        }
        Ok(())
    })?;
    log::info!(
        "{name}: backfilling {} rounds, {parallel} runs at once",
        rounds.len()
    );

    let analyses = Analyses::new(
        name,
        record.analyst.clone(),
        record.analyst_seconds,
        parallel,
    );
    let mut code = 0;
    for turn in 0..game.turns().len() {
        if crate::interrupt::interrupted() {
            break;
        }
        let current = load(name)?;
        let mut work = Vec::new();
        for (index, paired) in &plans {
            let round = current
                .rounds
                .get(*index)
                .ok_or_else(|| missing_round(name, *index))?;
            work.extend(
                pair_slots(game, &paired.pairs, turn)
                    .into_iter()
                    .filter(|slot| entry_of(round, *slot, turn).is_none())
                    .map(|slot| (*index, slot)),
            );
        }
        let played = play_turn(
            name,
            &record,
            game,
            turn,
            &work,
            &analyses,
            force_build_images,
            parallel,
        )?;
        if code == 0 {
            code = played;
        }
    }

    if !crate::interrupt::interrupted() {
        for (index, _) in &plans {
            settle(name, &record, *index, game, &mut code)?;
        }
    }
    // Unfinished rounds keep the seats it did not play from forfeiting.
    if crate::interrupt::interrupted() {
        log::warn!("{name}: the backfill was interrupted and its rounds stay unfinished");
        analyses.finish();
        return Ok(code.max(1));
    }

    modify(name, |record| {
        for (index, seconds) in &finished {
            record
                .rounds
                .get_mut(*index)
                .ok_or_else(|| missing_round(name, *index))?
                .finished_seconds = *seconds;
        }
        Ok(())
    })?;
    log::info!("{name}: the backfill is over");
    analyses.finish();

    Ok(code)
}

/// The pairs a backfill adds to `rounds`, each counting as met in the next.
fn plan_late(
    record: &ava_wire::Tournament,
    rounds: &[usize],
) -> std::io::Result<Vec<(usize, crate::swiss::Paired)>> {
    let mut planned = std::collections::HashSet::new();
    let mut plans = Vec::new();
    for &index in rounds {
        let paired = late_pairs(record, index, &planned)?;
        planned.extend(
            paired
                .pairs
                .iter()
                .map(|pair| crate::swiss::ordered(pair.first, pair.second)),
        );
        plans.push((index, paired));
    }

    Ok(plans)
}

/// The pairs a backfill adds to the round at `index`, and its bye after: the
/// missing seats and the bye pair up, one left over meets the closest score
/// that played. `planned` counts as met.
fn late_pairs(
    record: &ava_wire::Tournament,
    index: usize,
    planned: &std::collections::HashSet<(usize, usize)>,
) -> std::io::Result<crate::swiss::Paired> {
    let round = record
        .rounds
        .get(index)
        .ok_or_else(|| missing_round(&record.name, index))?;
    let present = seats_of(round);
    let mut pool: Vec<usize> = (0..record.seats.len())
        .filter(|seat| !present.contains(seat))
        .collect();
    pool.extend(round.bye);
    let lobby: Vec<usize> = (0..record.seats.len()).collect();
    let standings = standings_before(record, index, &lobby)?;
    let (pooled, played): (Vec<crate::swiss::Standing>, Vec<crate::swiss::Standing>) = standings
        .into_iter()
        .filter(|standing| present.contains(&standing.seat) || pool.contains(&standing.seat))
        .partition(|standing| pool.contains(&standing.seat));

    let mut met = met_pairs(record);
    met.extend(planned.iter().copied());
    let mut paired = crate::swiss::pair(&pooled, &met);
    if let Some(left) = paired.bye {
        let standing = pooled
            .iter()
            .find(|standing| standing.seat == left)
            .expect("the seat left over is one of the pool");
        if let Some(opponent) = crate::swiss::closest(standing, &played, &met) {
            paired.pairs.push(crate::swiss::pair_of(left, opponent));
            paired.bye = None;
        }
    }

    Ok(paired)
}

/// The entry of record `run` kept of `turn`, logging a run whose entries
/// cannot be read.
fn banked(
    name: &str,
    game: &dyn ava_game::Game,
    run: &str,
    turn: usize,
) -> Option<crate::runs::Entry> {
    let directory = std::path::Path::new(docker::RUN_DIRECTORY).join(run);

    crate::runs::entry_of_record(game, &directory, crate::runs::turn_entry(game, turn))
        .unwrap_or_else(|error| {
            log::warn!("{name}: the entries of {run} cannot be read: {error}");
            None
        })
}

/// The launch of the seat holding `setup` playing `turn` of the tournament's
/// game, given the `inputs` of that turn.
fn launch(
    record: &ava_wire::Tournament,
    setup: &ava_wire::Setup,
    turn: usize,
    inputs: Vec<docker::InputFile>,
    force_build_images: bool,
    parallel: usize,
) -> std::io::Result<docker::Launch> {
    let (name, model) = match &setup.name {
        Some(named) => (named.clone(), String::new()),
        None => (setup.agent.harness.clone(), setup.agent.model.clone()),
    };
    let mut launch = docker::prepare(&docker::Agent {
        name,
        model,
        game: record.game.clone(),
        limit: record.limit_seconds,
        parallel: parallel as u64,
        thinking: setup.thinking.clone(),
        force_build_images,
        analyst: None,
        turn,
        inputs,
    })?;
    // The round runs the analyses itself, so the run only records one.
    launch.analyst = record.analyst.clone();

    Ok(launch)
}

/// The files behind the `inputs` of the turn `seat` plays: the entries
/// of record the earlier turns of `round` kept. An input whose seat kept
/// nothing is left out, and the verifier of the turn says what that means.
fn resolve_inputs(
    name: &str,
    game: &dyn ava_game::Game,
    round: &ava_wire::Round,
    seat: usize,
    inputs: &[ava_game::Input],
) -> Vec<docker::InputFile> {
    inputs
        .iter()
        .filter_map(|input| {
            let entry = entry_against(game, round, input.seat, input.turn, seat)?;
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

/// Settle the pairings of the round at `index` the game cannot read from the
/// records and that were not settled before: every such pair of seats fights
/// in the scorer image, one fight after the other, each recorded as it ends.
/// A seat without an entry of the last turn forfeits its fights.
fn settle(
    name: &str,
    record: &ava_wire::Tournament,
    index: usize,
    game: &dyn ava_game::Game,
    code: &mut i32,
) -> std::io::Result<()> {
    let current = load(name)?;
    let round = current
        .rounds
        .get(index)
        .ok_or_else(|| missing_round(name, index))?;
    let mut played = PlayedRound::new(game, round);
    let number = index + 1;
    let console = directory(name).join(format!("{ROUND_LOG_PREFIX}{number}{ROUND_LOG_SUFFIX}"));

    for (first, second) in round_pairs(round) {
        if round.pairings.iter().any(|pairing| {
            (pairing.first, pairing.second) == (first, second)
                || (pairing.first, pairing.second) == (second, first)
        }) {
            continue;
        }
        let first_played = played.against(first, second)?;
        let second_played = played.against(second, first)?;
        if game
            .outcome((first, &first_played), (second, &second_played))
            .is_some()
        {
            continue;
        }
        if crate::interrupt::interrupted() {
            log::warn!("{name}: round {number} was interrupted before every pairing fought");
            *code = 1;
            return Ok(());
        }

        let entry = |played: &[ava_game::Played]| played.last().and_then(|turn| turn.entry.clone());
        let (tally, reason) = match (entry(&first_played), entry(&second_played)) {
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
                let forfeited = ava_game::forfeit(
                    first,
                    kept_first.is_some(),
                    second,
                    kept_second.is_some(),
                    game.forfeited_rounds(record.combats),
                );
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
            index,
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
    analyst: Option<ava_wire::Setup>,
    /// The seconds every analysis is given.
    seconds: u64,
    /// The queue the capped workers take runs from.
    queue: Option<std::sync::Mutex<std::sync::mpsc::Sender<String>>>,
    handles: std::sync::Mutex<Vec<std::thread::JoinHandle<()>>>,
}

impl Analyses {
    /// Ready to analyze the runs of the named tournament with `analyst`, at
    /// most `parallel` at a time.
    fn new(name: &str, analyst: Option<ava_wire::Setup>, seconds: u64, parallel: usize) -> Self {
        let mut analyses = Self {
            name: name.to_string(),
            analyst,
            seconds,
            queue: None,
            handles: std::sync::Mutex::new(Vec::new()),
        };

        if let Some(analyst) = &analyses.analyst {
            let (sender, receiver) = std::sync::mpsc::channel::<String>();
            let receiver = std::sync::Arc::new(std::sync::Mutex::new(receiver));
            let mut handles = Vec::new();
            for _ in 0..parallel.max(1) {
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

    /// Queue `run` for analysis, unless the tournament has no analyst.
    fn start(&self, run: &str) {
        if let Some(queue) = &self.queue {
            let _ = queue
                .lock()
                .expect("the queue is not poisoned")
                .send(run.to_string());
        }
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
fn analyze(name: &str, analyst: &ava_wire::Setup, seconds: u64, run: &str) {
    log::info!("{name}: analyzing {run} with {}", analyst.label());
    let outcome = docker::analyze(&docker::Analyze {
        run: run.to_string(),
        analyst: docker::Analyst::of(analyst, seconds),
    });
    if let Err(error) = outcome {
        log::error!("{name}: the analysis of {run} failed: {error}");
    }
}

/// Play `count` runs, at most `parallel` sandboxes at a time. The outcomes
/// come back in the order the runs were given.
fn bounded(
    count: usize,
    parallel: usize,
    job: impl Fn(usize) -> std::io::Result<i32> + Sync,
) -> Vec<std::io::Result<i32>> {
    let workers = parallel.clamp(1, count.max(1));
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

/// Record `pairing` on the round at `index` of the named tournament.
fn record_pairing(name: &str, index: usize, pairing: ava_wire::Pairing) -> std::io::Result<()> {
    modify(name, |record| {
        record
            .rounds
            .get_mut(index)
            .ok_or_else(|| missing_round(name, index))?
            .pairings
            .push(pairing);
        Ok(())
    })
}

/// The error naming a round the named tournament does not have.
fn missing_round(name: &str, index: usize) -> std::io::Error {
    std::io::Error::other(format!("{name} has no round {}", index + 1))
}

#[cfg(test)]
mod tests {
    /// A round the seats named by `entries` played, each entry a seat and a turn.
    fn round(entries: &[(usize, usize)]) -> ava_wire::Round {
        ava_wire::Round {
            started_seconds: 0,
            finished_seconds: Some(1),
            pairs: Vec::new(),
            bye: None,
            entries: entries
                .iter()
                .map(|(seat, turn)| ava_wire::Entry {
                    seat: *seat,
                    turn: *turn,
                    run: format!("run-{seat}-{turn}"),
                    attempt: None,
                    opponent: None,
                })
                .collect(),
            pairings: Vec::new(),
        }
    }

    #[test]
    fn a_round_pairs_the_seats_that_played_it() {
        let played = round(&[(0, 0), (1, 0), (3, 0)]);

        assert_eq!(super::seats_of(&played), vec![0, 1, 3]);
        assert_eq!(super::round_pairs(&played), vec![(0, 1), (0, 3), (1, 3)]);
    }

    #[test]
    fn every_turn_of_a_seat_names_it_once() {
        let played = round(&[(0, 0), (1, 0), (0, 1), (1, 1)]);

        assert_eq!(super::seats_of(&played), vec![0, 1]);
        assert_eq!(super::round_pairs(&played), vec![(0, 1)]);
    }

    /// A tournament of `seats` seats playing `rounds`, each round the seats
    /// that played it and whether it finished.
    fn tournament(seats: usize, rounds: &[(&[usize], bool)]) -> ava_wire::Tournament {
        ava_wire::Tournament {
            version: ava_wire::VERSION,
            name: "scratch".to_string(),
            game: "sanity-check".to_string(),
            game_version: String::new(),
            pairing: ava_wire::ROUND_ROBIN.to_string(),
            limit_seconds: 300,
            combats: 1,
            analyst: None,
            analyst_seconds: ava_wire::DEFAULT_ANALYST_SECONDS,
            created_seconds: 0,
            seats: (0..seats)
                .map(|seat| ava_wire::Setup {
                    agent: ava_wire::Agent {
                        harness: "pi".to_string(),
                        model: format!("model-{seat}"),
                    },
                    thinking: None,
                    backend: None,
                    name: None,
                })
                .collect(),
            rounds: rounds
                .iter()
                .map(|(played, finished)| {
                    let mut round =
                        round(&played.iter().map(|seat| (*seat, 0)).collect::<Vec<_>>());
                    round.finished_seconds = finished.then_some(1);
                    round
                })
                .collect(),
        }
    }

    #[test]
    fn a_backfill_plays_the_finished_rounds_a_seat_is_missing_from() {
        let joined = tournament(3, &[(&[0, 1], true), (&[0, 1, 2], true)]);
        assert_eq!(super::unplayed_rounds(&joined), vec![0]);

        let whole = tournament(2, &[(&[0, 1], true)]);
        assert!(super::unplayed_rounds(&whole).is_empty());
    }

    #[test]
    fn a_round_that_did_not_finish_is_left_alone() {
        let playing = tournament(2, &[(&[0, 1], true), (&[], false)]);

        assert!(super::unplayed_rounds(&playing).is_empty());
    }

    #[test]
    fn a_round_no_seat_played_pairs_nothing() {
        assert!(super::seats_of(&round(&[])).is_empty());
        assert!(super::round_pairs(&round(&[])).is_empty());
    }

    #[test]
    fn a_marker_names_the_rounds_of_its_play() {
        assert_eq!(super::marked_rounds("4711 1,3"), vec![1, 3]);
        assert_eq!(super::marked_rounds("4711 2"), vec![2]);
        assert!(super::marked_rounds("4711").is_empty());
        assert!(super::marked_rounds("").is_empty());
    }

    #[test]
    fn a_seat_no_round_holds_leaves_and_renumbers_the_seats_behind_it() {
        let mut joined = tournament(4, &[(&[0, 1, 3], true)]);
        joined.rounds[0].pairings = vec![ava_wire::Pairing {
            first: 1,
            second: 3,
            seconds: 1,
            tally: ava_wire::Tally::first_won(1),
            reason: None,
            run: None,
        }];

        assert!(!super::seat_is_held(&joined, 2));
        super::unseat(&mut joined, 2).unwrap();

        assert_eq!(joined.seats.len(), 3);
        assert_eq!(super::seats_of(&joined.rounds[0]), vec![0, 1, 2]);
        assert_eq!(
            (
                joined.rounds[0].pairings[0].first,
                joined.rounds[0].pairings[0].second
            ),
            (1, 2)
        );
    }

    #[test]
    fn a_seat_a_round_holds_stays() {
        let mut played = tournament(3, &[(&[0, 1, 2], true)]);

        assert!(super::seat_is_held(&played, 1));
        assert!(super::unseat(&mut played, 1).is_err());
        assert!(super::unseat(&mut played, 3).is_err());
        assert_eq!(played.seats.len(), 3);
    }

    /// Each slot as seat and opponent.
    fn slotted(slots: &[super::Slot]) -> Vec<(usize, Option<usize>)> {
        slots
            .iter()
            .map(|slot| (slot.seat, slot.opponent))
            .collect()
    }

    fn game(name: &str) -> &'static dyn ava_game::Game {
        ava_game::find(name).unwrap()
    }

    #[test]
    fn a_seat_plays_a_turn_it_has_no_finished_run_of() {
        // The runs the helper names are not on disk, so none of them finished.
        let played = round(&[(0, 0), (1, 0)]);
        let sanity = game("sanity-check");

        assert_eq!(
            slotted(&super::slots_to_play(sanity, &played, 3, 0)),
            vec![(0, None), (1, None), (2, None)]
        );
        assert_eq!(
            slotted(&super::slots_to_play(sanity, &round(&[]), 2, 0)),
            vec![(0, None), (1, None)]
        );
    }

    /// A swiss round of `pairs`, nothing played yet.
    fn paired(pairs: &[(usize, usize)], bye: Option<usize>) -> ava_wire::Round {
        let mut paired = round(&[]);
        paired.pairs = pairs
            .iter()
            .map(|(first, second)| ava_wire::Pair {
                first: *first,
                second: *second,
            })
            .collect();
        paired.bye = bye;
        paired
    }

    #[test]
    fn a_paired_round_plays_an_attack_once_per_pair_and_direction() {
        let crackme = game("crackme");
        let round = paired(&[(0, 3), (1, 2)], Some(4));

        assert!(!super::plays_against(crackme, 0));
        assert!(super::plays_against(crackme, 1));
        assert_eq!(
            slotted(&super::slots(crackme, &round, 5, 0)),
            vec![(0, None), (1, None), (2, None), (3, None)]
        );
        assert_eq!(
            slotted(&super::slots(crackme, &round, 5, 1)),
            vec![(0, Some(3)), (3, Some(0)), (1, Some(2)), (2, Some(1))]
        );
        assert_eq!(super::round_pairs(&round), vec![(0, 3), (1, 2)]);
        assert_eq!(super::seats_of(&round), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn a_paired_round_plays_a_turn_facing_nobody_once_per_seat() {
        let sanity = game("sanity-check");
        let round = paired(&[(0, 1), (1, 2)], None);

        assert_eq!(
            slotted(&super::slots(sanity, &round, 3, 0)),
            vec![(0, None), (1, None), (2, None)]
        );
    }

    #[test]
    fn an_attack_is_read_against_the_seat_it_faced() {
        let mut round = paired(&[(0, 1), (0, 2)], None);
        for (seat, turn, opponent) in [(0, 0, None), (0, 1, Some(1)), (0, 1, Some(2)), (1, 0, None)]
        {
            round.entries.push(ava_wire::Entry {
                seat,
                turn,
                run: format!("run-{seat}-{turn}-{opponent:?}"),
                attempt: None,
                opponent,
            });
        }

        let run = |seat, turn, opponent| {
            super::entry_against(game("crackme"), &round, seat, turn, opponent)
                .map(|entry| entry.run.clone())
        };
        assert_eq!(run(0, 1, 2).as_deref(), Some("run-0-1-Some(2)"));
        assert_eq!(run(0, 1, 1).as_deref(), Some("run-0-1-Some(1)"));
        assert_eq!(run(0, 0, 2).as_deref(), Some("run-0-0-None"));
        assert_eq!(run(1, 1, 0), None);
    }

    #[test]
    fn a_run_facing_every_seat_covers_the_seats_it_faced_alone() {
        // A round robin of seats 0 and 1, backfilled with seat 2.
        let mut round = paired(&[(0, 1), (0, 2)], None);
        for (seat, turn, opponent) in [
            (0, 0, None),
            (1, 0, None),
            (0, 1, None),
            (1, 1, None),
            (2, 0, None),
            (2, 1, Some(0)),
        ] {
            round.entries.push(ava_wire::Entry {
                seat,
                turn,
                run: format!("run-{seat}-{turn}"),
                attempt: None,
                opponent,
            });
        }

        let covered = |seat, opponent| {
            super::entry_for(
                &round,
                super::Slot {
                    seat,
                    opponent: Some(opponent),
                },
                1,
            )
            .map(|entry| entry.run.clone())
        };
        assert_eq!(covered(0, 1).as_deref(), Some("run-0-1"));
        assert_eq!(covered(1, 0).as_deref(), Some("run-1-1"));
        assert_eq!(covered(0, 2), None);
        assert_eq!(covered(2, 0).as_deref(), Some("run-2-1"));
    }

    #[test]
    fn a_late_seat_joins_a_paired_round_in_the_seat_left_out() {
        let mut joined = tournament(4, &[]);
        joined.pairing = ava_wire::SWISS.to_string();
        joined.rounds.push(paired(&[(0, 1)], Some(2)));

        assert_eq!(super::unplayed_rounds(&joined), vec![0]);
        let late = super::late_pairs(&joined, 0, &Default::default()).unwrap();
        assert_eq!(
            late.pairs,
            vec![ava_wire::Pair {
                first: 2,
                second: 3
            }]
        );
        assert_eq!(late.bye, None);
    }

    #[test]
    fn a_late_seat_left_over_meets_a_seat_that_played_the_round() {
        let mut joined = tournament(3, &[]);
        joined.pairing = ava_wire::SWISS.to_string();
        joined.rounds.push(paired(&[(0, 1)], None));

        let late = super::late_pairs(&joined, 0, &Default::default()).unwrap();
        assert_eq!(late.pairs.len(), 1);
        assert_eq!(late.pairs[0].second, 2);
        assert_eq!(late.bye, None);
    }

    #[test]
    fn a_seat_catching_up_on_several_rounds_meets_a_different_seat_in_each() {
        let mut joined = tournament(3, &[]);
        joined.pairing = ava_wire::SWISS.to_string();
        joined.rounds.push(paired(&[(0, 1)], None));
        joined.rounds.push(paired(&[(0, 1)], None));

        let plans = super::plan_late(&joined, &[0, 1]).unwrap();
        let opponent = |plan: usize| plans[plan].1.pairs[0].first;
        assert_eq!(plans.len(), 2);
        assert_ne!(opponent(0), opponent(1));
    }

    #[test]
    fn a_seat_leaving_renumbers_the_pairs_and_the_bye() {
        let mut joined = tournament(4, &[]);
        joined.rounds.push(paired(&[(0, 3)], Some(1)));
        joined.rounds[0].entries.push(ava_wire::Entry {
            seat: 0,
            turn: 1,
            run: "attack".to_string(),
            attempt: None,
            opponent: Some(3),
        });

        super::unseat(&mut joined, 2).unwrap();

        let round = &joined.rounds[0];
        assert_eq!(
            round.pairs,
            vec![ava_wire::Pair {
                first: 0,
                second: 2
            }]
        );
        assert_eq!(round.bye, Some(1));
        assert_eq!(round.entries[0].opponent, Some(2));
    }

    #[test]
    fn a_game_facing_other_seats_is_paired_swiss_unless_chosen_otherwise() {
        assert_eq!(super::default_pairing(game("crackme")), ava_wire::SWISS);
        assert_eq!(
            super::default_pairing(game("r2wars-gb")),
            ava_wire::ROUND_ROBIN
        );
        assert!(super::checked_pairing("swiss").is_ok());
        assert!(super::checked_pairing("knockout").is_err());
    }
}
