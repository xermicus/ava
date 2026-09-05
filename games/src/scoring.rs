//! Pairing the seats of a tournament and rating the matches they played.

const ANCHOR_RATING: f64 = 1000.0;
const RATING_SCALE: f64 = 400.0;
const LOGISTIC_BASE: f64 = 10.0;
const K_FACTOR: f64 = 32.0;
const VIRTUAL_DRAW: f64 = 0.5;
const FIT_ITERATIONS: u32 = 100;

/// A played match between two agents.
#[derive(Clone, Debug, PartialEq)]
pub struct Match {
    pub first: String,
    pub second: String,
    /// The score of `first` in `[0, 1]`: one for a win, half for a draw.
    pub score: f64,
}

/// An agent's place on the leaderboard.
#[derive(Clone, Debug, PartialEq)]
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

/// Every pair of `seats` once, the first seat of each pair the lower one.
pub fn round_robin(seats: usize) -> Vec<(usize, usize)> {
    (0..seats)
        .flat_map(|first| (first + 1..seats).map(move |second| (first, second)))
        .collect()
}

/// The matches `pairings` make between the agents `seats` hold, in the order
/// given: one per pairing that saw a round.
pub fn matches<'a>(
    seats: &[ava_wire::Agent],
    pairings: impl IntoIterator<Item = &'a ava_wire::Pairing>,
) -> Vec<Match> {
    pairings
        .into_iter()
        .filter_map(|pairing| {
            let first = seats.get(pairing.first)?;
            let second = seats.get(pairing.second)?;
            Some(Match {
                first: first.label(),
                second: second.label(),
                score: pairing.tally.score()?,
            })
        })
        .collect()
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

            wins[first][second] += played.score;
            wins[second][first] += 1.0 - played.score;
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
            let shift = K_FACTOR * (played.score - expected);
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
pub fn participants(matches: &[Match]) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::{BradleyTerry, Elo, Match, Scoring};

    fn played(first: &str, second: &str, score: f64) -> Match {
        Match {
            first: first.to_string(),
            second: second.to_string(),
            score,
        }
    }

    #[test]
    fn round_robin_pairs_every_seat_once() {
        assert_eq!(super::round_robin(1), Vec::<(usize, usize)>::new());
        assert_eq!(super::round_robin(3), vec![(0, 1), (0, 2), (1, 2)]);
    }

    #[test]
    fn the_winner_leads_under_both_systems() {
        let matches = [
            played("a", "b", 1.0),
            played("b", "c", 1.0),
            played("a", "c", 1.0),
        ];

        for leaderboard in [
            Elo.leaderboard(&matches),
            BradleyTerry.leaderboard(&matches),
        ] {
            let order: Vec<&str> = leaderboard
                .iter()
                .map(|rating| rating.agent.as_str())
                .collect();
            assert_eq!(order, ["a", "b", "c"]);
        }
    }

    #[test]
    fn matches_between_the_same_agent_move_nothing() {
        let matches = [played("a", "a", 1.0), played("a", "a", 0.0)];

        for rating in Elo.leaderboard(&matches) {
            assert_eq!(rating.rating, super::ANCHOR_RATING);
        }
    }
}
