pub const EMPTY: i8 = 0;
pub const PAWN: i8 = 1;
pub const KNIGHT: i8 = 2;
pub const BISHOP: i8 = 3;
pub const ROOK: i8 = 4;
pub const QUEEN: i8 = 5;
pub const KING: i8 = 6;

pub const WHITE: i8 = 0;
pub const BLACK: i8 = 1;

const WHITE_KINGSIDE: u8 = 1;
const WHITE_QUEENSIDE: u8 = 2;
const BLACK_KINGSIDE: u8 = 4;
const BLACK_QUEENSIDE: u8 = 8;

pub const CAPTURES: u8 = 1;
pub const TAKES_EN_PASSANT: u8 = 2;
pub const CASTLES: u8 = 4;
pub const PROMOTES: u8 = 8;
pub const GIVES_CHECK: u8 = 16;

pub const FILES: i32 = 8;
pub const RANKS: i32 = 8;
pub const SQUARES: usize = 64;

const WHITE_KING_START: i32 = 4;
const BLACK_KING_START: i32 = 60;
const WHITE_KINGSIDE_ROOK: i32 = 7;
const WHITE_QUEENSIDE_ROOK: i32 = 0;
const BLACK_KINGSIDE_ROOK: i32 = 63;
const BLACK_QUEENSIDE_ROOK: i32 = 56;

const CASTLING_STEP: i32 = 2;

const DOUBLE_STEP: i32 = 16;

pub const FIFTY_MOVE_PLIES: i32 = 100;

pub const REPETITIONS_DRAWN: usize = 3;

pub const PIECE_VALUES: [i32; 7] = [0, 100, 320, 330, 500, 900, 20000];

const VICTIM_WEIGHT: i32 = 16;

const KNIGHT_STEPS: [(i32, i32); 8] = [
    (1, 2),
    (2, 1),
    (2, -1),
    (1, -2),
    (-1, -2),
    (-2, -1),
    (-2, 1),
    (-1, 2),
];
const KING_STEPS: [(i32, i32); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];
const ROOK_RAYS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const BISHOP_RAYS: [(i32, i32); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

pub fn file_of(square: i32) -> i32 {
    square % FILES
}

pub fn rank_of(square: i32) -> i32 {
    square / FILES
}

pub fn opponent(side: i8) -> i8 {
    1 - side
}

pub fn piece_value(piece: i8) -> i32 {
    PIECE_VALUES[piece.unsigned_abs() as usize]
}

pub fn king_moves_between(from: i32, to: i32) -> i32 {
    (file_of(from) - file_of(to))
        .abs()
        .max((rank_of(from) - rank_of(to)).abs())
}

fn step(square: i32, files: i32, ranks: i32) -> Option<i32> {
    let file = file_of(square) + files;
    let rank = rank_of(square) + ranks;

    ((0..FILES).contains(&file) && (0..RANKS).contains(&rank)).then_some(rank * FILES + file)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Move {
    pub from: u8,
    pub to: u8,
    pub promotion: i8,
    pub captured: i8,
    pub flags: u8,
}

impl Move {
    pub fn does(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub board: [i8; SQUARES],
    pub side: i8,
    pub castling: u8,
    pub en_passant: i8,
    pub halfmove_clock: i32,
    pub fullmove_number: i32,
}

impl Position {
    pub fn start() -> Position {
        Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .expect("the starting position is a valid FEN")
    }

    pub fn from_fen(fen: &str) -> Result<Position, String> {
        let fields: Vec<&str> = fen.split_whitespace().collect();
        if fields.len() < 4 {
            return Err(format!(
                "a FEN needs at least 4 fields, this has {}",
                fields.len()
            ));
        }

        let mut board = [EMPTY; SQUARES];
        let mut rank = RANKS - 1;
        let mut file = 0;
        for character in fields[0].chars() {
            match character {
                '/' => {
                    if file != FILES {
                        return Err(format!("FEN rank {} has {file} files", RANKS - rank));
                    }
                    rank -= 1;
                    file = 0;
                }
                '1'..='8' => file += character as i32 - '0' as i32,
                _ => {
                    let piece = match character.to_ascii_lowercase() {
                        'p' => PAWN,
                        'n' => KNIGHT,
                        'b' => BISHOP,
                        'r' => ROOK,
                        'q' => QUEEN,
                        'k' => KING,
                        _ => return Err(format!("'{character}' is not a FEN piece")),
                    };
                    if !(0..FILES).contains(&file) || !(0..RANKS).contains(&rank) {
                        return Err("the FEN board overflows 8x8".to_string());
                    }
                    board[(rank * FILES + file) as usize] = if character.is_ascii_uppercase() {
                        piece
                    } else {
                        -piece
                    };
                    file += 1;
                }
            }
        }

        let side = match fields[1] {
            "w" => WHITE,
            "b" => BLACK,
            other => return Err(format!("'{other}' is not a FEN side to move")),
        };

        let mut castling = 0;
        for character in fields[2].chars() {
            match character {
                'K' => castling |= WHITE_KINGSIDE,
                'Q' => castling |= WHITE_QUEENSIDE,
                'k' => castling |= BLACK_KINGSIDE,
                'q' => castling |= BLACK_QUEENSIDE,
                '-' => {}
                _ => return Err(format!("'{character}' is not a FEN castling right")),
            }
        }

        let en_passant = match fields[3] {
            "-" => -1,
            square => match square_index(square) {
                Some(index) => index as i8,
                None => return Err(format!("'{square}' is not a FEN en passant square")),
            },
        };

        Ok(Position {
            board,
            side,
            castling,
            en_passant,
            halfmove_clock: fields
                .get(4)
                .and_then(|text| text.parse().ok())
                .unwrap_or(0),
            fullmove_number: fields
                .get(5)
                .and_then(|text| text.parse().ok())
                .unwrap_or(1),
        })
    }

    pub fn owner(piece: i8) -> i8 {
        if piece > 0 { WHITE } else { BLACK }
    }

    pub fn attacked(&self, square: i32, side: i8) -> bool {
        self.scan_attackers(square, side, true) > 0
    }

    pub fn count_attackers(&self, square: i32, side: i8) -> i32 {
        self.scan_attackers(square, side, false)
    }

    fn scan_attackers(&self, square: i32, side: i8, first_only: bool) -> i32 {
        let sign: i8 = if side == WHITE { 1 } else { -1 };
        let mut count = 0;

        let pawn_rank = if side == WHITE { -1 } else { 1 };
        for files in [-1, 1] {
            if let Some(from) = step(square, files, pawn_rank)
                && self.board[from as usize] == sign * PAWN
            {
                count += 1;
                if first_only {
                    return count;
                }
            }
        }

        for (steps, kind) in [(KNIGHT_STEPS, KNIGHT), (KING_STEPS, KING)] {
            for (files, ranks) in steps {
                if let Some(from) = step(square, files, ranks)
                    && self.board[from as usize] == sign * kind
                {
                    count += 1;
                    if first_only {
                        return count;
                    }
                }
            }
        }

        for (rays, slider) in [(ROOK_RAYS, ROOK), (BISHOP_RAYS, BISHOP)] {
            for (files, ranks) in rays {
                let mut current = square;
                while let Some(next) = step(current, files, ranks) {
                    current = next;
                    let piece = self.board[next as usize];
                    if piece == EMPTY {
                        continue;
                    }
                    if piece == sign * slider || piece == sign * QUEEN {
                        count += 1;
                        if first_only {
                            return count;
                        }
                    }
                    break;
                }
            }
        }

        count
    }

    pub fn king_square(&self, side: i8) -> Option<i32> {
        let king = if side == WHITE { KING } else { -KING };

        self.board
            .iter()
            .position(|&piece| piece == king)
            .map(|square| square as i32)
    }

    pub fn in_check(&self, side: i8) -> bool {
        match self.king_square(side) {
            Some(square) => self.attacked(square, opponent(side)),
            None => false,
        }
    }

    pub fn make(&mut self, played: Move) {
        let from = played.from as usize;
        let to = played.to as usize;
        let piece = self.board[from];
        let moving = self.side;

        self.board[from] = EMPTY;
        self.board[to] = match (played.does(PROMOTES), moving) {
            (true, WHITE) => played.promotion,
            (true, _) => -played.promotion,
            (false, _) => piece,
        };

        if played.does(TAKES_EN_PASSANT) {
            self.board[captured_pawn_square(from, to)] = EMPTY;
        }

        if played.does(CASTLES) {
            let (rook_from, rook_to) = castling_rook(played.to as i32);
            self.board[rook_to as usize] = self.board[rook_from as usize];
            self.board[rook_from as usize] = EMPTY;
        }

        for square in [from as i32, to as i32] {
            self.castling &= match square {
                WHITE_QUEENSIDE_ROOK => !WHITE_QUEENSIDE,
                WHITE_KINGSIDE_ROOK => !WHITE_KINGSIDE,
                BLACK_QUEENSIDE_ROOK => !BLACK_QUEENSIDE,
                BLACK_KINGSIDE_ROOK => !BLACK_KINGSIDE,
                _ => u8::MAX,
            };
        }
        if piece.abs() == KING {
            self.castling &= if moving == WHITE {
                !(WHITE_KINGSIDE | WHITE_QUEENSIDE)
            } else {
                !(BLACK_KINGSIDE | BLACK_QUEENSIDE)
            };
        }

        let distance = played.to as i32 - played.from as i32;
        self.en_passant = if piece.abs() == PAWN && distance.abs() == DOUBLE_STEP {
            ((played.from as i32 + played.to as i32) / 2) as i8
        } else {
            -1
        };

        if piece.abs() == PAWN || played.does(CAPTURES) {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }
        if moving == BLACK {
            self.fullmove_number += 1;
        }
        self.side = opponent(moving);
    }

    fn push_pawn_move(&self, moves: &mut Vec<Move>, from: i32, to: i32, flags: u8, captured: i8) {
        let last_rank = if self.side == WHITE { RANKS - 1 } else { 0 };

        if rank_of(to) != last_rank {
            moves.push(Move {
                from: from as u8,
                to: to as u8,
                promotion: 0,
                captured,
                flags,
            });
            return;
        }

        for promotion in [QUEEN, ROOK, BISHOP, KNIGHT] {
            moves.push(Move {
                from: from as u8,
                to: to as u8,
                promotion,
                captured,
                flags: flags | PROMOTES,
            });
        }
    }

    fn pseudo_legal(&self, moves: &mut Vec<Move>) {
        let side = self.side;
        let sign: i8 = if side == WHITE { 1 } else { -1 };

        for from in 0..SQUARES as i32 {
            let piece = self.board[from as usize];
            if piece == EMPTY || Position::owner(piece) != side {
                continue;
            }

            match piece.abs() {
                PAWN => self.pawn_moves(moves, from),
                KNIGHT | KING => {
                    let steps = if piece.abs() == KNIGHT {
                        KNIGHT_STEPS
                    } else {
                        KING_STEPS
                    };
                    for (files, ranks) in steps {
                        let Some(to) = step(from, files, ranks) else {
                            continue;
                        };
                        let occupant = self.board[to as usize];
                        if occupant != EMPTY && Position::owner(occupant) == side {
                            continue;
                        }
                        moves.push(Move {
                            from: from as u8,
                            to: to as u8,
                            promotion: 0,
                            captured: occupant.abs(),
                            flags: if occupant == EMPTY { 0 } else { CAPTURES },
                        });
                    }
                }
                BISHOP | ROOK | QUEEN => {
                    let rays: &[(i32, i32)] = match piece.abs() {
                        BISHOP => &BISHOP_RAYS,
                        ROOK => &ROOK_RAYS,
                        _ => &KING_STEPS,
                    };
                    for &(files, ranks) in rays {
                        let mut current = from;
                        while let Some(to) = step(current, files, ranks) {
                            current = to;
                            let occupant = self.board[to as usize];
                            if occupant != EMPTY && Position::owner(occupant) == side {
                                break;
                            }
                            moves.push(Move {
                                from: from as u8,
                                to: to as u8,
                                promotion: 0,
                                captured: occupant.abs(),
                                flags: if occupant == EMPTY { 0 } else { CAPTURES },
                            });
                            if occupant != EMPTY {
                                break;
                            }
                        }
                    }
                }
                _ => unreachable!("unknown piece kind"),
            }
        }

        self.castling_moves(moves, sign);
    }

    fn pawn_moves(&self, moves: &mut Vec<Move>, from: i32) {
        let side = self.side;
        let forward = if side == WHITE { 1 } else { -1 };
        let start_rank = if side == WHITE { 1 } else { RANKS - 2 };

        if let Some(ahead) = step(from, 0, forward)
            && self.board[ahead as usize] == EMPTY
        {
            self.push_pawn_move(moves, from, ahead, 0, 0);

            if rank_of(from) == start_rank
                && let Some(two_ahead) = step(from, 0, 2 * forward)
                && self.board[two_ahead as usize] == EMPTY
            {
                moves.push(Move {
                    from: from as u8,
                    to: two_ahead as u8,
                    promotion: 0,
                    captured: 0,
                    flags: 0,
                });
            }
        }

        for files in [-1, 1] {
            let Some(to) = step(from, files, forward) else {
                continue;
            };
            let occupant = self.board[to as usize];

            if occupant != EMPTY && Position::owner(occupant) != side {
                self.push_pawn_move(moves, from, to, CAPTURES, occupant.abs());
            } else if occupant == EMPTY && self.en_passant >= 0 && to as i8 == self.en_passant {
                moves.push(Move {
                    from: from as u8,
                    to: to as u8,
                    promotion: 0,
                    captured: PAWN,
                    flags: CAPTURES | TAKES_EN_PASSANT,
                });
            }
        }
    }

    fn castling_moves(&self, moves: &mut Vec<Move>, sign: i8) {
        let side = self.side;
        let (king_square, kingside, queenside, kingside_rook, queenside_rook) = if side == WHITE {
            (
                WHITE_KING_START,
                WHITE_KINGSIDE,
                WHITE_QUEENSIDE,
                WHITE_KINGSIDE_ROOK,
                WHITE_QUEENSIDE_ROOK,
            )
        } else {
            (
                BLACK_KING_START,
                BLACK_KINGSIDE,
                BLACK_QUEENSIDE,
                BLACK_KINGSIDE_ROOK,
                BLACK_QUEENSIDE_ROOK,
            )
        };

        if self.board[king_square as usize] != sign * KING
            || self.attacked(king_square, opponent(side))
        {
            return;
        }

        let empty = |square: i32| self.board[square as usize] == EMPTY;

        if self.castling & kingside != 0
            && self.board[kingside_rook as usize] == sign * ROOK
            && empty(king_square + 1)
            && empty(king_square + 2)
            && !self.attacked(king_square + 1, opponent(side))
        {
            moves.push(Move {
                from: king_square as u8,
                to: (king_square + CASTLING_STEP) as u8,
                promotion: 0,
                captured: 0,
                flags: CASTLES,
            });
        }

        if self.castling & queenside != 0
            && self.board[queenside_rook as usize] == sign * ROOK
            && empty(king_square - 1)
            && empty(king_square - 2)
            && empty(king_square - 3)
            && !self.attacked(king_square - 1, opponent(side))
        {
            moves.push(Move {
                from: king_square as u8,
                to: (king_square - CASTLING_STEP) as u8,
                promotion: 0,
                captured: 0,
                flags: CASTLES,
            });
        }
    }

    pub fn legal_moves(&self) -> Vec<Move> {
        let mut pseudo = Vec::with_capacity(SQUARES);
        self.pseudo_legal(&mut pseudo);

        let mut legal = Vec::with_capacity(pseudo.len());
        for mut played in pseudo {
            let mut next = *self;
            next.make(played);
            if next.in_check(self.side) {
                continue;
            }
            if next.in_check(opponent(self.side)) {
                played.flags |= GIVES_CHECK;
            }
            legal.push(played);
        }

        legal
    }

    pub fn legal_replies(&self) -> Vec<Move> {
        let mut pseudo = Vec::with_capacity(SQUARES);
        self.pseudo_legal(&mut pseudo);

        let king = self.king_square(self.side).unwrap_or(-1);
        let mut legal = Vec::with_capacity(pseudo.len());
        for played in pseudo {
            let mut next = *self;
            next.make(played);
            let king_now = if played.from as i32 == king {
                played.to as i32
            } else {
                king
            };
            if !next.attacked(king_now, opponent(self.side)) {
                legal.push(played);
            }
        }

        legal
    }

    pub fn pseudo_legal_count(&self, side: i8) -> i32 {
        let mut probe = *self;
        probe.side = side;

        let mut moves = Vec::with_capacity(SQUARES);
        probe.pseudo_legal(&mut moves);

        moves.len() as i32
    }

    pub fn repeats(&self, other: &Position) -> bool {
        self.board == other.board
            && self.side == other.side
            && self.castling == other.castling
            && self.en_passant == other.en_passant
    }

    pub fn insufficient_material(&self) -> bool {
        let mut minors = Vec::new();

        for (square, &piece) in self.board.iter().enumerate() {
            match piece.abs() {
                EMPTY | KING => {}
                KNIGHT | BISHOP => minors.push((piece, square as i32)),
                _ => return false,
            }
        }

        match minors.len() {
            0 | 1 => true,
            2 => {
                let (first, first_square) = minors[0];
                let (second, second_square) = minors[1];
                first.abs() == BISHOP
                    && second.abs() == BISHOP
                    && Position::owner(first) != Position::owner(second)
                    && is_light(first_square) == is_light(second_square)
            }
            _ => false,
        }
    }
}

pub fn is_light(square: i32) -> bool {
    (file_of(square) + rank_of(square)) % 2 == 1
}

fn captured_pawn_square(from: usize, to: usize) -> usize {
    if to > from { to - 8 } else { to + 8 }
}

pub fn castling_rook(to: i32) -> (i32, i32) {
    match to {
        6 => (WHITE_KINGSIDE_ROOK, 5),
        2 => (WHITE_QUEENSIDE_ROOK, 3),
        62 => (BLACK_KINGSIDE_ROOK, 61),
        58 => (BLACK_QUEENSIDE_ROOK, 59),
        _ => unreachable!("invalid castling target square"),
    }
}

fn square_index(name: &str) -> Option<usize> {
    let bytes = name.as_bytes();
    if bytes.len() != 2 {
        return None;
    }

    let file = bytes[0] as i32 - 'a' as i32;
    let rank = bytes[1] as i32 - '1' as i32;

    ((0..FILES).contains(&file) && (0..RANKS).contains(&rank))
        .then_some((rank * FILES + file) as usize)
}

pub fn material_for(position: &Position, side: i8) -> i32 {
    let mut balance = 0;

    for &piece in &position.board {
        if piece == EMPTY || piece.abs() == KING {
            continue;
        }
        let worth = piece_value(piece);
        balance += if piece > 0 { worth } else { -worth };
    }

    if side == WHITE { balance } else { -balance }
}

fn nearest_slider(
    board: &[i8; SQUARES],
    square: i32,
    rays: &[(i32, i32)],
    sign: i8,
    wanted: i8,
) -> Option<usize> {
    for &(files, ranks) in rays {
        let mut current = square;
        while let Some(next) = step(current, files, ranks) {
            current = next;
            let piece = board[next as usize];
            if piece == EMPTY {
                continue;
            }
            if piece == sign * wanted {
                return Some(next as usize);
            }
            break;
        }
    }

    None
}

fn cheapest_attacker(board: &[i8; SQUARES], square: i32, side: i8) -> Option<usize> {
    let sign: i8 = if side == WHITE { 1 } else { -1 };

    let pawn_rank = if side == WHITE { -1 } else { 1 };
    for files in [-1, 1] {
        if let Some(from) = step(square, files, pawn_rank)
            && board[from as usize] == sign * PAWN
        {
            return Some(from as usize);
        }
    }

    for (files, ranks) in KNIGHT_STEPS {
        if let Some(from) = step(square, files, ranks)
            && board[from as usize] == sign * KNIGHT
        {
            return Some(from as usize);
        }
    }

    for (rays, wanted) in [
        (&BISHOP_RAYS, BISHOP),
        (&ROOK_RAYS, ROOK),
        (&BISHOP_RAYS, QUEEN),
        (&ROOK_RAYS, QUEEN),
    ] {
        if let Some(from) = nearest_slider(board, square, rays, sign, wanted) {
            return Some(from);
        }
    }

    for (files, ranks) in KING_STEPS {
        if let Some(from) = step(square, files, ranks)
            && board[from as usize] == sign * KING
        {
            return Some(from as usize);
        }
    }

    None
}

pub fn static_exchange(position: &Position, played: Move) -> i32 {
    let mover = position.side;
    let to = played.to as usize;
    let from = played.from as usize;
    let mut board = position.board;

    let captured = if played.does(TAKES_EN_PASSANT) {
        PIECE_VALUES[PAWN as usize]
    } else if board[to] != EMPTY {
        piece_value(board[to])
    } else {
        0
    };

    let mut standing = piece_value(board[from]);
    board[to] = board[from];
    board[from] = EMPTY;
    if played.does(TAKES_EN_PASSANT) {
        board[captured_pawn_square(from, to)] = EMPTY;
    }

    let mut gains = vec![captured];
    let mut side = opponent(mover);
    while let Some(attacker) = cheapest_attacker(&board, to as i32, side) {
        let depth = gains.len();
        gains.push(standing - gains[depth - 1]);
        standing = piece_value(board[attacker]);
        board[to] = board[attacker];
        board[attacker] = EMPTY;
        side = opponent(side);
    }

    let mut depth = gains.len();
    while depth > 1 {
        depth -= 1;
        gains[depth - 1] = -(-gains[depth - 1]).max(gains[depth]);
    }

    gains[0]
}

pub fn order_by_capture_value(moves: &mut [Move], board: &[i8; SQUARES]) {
    moves.sort_by_key(|played| {
        let victim = piece_value(played.captured);
        let attacker = piece_value(board[played.from as usize]);
        let promotion = if played.does(PROMOTES) {
            piece_value(played.promotion)
        } else {
            0
        };

        -(victim * VICTIM_WEIGHT - attacker + promotion)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perft(position: &Position, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }

        let moves = position.legal_moves();
        if depth == 1 {
            return moves.len() as u64;
        }

        moves
            .into_iter()
            .map(|played| {
                let mut next = *position;
                next.make(played);
                perft(&next, depth - 1)
            })
            .sum()
    }

    const PERFT_SUITE: [(&str, &[u64]); 6] = [
        (
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            &[20, 400, 8902, 197281, 4865609],
        ),
        (
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            &[48, 2039, 97862, 4085603],
        ),
        (
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            &[14, 191, 2812, 43238, 674624],
        ),
        (
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            &[6, 264, 9467, 422333],
        ),
        (
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 0 1",
            &[44, 1486, 62379, 2103487],
        ),
        (
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 1",
            &[46, 2079, 89890, 3894594],
        ),
    ];

    fn check_perft(depth: usize) {
        for (fen, expected) in PERFT_SUITE {
            let position = Position::from_fen(fen).expect("the test FEN is valid");
            for (index, &leaves) in expected.iter().take(depth).enumerate() {
                assert_eq!(
                    perft(&position, index as u32 + 1),
                    leaves,
                    "at depth {} of {fen}",
                    index + 1
                );
            }
        }
    }

    #[test]
    fn the_move_generator_reaches_the_published_counts() {
        check_perft(3);
    }

    #[test]
    #[ignore]
    fn the_move_generator_reaches_the_published_counts_deeply() {
        check_perft(5);
    }

    #[test]
    fn a_capture_of_a_defended_piece_is_worth_the_exchange() {
        let position = Position::from_fen("4k3/3p4/8/8/8/8/8/3RK3 w - - 0 1")
            .expect("the position is a valid FEN");
        let takes_pawn = position
            .legal_moves()
            .into_iter()
            .find(|played| played.to == 51 && played.captured == PAWN)
            .expect("the rook can take the pawn");

        assert_eq!(
            static_exchange(&position, takes_pawn),
            PIECE_VALUES[PAWN as usize] - PIECE_VALUES[ROOK as usize],
            "the king recaptures the rook, so the exchange loses a rook for a pawn"
        );
    }

    #[test]
    fn an_undefended_capture_is_worth_the_captured_piece() {
        let position = Position::from_fen("4k3/8/8/8/8/3q4/8/3RK3 w - - 0 1")
            .expect("the position is a valid FEN");
        let takes_queen = position
            .legal_moves()
            .into_iter()
            .find(|played| played.captured == QUEEN)
            .expect("the rook can take the queen");

        assert_eq!(
            static_exchange(&position, takes_queen),
            PIECE_VALUES[QUEEN as usize]
        );
    }

    #[test]
    fn castling_moves_the_rook() {
        let mut position = Position::from_fen("4k3/8/8/8/8/8/8/4K2R w K - 0 1")
            .expect("the position is a valid FEN");
        let castles = position
            .legal_moves()
            .into_iter()
            .find(|played| played.does(CASTLES))
            .expect("white may castle kingside");

        position.make(castles);

        assert_eq!(position.board[6], KING);
        assert_eq!(position.board[5], ROOK);
        assert_eq!(position.board[7], EMPTY);
        assert_eq!(position.castling, 0, "castling clears the castling rights");
    }

    #[test]
    fn a_position_with_two_kings_is_a_draw_by_material() {
        let position = Position::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1")
            .expect("the position is a valid FEN");

        assert!(position.insufficient_material());
    }
}
