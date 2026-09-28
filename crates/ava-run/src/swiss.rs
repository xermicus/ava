//! Swiss pairing: seats that scored alike meet, rematches only when
//! unavoidable, and an odd lobby leaves one seat out.

/// The steps the search for a pairing without rematches may take before
/// settling for the greedy one.
const SEARCH_STEPS: usize = 100_000;

/// What pairing a round knows about one seat.
#[derive(Clone, Debug, PartialEq)]
pub struct Standing {
    pub seat: usize,
    /// The share of its pairings won, draws half, nothing before its first.
    pub score: Option<f64>,
    /// The rounds the seat sat out.
    pub byes: usize,
    /// Orders equal scores, drawn at random every round.
    pub tiebreak: u64,
}

/// A round as it was paired.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paired {
    pub pairs: Vec<ava_wire::Pair>,
    pub bye: Option<usize>,
}

/// The seats by score, unscored last, ties by their tiebreak.
fn ranked(standings: &[Standing]) -> Vec<&Standing> {
    let mut ranked: Vec<&Standing> = standings.iter().collect();
    ranked.sort_by(|first, second| {
        let score = |standing: &Standing| standing.score.unwrap_or(f64::NEG_INFINITY);
        score(second)
            .total_cmp(&score(first))
            .then(first.tiebreak.cmp(&second.tiebreak))
    });

    ranked
}

/// Pair `standings`, `met` holding the pairs that met before.
///
/// The bye goes to the lowest ranked of the seats that sat out least. Every
/// other seat meets the highest ranked unmet seat below it, or the next one
/// down when no pairing avoids a rematch.
pub fn pair(standings: &[Standing], met: &std::collections::HashSet<(usize, usize)>) -> Paired {
    let mut order: Vec<usize> = ranked(standings)
        .iter()
        .map(|standing| standing.seat)
        .collect();

    let bye = if order.len() % 2 == 1 {
        let fewest = standings
            .iter()
            .map(|standing| standing.byes)
            .min()
            .unwrap_or_default();
        let position = order
            .iter()
            .rposition(|seat| {
                standings
                    .iter()
                    .any(|standing| standing.seat == *seat && standing.byes == fewest)
            })
            .expect("an odd lobby has a seat with the fewest byes");
        Some(order.remove(position))
    } else {
        None
    };

    let mut pairs = Vec::new();
    let mut used = vec![false; order.len()];
    let mut steps = 0;
    if !search(&order, met, &mut used, &mut pairs, &mut steps) {
        pairs = order.chunks(2).map(|two| pair_of(two[0], two[1])).collect();
    }

    Paired { pairs, bye }
}

/// Pair the rest of `order` without a rematch, backtracking where a seat is
/// stranded. False when that is impossible or out of steps.
fn search(
    order: &[usize],
    met: &std::collections::HashSet<(usize, usize)>,
    used: &mut Vec<bool>,
    pairs: &mut Vec<ava_wire::Pair>,
    steps: &mut usize,
) -> bool {
    let Some(first) = used.iter().position(|taken| !taken) else {
        return true;
    };
    used[first] = true;

    for second in first + 1..order.len() {
        *steps += 1;
        if *steps > SEARCH_STEPS {
            break;
        }
        if used[second] || met.contains(&ordered(order[first], order[second])) {
            continue;
        }
        used[second] = true;
        pairs.push(pair_of(order[first], order[second]));
        if search(order, met, used, pairs, steps) {
            return true;
        }
        pairs.pop();
        used[second] = false;
    }

    used[first] = false;
    false
}

/// The seat of `candidates` closest in score to `seat`, unmet ones first.
pub fn closest(
    seat: &Standing,
    candidates: &[Standing],
    met: &std::collections::HashSet<(usize, usize)>,
) -> Option<usize> {
    let distance = |candidate: &Standing| {
        (candidate.score.unwrap_or_default() - seat.score.unwrap_or_default()).abs()
    };
    let unmet: Vec<&Standing> = candidates
        .iter()
        .filter(|candidate| !met.contains(&ordered(seat.seat, candidate.seat)))
        .collect();
    let pool: Vec<&Standing> = if unmet.is_empty() {
        candidates.iter().collect()
    } else {
        unmet
    };

    pool.into_iter()
        .min_by(|first, second| {
            distance(first)
                .total_cmp(&distance(second))
                .then(first.tiebreak.cmp(&second.tiebreak))
        })
        .map(|candidate| candidate.seat)
}

/// The two seats, the lower first.
pub fn ordered(first: usize, second: usize) -> (usize, usize) {
    (first.min(second), first.max(second))
}

/// The pair of two seats.
pub fn pair_of(first: usize, second: usize) -> ava_wire::Pair {
    let (first, second) = ordered(first, second);
    ava_wire::Pair { first, second }
}

#[cfg(test)]
mod tests {
    /// Standings from score and byes, ties in seat order.
    fn standings(seats: &[(Option<f64>, usize)]) -> Vec<super::Standing> {
        seats
            .iter()
            .enumerate()
            .map(|(seat, (score, byes))| super::Standing {
                seat,
                score: *score,
                byes: *byes,
                tiebreak: seat as u64,
            })
            .collect()
    }

    fn pairs(paired: &super::Paired) -> Vec<(usize, usize)> {
        paired
            .pairs
            .iter()
            .map(|pair| (pair.first, pair.second))
            .collect()
    }

    #[test]
    fn winners_meet_winners() {
        let lobby = standings(&[
            (Some(0.0), 0),
            (Some(1.0), 0),
            (Some(0.0), 0),
            (Some(1.0), 0),
        ]);
        let paired = super::pair(&lobby, &Default::default());

        assert_eq!(pairs(&paired), vec![(1, 3), (0, 2)]);
        assert_eq!(paired.bye, None);
    }

    #[test]
    fn seats_that_met_meet_the_next_one_down() {
        let lobby = standings(&[
            (Some(1.0), 0),
            (Some(1.0), 0),
            (Some(0.0), 0),
            (Some(0.0), 0),
        ]);
        let met = std::collections::HashSet::from([(0, 1), (2, 3)]);
        let paired = super::pair(&lobby, &met);

        assert_eq!(pairs(&paired), vec![(0, 2), (1, 3)]);
    }

    #[test]
    fn a_round_that_cannot_avoid_a_rematch_still_pairs_everyone() {
        let lobby = standings(&[(Some(1.0), 0), (Some(0.0), 0)]);
        let met = std::collections::HashSet::from([(0, 1)]);

        assert_eq!(pairs(&super::pair(&lobby, &met)), vec![(0, 1)]);
    }

    #[test]
    fn the_bye_goes_to_the_lowest_seat_that_sat_out_least() {
        let lobby = standings(&[(Some(1.0), 0), (Some(0.5), 0), (Some(0.0), 1)]);
        let paired = super::pair(&lobby, &Default::default());

        assert_eq!(paired.bye, Some(1));
        assert_eq!(pairs(&paired), vec![(0, 2)]);
    }

    #[test]
    fn a_seat_without_a_score_ranks_last() {
        let lobby = standings(&[(None, 0), (Some(0.0), 0), (Some(1.0), 0)]);

        assert_eq!(super::pair(&lobby, &Default::default()).bye, Some(0));
    }

    #[test]
    fn a_left_over_seat_meets_the_closest_score_it_has_not_met() {
        let lobby = standings(&[
            (Some(0.5), 0),
            (Some(0.875), 0),
            (Some(0.25), 0),
            (Some(0.75), 0),
        ]);
        let met = std::collections::HashSet::from([(0, 2)]);

        assert_eq!(super::closest(&lobby[0], &lobby[1..], &met), Some(3));
        let everyone = std::collections::HashSet::from([(0, 1), (0, 2), (0, 3)]);
        assert_eq!(super::closest(&lobby[0], &lobby[1..], &everyone), Some(2));
    }
}
