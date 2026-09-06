use crate::chess_vm::assembler::{Author, Program, assemble};
use crate::chess_vm::playout::{self, Failures};

pub(crate) const OPPONENTS: [(&str, &str); 27] = [
    ("random", include_str!("opponents/random.cvm")),
    ("greedy", include_str!("opponents/greedy.cvm")),
    ("cccp", include_str!("opponents/cccp.cvm")),
    ("min_opp", include_str!("opponents/min_opp.cvm")),
    ("max_opp", include_str!("opponents/max_opp.cvm")),
    ("suicide_king", include_str!("opponents/suicide_king.cvm")),
    ("huddle", include_str!("opponents/huddle.cvm")),
    ("swarm", include_str!("opponents/swarm.cvm")),
    ("pacifist", include_str!("opponents/pacifist.cvm")),
    ("generous", include_str!("opponents/generous.cvm")),
    ("alphabetical", include_str!("opponents/alphabetical.cvm")),
    ("same_color", include_str!("opponents/same_color.cvm")),
    (
        "opposite_color",
        include_str!("opponents/opposite_color.cvm"),
    ),
    ("equalizer", include_str!("opponents/equalizer.cvm")),
    ("shallow", include_str!("opponents/shallow.cvm")),
    ("steady", include_str!("opponents/steady.cvm")),
    ("sensei", include_str!("opponents/sensei.cvm")),
    ("positional", include_str!("opponents/positional.cvm")),
    ("tactical", include_str!("opponents/tactical.cvm")),
    ("weighted", include_str!("opponents/weighted.cvm")),
    ("composite", include_str!("opponents/composite.cvm")),
    ("king_attack", include_str!("opponents/king_attack.cvm")),
    ("bulwark", include_str!("opponents/bulwark.cvm")),
    ("pawnstorm", include_str!("opponents/pawnstorm.cvm")),
    ("phases", include_str!("opponents/phases.cvm")),
    ("hoarder", include_str!("opponents/hoarder.cvm")),
    ("loose_pieces", include_str!("opponents/loose_pieces.cvm")),
];

const CALIBRATION_SEED: u64 = 0x00C0_FFEE;

const CALIBRATION_GAMES_PER_PAIR: u32 = 6;

const CALIBRATION_PLY_CAP: u32 = 100;

const GRADING_SEED: u64 = 0x009E_7D0F;

const GRADING_GAMES: u32 = 20;

pub(crate) const GRADING_PLY_CAP: u32 = 120;

const RATING_SCALE: f64 = 400.0;
const LOGISTIC_BASE: f64 = 10.0;

const FIT_ITERATIONS: u32 = 4000;
const FIT_STEP: f64 = 4.0;
const FIT_PRIOR: f64 = 0.004;

const PERFORMANCE_MARGIN: f64 = 1200.0;
const PERFORMANCE_STEPS: u32 = 48;

pub struct Anchor {
    pub name: &'static str,
    program: Program,
    pub rating: f64,
}

pub struct Field {
    pub anchors: Vec<Anchor>,
    pub lowest: f64,
    pub highest: f64,
}

impl Field {
    pub fn load() -> Field {
        match cache::read() {
            Some(ratings) => Field::rated(ratings),
            None => {
                let field = calibrate();
                cache::write(&field);
                field
            }
        }
    }

    fn rated(ratings: Vec<f64>) -> Field {
        let anchors = programs()
            .into_iter()
            .zip(ratings)
            .map(|((name, program), rating)| Anchor {
                name,
                program,
                rating,
            })
            .collect();

        Field::from_anchors(anchors)
    }

    fn from_anchors(anchors: Vec<Anchor>) -> Field {
        let ratings = || anchors.iter().map(|anchor| anchor.rating);

        Field {
            lowest: ratings().fold(f64::INFINITY, f64::min),
            highest: ratings().fold(f64::NEG_INFINITY, f64::max),
            anchors,
        }
    }
}

fn programs() -> Vec<(&'static str, Program)> {
    OPPONENTS
        .into_iter()
        .map(|(name, source)| {
            let program = assemble(source, Author::Opponent).unwrap_or_else(|error| {
                panic!("the built-in opponent {name} does not assemble: {error}")
            });
            (name, program)
        })
        .collect()
}

pub fn calibrate() -> Field {
    let programs = programs();
    let count = programs.len();

    let mut tasks = Vec::new();
    for first in 0..count {
        for second in (first + 1)..count {
            for game in 0..CALIBRATION_GAMES_PER_PAIR {
                tasks.push((first, second, game));
            }
        }
    }

    let play = |&(first, second, game): &(usize, usize, u32)| -> (usize, usize, f64) {
        let first_is_white = game % 2 == 0;
        let seed = playout::derive_seed(
            CALIBRATION_SEED,
            (first * count + second) as u64 * 64 + game as u64,
        );
        let (white, black) = if first_is_white {
            (first, second)
        } else {
            (second, first)
        };
        let played = playout::play(
            &programs[white].1,
            &programs[black].1,
            seed,
            CALIBRATION_PLY_CAP,
        );
        let points = if first_is_white {
            played.white_points
        } else {
            1.0 - played.white_points
        };

        (first, second, points)
    };

    let mut games = vec![vec![0.0f64; count]; count];
    let mut points = vec![vec![0.0f64; count]; count];
    for (first, second, scored) in playout::run_in_parallel(&tasks, &play) {
        points[first][second] += scored;
        points[second][first] += 1.0 - scored;
        games[first][second] += 1.0;
        games[second][first] += 1.0;
    }

    Field::rated(fit_ratings(&games, &points))
}

fn fit_ratings(games: &[Vec<f64>], points: &[Vec<f64>]) -> Vec<f64> {
    let count = games.len();
    let scored: Vec<f64> = points.iter().map(|against| against.iter().sum()).collect();
    let mut ratings = vec![0.0f64; count];

    for _ in 0..FIT_ITERATIONS {
        let mut gradient = vec![0.0f64; count];
        for player in 0..count {
            let mut modelled = 0.0;
            for opponent in 0..count {
                if player != opponent {
                    modelled += games[player][opponent]
                        * expected_score(ratings[player], ratings[opponent]);
                }
            }
            gradient[player] = scored[player] - modelled - FIT_PRIOR * ratings[player];
        }

        for player in 0..count {
            ratings[player] += FIT_STEP * gradient[player];
        }

        let mean = ratings.iter().sum::<f64>() / count as f64;
        for rating in &mut ratings {
            *rating -= mean;
        }
    }

    let lowest = ratings.iter().copied().fold(f64::INFINITY, f64::min);
    for rating in &mut ratings {
        *rating -= lowest;
    }

    ratings
}

fn expected_score(rating: f64, opponent: f64) -> f64 {
    1.0 / (1.0 + LOGISTIC_BASE.powf((opponent - rating) / RATING_SCALE))
}

pub struct Standing {
    pub name: &'static str,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
}

pub struct Report {
    pub rating: f64,
    pub standings: Vec<Standing>,
    pub failures: Failures,
}

fn grade(program: &Program, field: &Field, games: u32, ply_cap: u32, seed: u64) -> Report {
    let mut tasks = Vec::new();
    for anchor in 0..field.anchors.len() {
        for game in 0..games {
            tasks.push((anchor, game));
        }
    }

    let play = |&(anchor, game): &(usize, u32)| -> (usize, f64, Failures) {
        let submission_is_white = game % 2 == 0;
        let seed = playout::derive_seed(seed, anchor as u64 * 4096 + game as u64);
        let opponent = &field.anchors[anchor].program;
        let (white, black) = if submission_is_white {
            (program, opponent)
        } else {
            (opponent, program)
        };
        let played = playout::play(white, black, seed, ply_cap);

        if submission_is_white {
            (anchor, played.white_points, played.white_failures)
        } else {
            (anchor, 1.0 - played.white_points, played.black_failures)
        }
    };

    let mut standings: Vec<Standing> = field
        .anchors
        .iter()
        .map(|anchor| Standing {
            name: anchor.name,
            wins: 0,
            draws: 0,
            losses: 0,
        })
        .collect();
    let mut failures = Failures::default();
    let mut scored = 0.0f64;

    for (anchor, points, game_failures) in playout::run_in_parallel(&tasks, &play) {
        failures.add(&game_failures);
        scored += points;
        let standing = &mut standings[anchor];
        match playout::outcome(points) {
            playout::Outcome::Won => standing.wins += 1,
            playout::Outcome::Drawn => standing.draws += 1,
            playout::Outcome::Lost => standing.losses += 1,
        }
    }

    Report {
        rating: performance_rating(field, scored, games),
        standings,
        failures,
    }
}

pub fn grade_submission(program: &Program, field: &Field) -> Report {
    grade(program, field, GRADING_GAMES, GRADING_PLY_CAP, GRADING_SEED)
}

fn performance_rating(field: &Field, scored: f64, games: u32) -> f64 {
    let mut low = field.lowest - PERFORMANCE_MARGIN;
    let mut high = field.highest + PERFORMANCE_MARGIN;

    for _ in 0..PERFORMANCE_STEPS {
        let middle = (low + high) / 2.0;
        let expected: f64 = field
            .anchors
            .iter()
            .map(|anchor| games as f64 * expected_score(middle, anchor.rating))
            .sum();

        if expected < scored {
            low = middle;
        } else {
            high = middle;
        }
    }

    (low + high) / 2.0
}

mod cache {
    use super::{
        CALIBRATION_GAMES_PER_PAIR, CALIBRATION_PLY_CAP, CALIBRATION_SEED, Field, OPPONENTS,
    };

    const FILE_PREFIX: &str = "ava-chess-vm-anchors-";
    const SEPARATOR: char = ' ';

    const HASH_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
    const HASH_PRIME: u64 = 0x0000_0100_0000_01B3;

    fn hashed(fingerprint: &[u8]) -> u64 {
        let mut hash = HASH_OFFSET;

        for byte in fingerprint {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(HASH_PRIME);
        }

        hash
    }

    fn fingerprint() -> Vec<u8> {
        let mut bytes = Vec::new();

        for (name, source) in OPPONENTS {
            bytes.extend_from_slice(name.as_bytes());
            bytes.extend_from_slice(source.as_bytes());
        }
        bytes.extend_from_slice(
            format!("{CALIBRATION_SEED}{CALIBRATION_GAMES_PER_PAIR}{CALIBRATION_PLY_CAP}")
                .as_bytes(),
        );

        if let Ok(executable) = std::env::current_exe()
            && let Ok(metadata) = std::fs::metadata(executable)
        {
            bytes.extend_from_slice(
                format!("{:?}{}", metadata.modified(), metadata.len()).as_bytes(),
            );
        }

        bytes
    }

    fn path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("{FILE_PREFIX}{:016x}", hashed(&fingerprint())))
    }

    pub(super) fn read() -> Option<Vec<f64>> {
        let path = path();
        let contents = std::fs::read_to_string(&path).ok()?;

        let mut ratings = Vec::with_capacity(OPPONENTS.len());
        for (line, (name, _)) in contents.lines().zip(OPPONENTS) {
            let (written, rating) = line.split_once(SEPARATOR)?;
            if written != name {
                return None;
            }
            ratings.push(rating.parse().ok()?);
        }

        if !valid(&ratings) {
            log::warn!("{} does not contain valid ratings", path.display());
            return None;
        }

        log::info!("read the ratings from {}", path.display());

        Some(ratings)
    }

    pub(super) fn valid(ratings: &[f64]) -> bool {
        let lowest = ratings.iter().copied().fold(f64::INFINITY, f64::min);
        let highest = ratings.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        ratings.len() == OPPONENTS.len()
            && ratings.iter().all(|rating| rating.is_finite())
            && lowest == 0.0
            && highest > 0.0
    }

    pub(super) fn write(field: &Field) {
        let path = path();
        let mut contents = String::new();
        for anchor in &field.anchors {
            contents.push_str(&format!("{}{SEPARATOR}{}\n", anchor.name, anchor.rating));
        }

        let pending = path.with_extension(std::process::id().to_string());
        let written =
            std::fs::write(&pending, contents).and_then(|()| std::fs::rename(&pending, &path));

        match written {
            Ok(()) => log::info!("wrote the ratings to {}", path.display()),
            Err(error) => log::warn!("cannot write the ratings to {path:?}: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess_vm::assembler::Author;

    const STARTER: &str = include_str!("task/bot.cvm");
    const REFERENCE: &str = include_str!("reference.cvm");

    const RATED: [(&str, f64); 27] = [
        ("sensei", 1200.8536),
        ("steady", 1111.5738),
        ("composite", 970.8463),
        ("weighted", 935.1479),
        ("shallow", 930.8074),
        ("tactical", 926.4912),
        ("loose_pieces", 864.1785),
        ("hoarder", 840.2682),
        ("positional", 836.3277),
        ("phases", 836.3277),
        ("pawnstorm", 714.9901),
        ("bulwark", 678.4289),
        ("king_attack", 603.1121),
        ("greedy", 543.2904),
        ("opposite_color", 459.6847),
        ("cccp", 449.2196),
        ("min_opp", 431.7289),
        ("same_color", 431.7289),
        ("alphabetical", 367.7885),
        ("huddle", 360.5384),
        ("swarm", 345.9157),
        ("equalizer", 338.5363),
        ("random", 261.2521),
        ("generous", 257.1703),
        ("suicide_king", 248.9269),
        ("max_opp", 182.8909),
        ("pacifist", 0.0000),
    ];

    const TOLERANCE: f64 = 0.05;

    fn submission(source: &str) -> Program {
        assemble(source, Author::Agent).expect("the program assembles")
    }

    #[test]
    #[ignore]
    fn the_round_robin_reproduces_the_recorded_ratings() {
        let field = calibrate();

        for (name, rating) in RATED {
            let anchor = field
                .anchors
                .iter()
                .find(|anchor| anchor.name == name)
                .unwrap_or_else(|| panic!("{name} is not in the field"));

            assert!(
                (anchor.rating - rating).abs() < TOLERANCE,
                "{name} is rated {:.4}, it was rated {rating:.4}",
                anchor.rating
            );
        }

        assert_eq!(field.lowest, 0.0, "the weakest opponent has rating 0");
        assert!((field.highest - RATED[0].1).abs() < TOLERANCE);
    }

    #[test]
    fn an_invalid_cache_is_rejected() {
        let rated: Vec<f64> = (0..OPPONENTS.len()).map(|place| place as f64).collect();
        assert!(cache::valid(&rated));

        for spoiled in [
            rated[..OPPONENTS.len() - 1].to_vec(),
            rated.iter().map(|rating| rating + 1.0).collect(),
            vec![0.0; OPPONENTS.len()],
            rated
                .iter()
                .map(|rating| if *rating == 0.0 { f64::NAN } else { *rating })
                .collect(),
        ] {
            assert!(
                !cache::valid(&spoiled),
                "{spoiled:?} is not a valid set of ratings"
            );
        }
    }

    #[test]
    #[ignore]
    fn the_reference_rates_far_above_the_starter() {
        let field = calibrate();

        let starter = grade_submission(&submission(STARTER), &field);
        let reference = grade_submission(&submission(REFERENCE), &field);

        assert!(
            starter.failures.is_empty(),
            "the starter answers every decision"
        );
        assert!(
            reference.failures.is_empty(),
            "the reference answers every decision"
        );

        assert!(
            (250.0..400.0).contains(&starter.rating),
            "the starter rating is {:.0}",
            starter.rating
        );
        assert!(
            reference.rating > 900.0,
            "the reference rating is {:.0}",
            reference.rating
        );
        assert!(
            reference.rating < field.highest,
            "the reference rates below the strongest opponent"
        );
    }
}
