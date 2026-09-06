use crate::chess_vm::assembler::Program;
use crate::chess_vm::board::{
    self, BLACK, CASTLES, Move, Position, SQUARES, TAKES_EN_PASSANT, WHITE,
};
use crate::chess_vm::machine::{self, Failure};

const ADJUDICATION_MARGIN: i32 = 400;

const WIN: f64 = 1.0;
const DRAW: f64 = 0.5;
const LOSS: f64 = 0.0;

const WON_ABOVE: f64 = 0.75;
const DRAWN_ABOVE: f64 = 0.25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Won,
    Drawn,
    Lost,
}

pub fn outcome(points: f64) -> Outcome {
    if points > WON_ABOVE {
        Outcome::Won
    } else if points > DRAWN_ABOVE {
        Outcome::Drawn
    } else {
        Outcome::Lost
    }
}

pub struct Random(u64);

impl Random {
    pub fn new(seed: u64) -> Random {
        Random(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;

        self.0
    }

    pub fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound.max(1) as u64) as usize
    }
}

pub fn derive_seed(seed: u64, stream: u64) -> u64 {
    let mut mixed = seed
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(stream.wrapping_add(1).wrapping_mul(0xBF58_476D_1CE4_E5B9));
    mixed ^= mixed >> 30;
    mixed = mixed.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    mixed ^= mixed >> 27;
    mixed = mixed.wrapping_mul(0x94D0_49BB_1331_11EB);
    mixed ^= mixed >> 31;

    mixed | 1
}

#[derive(Default, Clone)]
pub struct Failures {
    pub faults: u32,
    pub overruns: u32,
    pub first_message: Option<String>,
}

impl Failures {
    pub fn is_empty(&self) -> bool {
        self.faults == 0 && self.overruns == 0
    }

    fn record(&mut self, failure: Option<Failure>) {
        match failure {
            None => {}
            Some(Failure::Overrun) => self.overruns += 1,
            Some(Failure::Fault(message)) => {
                self.faults += 1;
                if self.first_message.is_none() {
                    self.first_message = Some(message);
                }
            }
        }
    }

    pub fn add(&mut self, other: &Failures) {
        self.faults += other.faults;
        self.overruns += other.overruns;
        if self.first_message.is_none() {
            self.first_message = other.first_message.clone();
        }
    }
}

pub struct Playout {
    pub white_points: f64,
    pub white_failures: Failures,
    pub black_failures: Failures,
}

pub fn play(white: &Program, black: &Program, seed: u64, ply_cap: u32) -> Playout {
    let mut position = Position::start();
    let mut history = vec![position];
    let mut random = Random::new(seed);
    let mut move_counts = [0i32; SQUARES];
    let mut white_failures = Failures::default();
    let mut black_failures = Failures::default();
    let mut ply = 0i32;

    let winner = loop {
        let legal = position.legal_moves();
        if legal.is_empty() {
            break position
                .in_check(position.side)
                .then(|| board::opponent(position.side));
        }
        if position.halfmove_clock >= board::FIFTY_MOVE_PLIES || position.insufficient_material() {
            break None;
        }
        if history
            .iter()
            .filter(|earlier| earlier.repeats(&position))
            .count()
            >= board::REPETITIONS_DRAWN
        {
            break None;
        }
        if ply as u32 >= ply_cap {
            break adjudicate(&position);
        }

        let (program, faults) = if position.side == WHITE {
            (white, &mut white_failures)
        } else {
            (black, &mut black_failures)
        };

        let (chosen, failure) =
            machine::choose_move(program, &position, &legal, &mut random, &move_counts, ply);
        faults.record(failure);

        let played = legal[chosen];
        advance_move_counts(&mut move_counts, played);
        position.make(played);
        history.push(position);
        ply += 1;
    };

    let white_points = match winner {
        None => DRAW,
        Some(WHITE) => WIN,
        Some(_) => LOSS,
    };

    Playout {
        white_points,
        white_failures,
        black_failures,
    }
}

fn adjudicate(position: &Position) -> Option<i8> {
    let balance = board::material_for(position, WHITE);

    if balance > ADJUDICATION_MARGIN {
        Some(WHITE)
    } else if balance < -ADJUDICATION_MARGIN {
        Some(BLACK)
    } else {
        None
    }
}

fn advance_move_counts(counts: &mut [i32; SQUARES], played: Move) {
    let from = played.from as usize;
    let to = played.to as usize;

    counts[to] = counts[from] + 1;
    counts[from] = 0;

    if played.does(TAKES_EN_PASSANT) {
        let captured = if to > from { to - 8 } else { to + 8 };
        counts[captured] = 0;
    }

    if played.does(CASTLES) {
        let (rook_from, rook_to) = board::castling_rook(played.to as i32);
        counts[rook_to as usize] = counts[rook_from as usize] + 1;
        counts[rook_from as usize] = 0;
    }
}

pub fn run_in_parallel<Task, Outcome, Run>(tasks: &[Task], run: &Run) -> Vec<Outcome>
where
    Task: Sync,
    Outcome: Send,
    Run: Fn(&Task) -> Outcome + Sync,
{
    let workers = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1)
        .min(tasks.len().max(1));

    if workers <= 1 {
        return tasks.iter().map(run).collect();
    }

    let next = std::sync::atomic::AtomicUsize::new(0);
    let finished = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if index >= tasks.len() {
                            break done;
                        }
                        done.push((index, run(&tasks[index])));
                    }
                })
            })
            .collect();

        workers
            .into_iter()
            .map(|worker| worker.join().expect("a task panicked"))
            .collect::<Vec<_>>()
    });

    let mut ordered: Vec<Option<Outcome>> = (0..tasks.len()).map(|_| None).collect();
    for done in finished {
        for (index, outcome) in done {
            ordered[index] = Some(outcome);
        }
    }

    ordered
        .into_iter()
        .map(|outcome| outcome.expect("every task was run"))
        .collect()
}
