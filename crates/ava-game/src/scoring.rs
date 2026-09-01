//! Rating systems turning played matches into a leaderboard.

const HALF_FORFEITED: f64 = 0.0;
const HALF_DRAWN: f64 = 0.5;
const HALF_STUMPED: f64 = 1.0;
const ANCHOR_RATING: f64 = 1000.0;
const RATING_SCALE: f64 = 400.0;
const LOGISTIC_BASE: f64 = 10.0;
const K_FACTOR: f64 = 32.0;
const VIRTUAL_DRAW: f64 = 0.5;
const FIT_ITERATIONS: u32 = 100;

/// One half of a match: one agent authored the puzzle, the other attempted it.
#[derive(Debug)]
pub struct Half {
    /// The verifier accepted the reference solution the generator submitted.
    pub valid: bool,
    /// A fresh generator instance solved its own puzzle from the solver's view.
    pub generator_solved: bool,
    /// The solver passed the verifier within its budget.
    pub solver_solved: bool,
}

/// A played match: both agents generated one puzzle and solved one.
#[derive(Debug)]
pub struct Match {
    /// The agent generating in the first half.
    pub first: String,
    /// The agent generating in the second half.
    pub second: String,
    /// The half where `first` generated and `second` solved.
    pub first_half: Half,
    /// The half where `second` generated and `first` solved.
    pub second_half: Half,
}

/// An agent's place on the leaderboard.
#[derive(Debug)]
pub struct Rating {
    /// The rated agent.
    pub agent: String,
    /// The rating on an Elo like scale anchored at 1000.
    pub rating: f64,
}

/// A rating system turning played matches into a leaderboard.
pub trait Scoring {
    /// The name identifying the rating system.
    const NAME: &'static str;

    /// Rate every agent appearing in `matches`, best first.
    fn leaderboard(&self, matches: &[Match]) -> Vec<Rating>;
}

/// The score of one match for the `first` agent, in [0, 1].
///
/// A generator takes a half by stumping the solver with a puzzle it solved
/// itself. An invalid puzzle, or one the generator cannot solve from the
/// solver's view, forfeits the half. Both sides solving is a draw.
pub fn match_score(played: &Match) -> f64 {
    (half_score(&played.first_half) + (1.0 - half_score(&played.second_half))) / 2.0
}

fn half_score(half: &Half) -> f64 {
    if !half.valid || !half.generator_solved {
        return HALF_FORFEITED;
    }
    if half.solver_solved {
        HALF_DRAWN
    } else {
        HALF_STUMPED
    }
}

/// Bradley-Terry ratings fitted over the whole match history.
///
/// One virtual draw between every pair of agents keeps each rating finite and
/// the comparison graph connected.
pub struct BradleyTerry;

impl Scoring for BradleyTerry {
    const NAME: &'static str = "bradley-terry";

    fn leaderboard(&self, matches: &[Match]) -> Vec<Rating> {
        let agents = participants(matches);
        let count = agents.len();

        if count < 2 {
            return agents
                .into_iter()
                .map(|agent| Rating {
                    agent,
                    rating: ANCHOR_RATING,
                })
                .collect();
        }

        let mut wins = vec![vec![0.0f64; count]; count];
        for played in matches {
            let first = position(&agents, &played.first);
            let second = position(&agents, &played.second);
            if first == second {
                continue;
            }

            let score = match_score(played);
            wins[first][second] += score;
            wins[second][first] += 1.0 - score;
        }

        for (agent, row) in wins.iter_mut().enumerate() {
            for (opponent, won) in row.iter_mut().enumerate() {
                if agent != opponent {
                    *won += VIRTUAL_DRAW;
                }
            }
        }

        let mut strengths = vec![1.0f64; count];
        for _ in 0..FIT_ITERATIONS {
            let mut updated = vec![0.0f64; count];
            for agent in 0..count {
                let won: f64 = wins[agent].iter().sum();
                let mut denominator = 0.0;
                for opponent in 0..count {
                    if opponent == agent {
                        continue;
                    }
                    let played = wins[agent][opponent] + wins[opponent][agent];
                    denominator += played / (strengths[agent] + strengths[opponent]);
                }
                updated[agent] = won / denominator;
            }
            normalize(&mut updated);
            strengths = updated;
        }

        ranked(
            agents
                .into_iter()
                .zip(strengths)
                .map(|(agent, strength)| Rating {
                    agent,
                    rating: ANCHOR_RATING + RATING_SCALE * strength.log10(),
                })
                .collect(),
        )
    }
}

/// Elo ratings updated sequentially in match order.
pub struct Elo;

impl Scoring for Elo {
    const NAME: &'static str = "elo";

    fn leaderboard(&self, matches: &[Match]) -> Vec<Rating> {
        let agents = participants(matches);
        let mut ratings = vec![ANCHOR_RATING; agents.len()];

        for played in matches {
            let first = position(&agents, &played.first);
            let second = position(&agents, &played.second);
            if first == second {
                continue;
            }

            let advantage = (ratings[second] - ratings[first]) / RATING_SCALE;
            let expected = 1.0 / (1.0 + LOGISTIC_BASE.powf(advantage));
            let shift = K_FACTOR * (match_score(played) - expected);
            ratings[first] += shift;
            ratings[second] -= shift;
        }

        ranked(
            agents
                .into_iter()
                .zip(ratings)
                .map(|(agent, rating)| Rating { agent, rating })
                .collect(),
        )
    }
}

/// Every agent appearing in `matches`, in order of appearance.
fn participants(matches: &[Match]) -> Vec<String> {
    let mut agents: Vec<String> = Vec::new();

    for played in matches {
        for agent in [&played.first, &played.second] {
            if !agents.iter().any(|known| known == agent) {
                agents.push(agent.clone());
            }
        }
    }

    agents
}

fn position(agents: &[String], agent: &str) -> usize {
    agents
        .iter()
        .position(|known| known == agent)
        .expect("every match participant is collected")
}

/// Scale the strengths to a geometric mean of one, pinning the fit.
fn normalize(strengths: &mut [f64]) {
    let mean = strengths.iter().map(|strength| strength.ln()).sum::<f64>() / strengths.len() as f64;
    let scale = mean.exp();

    for strength in strengths.iter_mut() {
        *strength /= scale;
    }
}

fn ranked(mut leaderboard: Vec<Rating>) -> Vec<Rating> {
    leaderboard.sort_by(|left, right| {
        right
            .rating
            .partial_cmp(&left.rating)
            .expect("ratings are finite")
    });
    leaderboard
}
