use crate::chess_vm::assembler::{Instruction, Opcode, Program, REGISTERS};
use crate::chess_vm::board::{
    self, CAPTURES, CASTLES, GIVES_CHECK, KING, Move, PROMOTES, Position, SQUARES, WHITE,
};

const SCORE_LIMIT: i32 = 500_000_000;

const INFINITY: i32 = 1_000_000_000;

const MATE_VALUE: i32 = 900_000_000;

#[derive(Debug)]
pub enum Failure {
    Fault(String),
    Overrun,
}

struct MoveContext<'a> {
    position: &'a Position,
    played: Move,
    mover: i8,
    move_counts: &'a [i32; SQUARES],
    ply: i32,
    after: Position,
    replies: Option<Vec<Move>>,
    mobility: Option<i32>,
}

impl<'a> MoveContext<'a> {
    fn new(
        position: &'a Position,
        played: Move,
        move_counts: &'a [i32; SQUARES],
        ply: i32,
    ) -> MoveContext<'a> {
        let mut after = *position;
        after.make(played);

        MoveContext {
            position,
            played,
            mover: position.side,
            move_counts,
            ply,
            after,
            replies: None,
            mobility: None,
        }
    }

    fn replies(&mut self) -> &[Move] {
        if self.replies.is_none() {
            self.replies = Some(self.after.legal_replies());
        }

        self.replies
            .as_ref()
            .expect("the replies were just counted")
    }

    fn material(&self) -> i32 {
        board::material_for(&self.after, self.mover)
    }

    fn gain(&self) -> i32 {
        board::piece_value(self.played.captured)
    }

    fn mobility(&mut self) -> i32 {
        if self.mobility.is_none() {
            self.mobility = Some(self.after.pseudo_legal_count(self.mover));
        }

        self.mobility.expect("the mobility was just counted")
    }

    fn opponent_mobility(&mut self) -> i32 {
        self.replies().len() as i32
    }

    fn mate_in_one(&mut self) -> i32 {
        let mated = self.replies().is_empty() && self.after.in_check(self.after.side);
        mated as i32
    }

    fn king_distance(&self) -> i32 {
        match (
            self.after.king_square(WHITE),
            self.after.king_square(board::BLACK),
        ) {
            (Some(white), Some(black)) => board::king_moves_between(white, black),
            _ => 0,
        }
    }

    fn summed_king_distance(&self, side: i8) -> i32 {
        let Some(king) = self.after.king_square(side) else {
            return 0;
        };

        let mut total = 0;
        for (square, &piece) in self.after.board.iter().enumerate() {
            if piece == board::EMPTY || piece.abs() == KING {
                continue;
            }
            if Position::owner(piece) == self.mover {
                total += board::king_moves_between(square as i32, king);
            }
        }

        total
    }

    fn exposed(&self) -> i32 {
        let opponent = board::opponent(self.mover);
        let mut worst = 0;

        for (square, &piece) in self.after.board.iter().enumerate() {
            if piece == board::EMPTY || Position::owner(piece) != self.mover {
                continue;
            }
            if self.after.attacked(square as i32, opponent) {
                worst = worst.max(board::piece_value(piece));
            }
        }

        worst
    }

    fn light_count(&self) -> i32 {
        let mut count = 0;

        for (square, &piece) in self.after.board.iter().enumerate() {
            if piece != board::EMPTY
                && Position::owner(piece) == self.mover
                && board::is_light(square as i32)
            {
                count += 1;
            }
        }

        count
    }

    fn attackers(&self, square: i32) -> i32 {
        self.after
            .count_attackers(square, board::opponent(self.mover))
    }

    fn defenders(&self, square: i32) -> i32 {
        self.after.count_attackers(square, self.mover)
    }

    fn piece_at(&self, square: i32) -> i32 {
        let piece = self.after.board[square as usize] as i32;

        if self.mover == WHITE { piece } else { -piece }
    }

    fn to_rank(&self) -> i32 {
        let rank = board::rank_of(self.played.to as i32);

        if self.mover == WHITE {
            rank
        } else {
            board::RANKS - 1 - rank
        }
    }

    fn center(&self) -> i32 {
        let square = self.played.to as i32;
        let file = board::file_of(square);
        let rank = board::rank_of(square);

        file.min(board::FILES - 1 - file) + rank.min(board::RANKS - 1 - rank)
    }

    fn safe(&self) -> i32 {
        let to = self.played.to as i32;
        let attackers = self.attackers(to);
        let defenders = self.defenders(to);

        (attackers == 0 || defenders >= attackers) as i32
    }

    fn static_exchange(&self) -> i32 {
        board::static_exchange(self.position, self.played)
    }
}

fn run(
    program: &Program,
    context: &mut MoveContext,
    budget: u64,
    cycles: &mut u64,
) -> Result<i32, Failure> {
    let mut registers = [0i32; REGISTERS];
    let mut score = 0i32;
    let mut counter = 0usize;

    loop {
        let Some(instruction) = program.code().get(counter).copied() else {
            return Err(Failure::Fault(format!(
                "ran past the last instruction (at {counter}) without a HALT"
            )));
        };

        *cycles += instruction.cycles;
        if *cycles > budget {
            return Err(Failure::Overrun);
        }
        counter += 1;

        let left = registers[instruction.left];
        let right = registers[instruction.right];

        let written = match instruction.opcode {
            Opcode::Halt => return Ok(score.clamp(-SCORE_LIMIT, SCORE_LIMIT)),
            Opcode::Score => {
                score = left;
                None
            }
            Opcode::Jump
            | Opcode::JumpIfZero
            | Opcode::JumpIfNotZero
            | Opcode::JumpIfLess
            | Opcode::JumpIfLessOrEqual
            | Opcode::JumpIfGreater
            | Opcode::JumpIfGreaterOrEqual => {
                if jumps(instruction.opcode, left, right) {
                    counter = jump_target(&instruction, program)?;
                }
                None
            }
            _ => Some(value(&instruction, program, context, left, right)?),
        };

        if let Some(value) = written {
            registers[instruction.destination] = value;
        }
    }
}

fn jumps(opcode: Opcode, left: i32, right: i32) -> bool {
    match opcode {
        Opcode::Jump => true,
        Opcode::JumpIfZero => left == 0,
        Opcode::JumpIfNotZero => left != 0,
        Opcode::JumpIfLess => left < right,
        Opcode::JumpIfLessOrEqual => left <= right,
        Opcode::JumpIfGreater => left > right,
        Opcode::JumpIfGreaterOrEqual => left >= right,
        _ => unreachable!("only the jumps ask whether they are taken"),
    }
}

fn jump_target(instruction: &Instruction, program: &Program) -> Result<usize, Failure> {
    let target = instruction.immediate;

    if target < 0 || target as usize >= program.code().len() {
        return Err(Failure::Fault(format!(
            "line {}: jumped to instruction {target}, which is outside the code",
            instruction.line
        )));
    }

    Ok(target as usize)
}

fn value(
    instruction: &Instruction,
    program: &Program,
    context: &mut MoveContext,
    left: i32,
    right: i32,
) -> Result<i32, Failure> {
    let line = instruction.line;
    let played = context.played;

    let value = match instruction.opcode {
        Opcode::LoadImmediate => instruction.immediate,
        Opcode::Move => left,
        Opcode::Add => left.wrapping_add(right),
        Opcode::Subtract => left.wrapping_sub(right),
        Opcode::Multiply => left.wrapping_mul(right),
        Opcode::Divide => {
            if right == 0 {
                0
            } else {
                left.wrapping_div(right)
            }
        }
        Opcode::Modulo => {
            if right == 0 {
                0
            } else {
                left.wrapping_rem(right)
            }
        }
        Opcode::Minimum => left.min(right),
        Opcode::Maximum => left.max(right),
        Opcode::AddImmediate => left.wrapping_add(instruction.immediate),
        Opcode::MultiplyImmediate => left.wrapping_mul(instruction.immediate),
        Opcode::Negate => left.wrapping_neg(),
        Opcode::LoadData => {
            let address = left.wrapping_add(instruction.immediate);
            if address < 0 || address as usize >= program.data().len() {
                return Err(Failure::Fault(format!(
                    "line {line}: LD read data address {address}, outside 0..{}",
                    program.data().len()
                )));
            }
            program.data()[address as usize]
        }
        Opcode::Material => context.material(),
        Opcode::Gain => context.gain(),
        Opcode::Mobility => context.mobility(),
        Opcode::OpponentMobility => context.opponent_mobility(),
        Opcode::GivesCheck => played.does(GIVES_CHECK) as i32,
        Opcode::IsCapture => played.does(CAPTURES) as i32,
        Opcode::IsCastle => played.does(CASTLES) as i32,
        Opcode::IsPromotion => played.does(PROMOTES) as i32,
        Opcode::Promotion => played.promotion as i32,
        Opcode::MateInOne => context.mate_in_one(),
        Opcode::KingDistance => context.king_distance(),
        Opcode::OwnKingDistance => context.summed_king_distance(context.mover),
        Opcode::EnemyKingDistance => context.summed_king_distance(board::opponent(context.mover)),
        Opcode::Exposed => context.exposed(),
        Opcode::LightCount => context.light_count(),
        Opcode::Attackers => context.attackers(square(left, "ATTACKERS", line)?),
        Opcode::Defenders => context.defenders(square(left, "DEFENDERS", line)?),
        Opcode::PieceAt => context.piece_at(square(left, "PIECEAT", line)?),
        Opcode::MoveCount => context.move_counts[square(left, "MOVECOUNT", line)? as usize],
        Opcode::FromSquare => played.from as i32,
        Opcode::ToSquare => played.to as i32,
        Opcode::ToRank => context.to_rank(),
        Opcode::MovedPiece => context.position.board[played.from as usize].unsigned_abs() as i32,
        Opcode::Center => context.center(),
        Opcode::StaticExchange => context.static_exchange(),
        Opcode::Hangs => (context.static_exchange() < 0) as i32,
        Opcode::Safe => context.safe(),
        Opcode::Ply => context.ply,
        Opcode::Score | Opcode::Halt => unreachable!("SCORE and HALT write no register"),
        Opcode::Jump
        | Opcode::JumpIfZero
        | Opcode::JumpIfNotZero
        | Opcode::JumpIfLess
        | Opcode::JumpIfLessOrEqual
        | Opcode::JumpIfGreater
        | Opcode::JumpIfGreaterOrEqual => unreachable!("a jump writes no register"),
    };

    Ok(value)
}

fn square(value: i32, instruction: &str, line: usize) -> Result<i32, Failure> {
    if (0..SQUARES as i32).contains(&value) {
        return Ok(value);
    }

    Err(Failure::Fault(format!(
        "line {line}: {instruction} was given square {value}, outside 0..{SQUARES}"
    )))
}

pub fn choose_move(
    program: &Program,
    position: &Position,
    legal: &[Move],
    random: &mut super::playout::Random,
    move_counts: &[i32; SQUARES],
    ply: i32,
) -> (usize, Option<Failure>) {
    let budget = program.budget();
    let mut cycles = 0u64;
    let mut scores = Vec::with_capacity(legal.len());

    for &played in legal {
        let scored = if program.search_depth() <= 1 {
            let mut context = MoveContext::new(position, played, move_counts, ply);
            run(program, &mut context, budget, &mut cycles)
        } else {
            let mut after = *position;
            after.make(played);
            search_value(
                program,
                &after,
                program.search_depth() - 1,
                -INFINITY,
                INFINITY,
                budget,
                &mut cycles,
                move_counts,
                ply,
            )
            .map(|value| -value)
        };

        match scored {
            Ok(score) => scores.push(score.clamp(-SCORE_LIMIT, SCORE_LIMIT)),
            Err(failure) => return (0, Some(failure)),
        }
    }

    let best = *scores.iter().max().expect("the position has legal moves");
    let winners: Vec<usize> = scores
        .iter()
        .enumerate()
        .filter(|&(_, &score)| score == best)
        .map(|(index, _)| index)
        .collect();

    (winners[random.below(winners.len())], None)
}

#[allow(clippy::too_many_arguments)]
fn search_value(
    program: &Program,
    position: &Position,
    depth: i32,
    mut alpha: i32,
    beta: i32,
    budget: u64,
    cycles: &mut u64,
    move_counts: &[i32; SQUARES],
    ply: i32,
) -> Result<i32, Failure> {
    if position.halfmove_clock >= board::FIFTY_MOVE_PLIES || position.insufficient_material() {
        return Ok(0);
    }

    let mut moves = position.legal_replies();
    if moves.is_empty() {
        return Ok(if position.in_check(position.side) {
            -MATE_VALUE
        } else {
            0
        });
    }
    board::order_by_capture_value(&mut moves, &position.board);

    let mut best = -INFINITY;
    for played in moves {
        let value = if depth <= 1 {
            let mut context = MoveContext::new(position, played, move_counts, ply);
            run(program, &mut context, budget, cycles)?
        } else {
            let mut after = *position;
            after.make(played);
            -search_value(
                program,
                &after,
                depth - 1,
                -beta,
                -alpha,
                budget,
                cycles,
                move_counts,
                ply,
            )?
        };

        best = best.max(value);
        alpha = alpha.max(best);
        if alpha >= beta {
            break;
        }
    }

    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess_vm::assembler::{Author, assemble};

    fn score(source: &str, fen: &str, wanted: impl Fn(&Move) -> bool) -> Result<i32, Failure> {
        let program = assemble(source, Author::Agent).expect("the program assembles");
        let position = Position::from_fen(fen).expect("the position is a valid FEN");
        let played = position
            .legal_moves()
            .into_iter()
            .find(|played| wanted(played))
            .expect("the position has the move the test scores");

        let move_counts = [0i32; SQUARES];
        let mut context = MoveContext::new(&position, played, &move_counts, 0);

        run(&program, &mut context, program.budget(), &mut 0)
    }

    fn scored_in_the_starting_position(source: &str) -> Result<i32, Failure> {
        score(
            source,
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            |_| true,
        )
    }

    #[test]
    fn a_capture_is_worth_the_captured_piece() {
        let taken = score(
            "    gain r0\n    score r0\n    halt\n",
            "4k3/8/8/8/8/3q4/8/3RK3 w - - 0 1",
            |played| played.captured == board::QUEEN,
        );

        assert_eq!(taken.ok(), Some(board::PIECE_VALUES[board::QUEEN as usize]));
    }

    #[test]
    fn matein1_detects_a_mate_in_one() {
        let mate = score(
            "    matein1 r0\n    score r0\n    halt\n",
            "rnbqkbnr/pppp1ppp/8/4p3/6P1/5P2/PPPPP2P/RNBQKBNR b KQkq - 0 2",
            |played| played.from == 59 && played.to == 31,
        );

        assert_eq!(mate.ok(), Some(1), "Qd8h4 mates");
    }

    #[test]
    fn moving_into_a_losing_exchange_scores_the_loss() {
        let source = "    see r0\n    hangs r1\n    muli r1, r1, -1\n    add r0, r0, r1\n    score r0\n    halt\n";
        let lost = score(source, "4k3/3p4/8/8/8/8/8/3RK3 w - - 0 1", |played| {
            played.captured == board::PAWN
        })
        .expect("the program runs");

        let expected = board::PIECE_VALUES[board::PAWN as usize]
            - board::PIECE_VALUES[board::ROOK as usize]
            - 1;
        assert_eq!(lost, expected, "the king recaptures, so the rook is lost");
    }

    #[test]
    fn the_score_of_a_program_that_never_scores_is_zero() {
        assert_eq!(scored_in_the_starting_position("    halt\n").ok(), Some(0));
    }

    #[test]
    fn a_fault_names_the_line_it_is_on() {
        for (source, expected) in [
            ("    ldi r0, 1\n", "ran past the last instruction"),
            ("    jmp done\ndone:\n", "line 1: jumped to instruction 1"),
            (
                "    ldi r0, 5\n    ld r1, r0, 0\n    halt\n",
                "line 2: LD read data address 5",
            ),
            (
                "    ldi r0, 64\n    pieceat r1, r0\n    halt\n",
                "line 2: PIECEAT was given square 64",
            ),
            (
                "    ldi r0, -1\n    movecount r1, r0\n    halt\n",
                "line 2: MOVECOUNT was given square -1",
            ),
        ] {
            match scored_in_the_starting_position(source) {
                Err(Failure::Fault(message)) => assert!(
                    message.starts_with(expected),
                    "expected {expected:?}, got {message:?}"
                ),
                _ => panic!("the machine ran an invalid program: {source}"),
            }
        }
    }

    #[test]
    fn a_program_that_never_halts_runs_out_of_cycles() {
        let looping = scored_in_the_starting_position("here:\n    jmp here\n");

        assert!(
            matches!(looping, Err(Failure::Overrun)),
            "the cycle budget stops an endless loop"
        );
    }

    #[test]
    fn the_highest_scoring_move_is_played() {
        let program = assemble("    gain r0\n    score r0\n    halt\n", Author::Agent)
            .expect("the program assembles");
        let position = Position::from_fen("4k3/8/8/8/8/3q4/8/3RK3 w - - 0 1")
            .expect("the position is a valid FEN");
        let legal = position.legal_moves();

        let (chosen, failure) = choose_move(
            &program,
            &position,
            &legal,
            &mut crate::chess_vm::playout::Random::new(1),
            &[0i32; SQUARES],
            0,
        );

        assert!(failure.is_none());
        assert_eq!(legal[chosen].captured, board::QUEEN);
    }

    #[test]
    fn a_faulting_program_plays_the_first_move_and_reports_the_fault() {
        let program = assemble("    ldi r0, 1\n", Author::Agent).expect("the program assembles");
        let position = Position::start();
        let legal = position.legal_moves();

        let (chosen, failure) = choose_move(
            &program,
            &position,
            &legal,
            &mut crate::chess_vm::playout::Random::new(1),
            &[0i32; SQUARES],
            0,
        );

        assert_eq!(chosen, 0);
        assert!(matches!(failure, Some(Failure::Fault(_))));
    }
}
