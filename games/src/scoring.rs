//! Pairing the seats of a tournament and rating the matches they played.

const ANCHOR_RATING: f64 = 1000.0;
const RATING_SCALE: f64 = 400.0;
const LOGISTIC_BASE: f64 = 10.0;
const K_FACTOR: f64 = 32.0;
const VIRTUAL_DRAW: f64 = 0.5;
const FIT_ITERATIONS: u32 = 100;

/// The matches a rating needs behind it before it counts in full.
///
/// A rating over a handful of matches is mostly the anchor it started at, and
/// an agent that won its only fight is not the best agent there is.
const SETTLED_MATCHES: f64 = 10.0;

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

/// The most either weight can be: the whole of the score of a match.
const MAXIMUM_WEIGHT: f64 = 1.0;

/// The share of a match a weight of one moves, so a win under the worst gap
/// there is comes out a draw and never a loss.
const MOVED_SHARE: f64 = 0.5;

/// What the run behind one side of a pairing spent.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spend {
    /// The dollars of the run, at the price of its route.
    pub cost: f64,
    /// The second its entry of record was banked at, on the scoring clock.
    pub seconds: u64,
    /// The seconds the run was given.
    pub limit: u64,
}

impl Spend {
    /// The share of its budget the run spent before it banked what counts.
    pub fn slowness(&self) -> f64 {
        if self.limit == 0 {
            return 0.0;
        }

        (self.seconds as f64 / self.limit as f64).min(1.0)
    }
}

/// The shares of its points an entry loses for the cost and the speed of the
/// run that kept it, both nothing for the points as the game ranks them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Weights {
    /// The share the dearest run of a round loses, every other run in proportion to its cost.
    pub cost: f64,
    /// The share an entry banked as the budget ran out loses, an earlier one in proportion.
    pub speed: f64,
}

impl Weights {
    /// The weights clamped between nothing and the whole of the points.
    pub fn clamped(cost: f64, speed: f64) -> Self {
        Self {
            cost: cost.clamp(0.0, MAXIMUM_WEIGHT),
            speed: speed.clamp(0.0, MAXIMUM_WEIGHT),
        }
    }

    /// Whether the weights leave the points alone.
    pub fn none(&self) -> bool {
        *self == Self::default()
    }

    /// The `score` of the first side of a pairing, in `[0, 1]`, moved by what
    /// the two sides spent: the dearer side hands over the cost weight's
    /// share of how far apart the two bills are, and the slower side the
    /// speed weight's share of how far apart the two entries were banked in
    /// the budget. A weight of one moves half a match at most, so the widest
    /// gap there is turns a win into a draw and never into a loss.
    pub fn weighed_score(&self, score: f64, first: Spend, second: Spend) -> f64 {
        let apart = |first: f64, second: f64| {
            let sum = first + second;
            if sum > 0.0 {
                (first - second) / sum
            } else {
                0.0
            }
        };
        let moved = MOVED_SHARE
            * (self.cost * apart(first.cost, second.cost)
                + self.speed * apart(first.slowness(), second.slowness()));

        (score - moved).clamp(0.0, 1.0)
    }
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

/// The matches `pairings` make between the agents the seat `labels` name,
/// in the order given: one per pairing that saw a round.
pub fn matches<'a>(
    labels: &[String],
    pairings: impl IntoIterator<Item = &'a ava_wire::Pairing>,
) -> Vec<Match> {
    pairings
        .into_iter()
        .filter_map(|pairing| {
            Some(Match {
                first: labels.get(pairing.first)?.clone(),
                second: labels.get(pairing.second)?.clone(),
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

/// `rating` pulled back towards the anchor by how few `matches` back it: one
/// match keeps a tenth of the distance, ten matches half, and it fades from
/// there.
pub fn settled(rating: f64, matches: u64) -> f64 {
    let matches = matches as f64;

    ANCHOR_RATING + (rating - ANCHOR_RATING) * matches / (matches + SETTLED_MATCHES)
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
    fn weights_move_a_match_by_what_the_sides_spent() {
        let dear = super::Spend {
            cost: 4.0,
            seconds: 600,
            limit: 600,
        };
        let cheap = super::Spend {
            cost: 0.0,
            seconds: 0,
            limit: 600,
        };

        let none = super::Weights::default();
        assert_eq!(none.weighed_score(1.0, dear, cheap), 1.0);

        let cost = super::Weights::clamped(1.0, 0.0);
        assert_eq!(cost.weighed_score(1.0, dear, cheap), 0.5);
        assert_eq!(cost.weighed_score(1.0, cheap, dear), 1.0);
        assert_eq!(cost.weighed_score(0.0, dear, cheap), 0.0);

        // Twice the bill of the other side is a third of the gap.
        let twice = super::Spend { cost: 2.0, ..cheap };
        let once = super::Spend { cost: 1.0, ..cheap };
        assert!((cost.weighed_score(1.0, twice, once) - 5.0 / 6.0).abs() < 1e-12);

        let both = super::Weights::clamped(0.5, 0.5);
        assert_eq!(both.weighed_score(0.5, dear, cheap), 0.0);
        assert_eq!(both.weighed_score(0.5, cheap, cheap), 0.5);
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
    fn a_rating_settles_as_its_matches_pile_up() {
        assert_eq!(super::settled(1200.0, 0), super::ANCHOR_RATING);
        assert_eq!(super::settled(1200.0, 10), 1100.0);
        assert!(super::settled(1200.0, 1000) > 1198.0);
        assert!(super::settled(800.0, 10) < super::ANCHOR_RATING);
    }

    #[test]
    fn matches_between_the_same_agent_move_nothing() {
        let matches = [played("a", "a", 1.0), played("a", "a", 0.0)];

        for rating in Elo.leaderboard(&matches) {
            assert_eq!(rating.rating, super::ANCHOR_RATING);
        }
    }
}
