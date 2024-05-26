use crate::board::bitboard::{
    A_FILE, B_FILE, EMPTY, G_FILE, H_FILE, RANK_1, RANK_4, RANK_5, RANK_8,
};
use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::square::*;
use crate::board::Board;
use log::debug;
use rustc_hash::FxHashMap;

use super::ray_table::{Direction, RayTable};

pub type PieceTarget = (u64, u64); // (piece_square, targets)

pub struct Targets {
    kings: [u64; 64],
    knights: [u64; 64],
    rays: RayTable,
    attacks_cache: FxHashMap<(u8, u64), u64>,
}

const ROOK_DIRS: [Direction; 4] = [
    Direction::North,
    Direction::East,
    Direction::South,
    Direction::West,
];

const BISHOP_DIRS: [Direction; 4] = [
    Direction::NorthWest,
    Direction::NorthEast,
    Direction::SouthWest,
    Direction::SouthEast,
];

impl Default for Targets {
    fn default() -> Self {
        let ray_table = RayTable::new();

        Self {
            kings: generate_king_targets_table(),
            knights: generate_knight_targets_table(),
            rays: ray_table,
            attacks_cache: FxHashMap::default(),
        }
    }
}

impl Targets {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get(&self, square: u64, piece: Piece) -> u64 {
        let square_i = square.trailing_zeros() as usize;
        match piece {
            Piece::Knight => self.knights[square_i],
            Piece::King => self.kings[square_i],
            _ => 0,
        }
    }

    pub fn get_cached_attack(&self, color: Color, board_hash: u64) -> Option<u64> {
        self.attacks_cache.get(&(color as u8, board_hash)).copied()
    }

    pub fn cache_attack(&mut self, color: Color, board_hash: u64, attack_targets: u64) -> u64 {
        match self
            .attacks_cache
            .insert((color as u8, board_hash), attack_targets)
        {
            Some(old_targets) => old_targets,
            None => attack_targets,
        }
    }
}

pub fn generate_piece_targets(
    board: &Board,
    color: Color,
    piece: Piece,
    targets: &Targets,
) -> Vec<PieceTarget> {
    let mut piece_targets: Vec<(u64, u64)> = vec![];
    let pieces = board.pieces(color).locate(piece);
    let occupied = board.pieces(color).occupied();

    for &sq in &ORDERED {
        if pieces & sq == 0 {
            continue;
        }

        let candidates = targets.get(sq, piece) & !occupied;

        if candidates == 0 {
            continue;
        }

        piece_targets.push((sq, candidates));
    }

    piece_targets
}

pub fn generate_pawn_targets(board: &Board, color: Color) -> Vec<PieceTarget> {
    let mut piece_targets: Vec<PieceTarget> = vec![];

    let pawns = board.pieces(color).locate(Piece::Pawn);
    let occupied = board.occupied();

    let single_move_targets = match color {
        Color::White => pawns << 8, // move 1 rank up the board
        Color::Black => pawns >> 8, // move 1 rank down the board
    };
    let double_move_targets = match color {
        Color::White => RANK_4,
        Color::Black => RANK_5,
    };
    let move_targets = (single_move_targets | double_move_targets) & !occupied;

    for x in 0..64 {
        let pawn = 1 << x;
        if pawns & pawn == 0 {
            continue;
        }
        let mut targets = EMPTY;

        let single_move = match color {
            Color::White => pawn << 8,
            Color::Black => pawn >> 8,
        };

        if single_move & occupied > 0 {
            // pawn is blocked and can make no moves
            continue;
        }

        let double_move = match color {
            Color::White => single_move << 8,
            Color::Black => single_move >> 8,
        };

        targets |= single_move & move_targets;
        targets |= double_move & move_targets;

        if targets == EMPTY {
            continue;
        }

        piece_targets.push((pawn, targets));
    }

    let attack_targets = board.pieces(color.opposite()).occupied();

    for (pawn, targets) in generate_pawn_attack_targets(board, color) {
        if attack_targets & targets > 0 {
            piece_targets.push((pawn, attack_targets & targets));
        }
    }

    piece_targets
}

// having a separate function for generating pawn attacks is useful for generating
// attack maps. this separates the attacked squares from the ones with enemy pieces
// on them
pub fn generate_pawn_attack_targets(board: &Board, color: Color) -> Vec<PieceTarget> {
    let mut piece_targets: Vec<PieceTarget> = vec![];

    let pawns = board.pieces(color).locate(Piece::Pawn);

    for x in 0..64 {
        let pawn = 1 << x;
        if pawns & pawn == 0 {
            continue;
        }

        let attack_west = match color {
            Color::White => (pawn << 9) & !A_FILE,
            Color::Black => (pawn >> 7) & !A_FILE,
        };

        let attack_east = match color {
            Color::White => (pawn << 7) & !H_FILE,
            Color::Black => (pawn >> 9) & !H_FILE,
        };

        let targets = attack_east | attack_west;

        piece_targets.push((pawn, targets));
    }

    piece_targets
}

pub fn generate_knight_targets_table() -> [u64; 64] {
    let mut table = [0; 64];

    for square_i in 0..64 {
        let knight = 1 << square_i;

        // nne = north-north-east, nee = north-east-east, etc..
        let move_nne = knight << 17 & !A_FILE;
        let move_nee = knight << 10 & !A_FILE & !B_FILE;
        let move_see = knight >> 6 & !A_FILE & !B_FILE;
        let move_sse = knight >> 15 & !A_FILE;
        let move_nnw = knight << 15 & !H_FILE;
        let move_nww = knight << 6 & !G_FILE & !H_FILE;
        let move_sww = knight >> 10 & !G_FILE & !H_FILE;
        let move_ssw = knight >> 17 & !H_FILE;

        let targets =
            move_nne | move_nee | move_see | move_sse | move_nnw | move_nww | move_sww | move_ssw;

        table[square_i] = targets;
    }

    table
}

pub fn generate_king_targets_table() -> [u64; 64] {
    let mut table = [0; 64];

    for square_i in 0..64 {
        let king = 1 << square_i;

        let mut targets = EMPTY;

        // shift the king's position. in the event that it falls off of the boundary,
        // we want to negate the rank/file where the king would fall.
        targets |= (king << 9) & !RANK_1 & !A_FILE; // northeast
        targets |= (king << 8) & !RANK_1; // north
        targets |= (king << 7) & !RANK_1 & !H_FILE; // northwest

        targets |= (king >> 7) & !RANK_8 & !A_FILE; // southeast
        targets |= (king >> 8) & !RANK_8; // south
        targets |= (king >> 9) & !RANK_8 & !H_FILE; // southwest

        targets |= (king << 1) & !A_FILE; // east
        targets |= (king >> 1) & !H_FILE; // west

        table[square_i] = targets;
    }

    table
}

fn generate_ray_targets(
    board: &Board,
    color: Color,
    targets: &Targets,
    ray_piece: Piece,
    ray_dirs: [Direction; 4],
) -> Vec<PieceTarget> {
    let pieces = board.pieces(color).locate(ray_piece);
    let occupied = board.occupied();
    let mut piece_targets: Vec<(u64, u64)> = vec![];

    for x in 0..64 {
        let piece = 1 << x;
        if pieces & piece == 0 {
            continue;
        }

        let mut target_squares = EMPTY;

        for dir in ray_dirs.iter() {
            let ray = targets.rays.get(piece, *dir);
            if ray == 0 {
                continue;
            }

            let intercepts = ray & occupied;

            if intercepts == 0 {
                piece_targets.push((piece, ray));
                continue;
            }

            // intercept = where the piece's ray is terminated.
            // in each direction, the goal is to select the intercept
            // that is closest to the piece. for each direction, this is either
            // the leftmost or rightmost bit.
            let intercept = match dir {
                // ROOKS
                Direction::North => rightmost_bit(intercepts),
                Direction::East => rightmost_bit(intercepts),
                Direction::South => leftmost_bit(intercepts),
                Direction::West => leftmost_bit(intercepts),

                // BISHOPS
                Direction::NorthWest => rightmost_bit(intercepts),
                Direction::NorthEast => rightmost_bit(intercepts),
                Direction::SouthWest => leftmost_bit(intercepts),
                Direction::SouthEast => leftmost_bit(intercepts),
            };

            let blocked_squares = targets.rays.get(intercept, *dir);

            target_squares |= ray ^ blocked_squares;

            // if the intercept is the same color piece, remove it from the targets.
            // otherwise, it is a target square because it belongs to the other
            // color and can therefore be captured
            if intercept & board.pieces(color).occupied() > 0 {
                target_squares ^= intercept;
            }
        }

        piece_targets.push((piece, target_squares));
    }

    piece_targets
}

pub fn generate_rook_targets(board: &Board, color: Color, targets: &Targets) -> Vec<PieceTarget> {
    generate_ray_targets(board, color, targets, Piece::Rook, ROOK_DIRS)
}

pub fn generate_bishop_targets(board: &Board, color: Color, targets: &Targets) -> Vec<PieceTarget> {
    generate_ray_targets(board, color, targets, Piece::Bishop, BISHOP_DIRS)
}

pub fn generate_queen_targets(board: &Board, color: Color, targets: &Targets) -> Vec<PieceTarget> {
    let mut piece_targets: Vec<PieceTarget> = vec![];

    piece_targets.append(&mut generate_ray_targets(
        board,
        color,
        targets,
        Piece::Queen,
        ROOK_DIRS,
    ));
    piece_targets.append(&mut generate_ray_targets(
        board,
        color,
        targets,
        Piece::Queen,
        BISHOP_DIRS,
    ));

    piece_targets
}

pub fn generate_attack_targets(board: &Board, color: Color, targets: &mut Targets) -> u64 {
    let board_hash = board.current_position_hash();

    if let Some(cached_targets) = targets.get_cached_attack(color, board_hash) {
        debug!(
            "cached attack targets for color {:?}:\n{}",
            color, cached_targets
        );
        return cached_targets;
    }

    let mut piece_targets: Vec<PieceTarget> = vec![];
    let mut attack_targets = EMPTY;

    piece_targets.append(&mut generate_pawn_attack_targets(board, color));
    piece_targets.append(&mut generate_piece_targets(
        board,
        color,
        Piece::Knight,
        targets,
    ));
    piece_targets.append(&mut generate_rook_targets(board, color, targets));
    piece_targets.append(&mut generate_bishop_targets(board, color, targets));
    piece_targets.append(&mut generate_queen_targets(board, color, targets));
    piece_targets.append(&mut generate_piece_targets(
        board,
        color,
        Piece::King,
        targets,
    ));

    for (_piece, targets) in piece_targets {
        attack_targets |= targets;
    }

    targets.cache_attack(color, board_hash, attack_targets);

    attack_targets
}

fn rightmost_bit(x: u64) -> u64 {
    x & (!x + 1)
}

fn leftmost_bit(x: u64) -> u64 {
    let mut b = x;

    // fill in rightmost bits
    b |= b >> 32;
    b |= b >> 16;
    b |= b >> 8;
    b |= b >> 4;
    b |= b >> 2;
    b |= b >> 1;

    // get the leftmost bit
    b ^ (b >> 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess_move::chess_move_collection::ChessMoveCollection;
    use crate::chess_move::standard::StandardChessMove;
    use crate::{
        board::bitboard::{render_occupied, RANK_3},
        chess_moves, std_move,
    };

    #[test]
    fn test_generate_attack_targets() {
        let mut targets = Targets::new();
        let mut board = Board::new();

        board.put(A4, Piece::Pawn, Color::White).unwrap();
        board.put(B5, Piece::Pawn, Color::Black).unwrap();
        board.put(B1, Piece::Rook, Color::White).unwrap();
        board.put(H1, Piece::King, Color::Black).unwrap();
        board.put(A5, Piece::Queen, Color::White).unwrap();
        println!("Testing board:\n{}", board);

        let expected_white_targets = EMPTY
            // pawn
            | B5
            // rook
            | B2
            | B3
            | B4
            | B5
            | (RANK_1 ^ B1)
            // queen - north
            | A6
            | A7
            | A8
            // queen - northeast
            | B6
            | C7
            | D8
            // queen - east
            | B5
            // queen - southeast
            | B4
            | C3
            | D2
            | A1;
        let white_targets = generate_attack_targets(&board, Color::White, &mut targets);
        assert_eq!(expected_white_targets, white_targets);

        let expected_black_targets = EMPTY
            // pawn
            | A4
            | C4
            // king
            | G1
            | G2
            | H2;
        let black_targets = generate_attack_targets(&board, Color::Black, &mut targets);
        assert_eq!(expected_black_targets, black_targets);
    }

    #[test]
    pub fn test_generate_attack_targets_2() {
        let mut targets = Targets::new();
        let mut board = Board::starting_position();
        let mut moves = chess_moves![
            std_move!(E2, E4),
            std_move!(F7, F5),
            std_move!(D1, H5),
            std_move!(G7, G6),
        ];

        for m in moves.drain() {
            m.apply(&mut board).unwrap();
            board.toggle_turn();
        }
        println!("Testing board:\n{}", board);

        //   +---+---+---+---+---+---+---+---+
        // 8 | r | n | b | q | k | b | n | r |
        //   +---+---+---+---+---+---+---+---+
        // 7 | p | p | p | p | p |   |   | p |
        //   +---+---+---+---+---+---+---+---+
        // 6 |   |   |   |   |   |   | p |   |
        //   +---+---+---+---+---+---+---+---+
        // 5 |   |   |   |   |   | p |   | Q |
        //   +---+---+---+---+---+---+---+---+
        // 4 |   |   |   |   | P |   |   |   |
        //   +---+---+---+---+---+---+---+---+
        // 3 |   |   |   |   |   |   |   |   |
        //   +---+---+---+---+---+---+---+---+
        // 2 | P | P | P | P |   | P | P | P |
        //   +---+---+---+---+---+---+---+---+
        // 1 | R | N | B |   | K | B | N | R |
        //   +---+---+---+---+---+---+---+---+
        //     A   B   C   D   E   F   G   H

        let expected_white_targets = EMPTY
            // knights
            | RANK_3
            // forward pawn
            | D5
            | F5
            // queen - north
            | H6
            | H7
            // queen - nortwest
            | G6
            // queen - west
            | G5
            | F5
            // queen - southwest
            | G4
            | F3
            | E2
            | D1
            // queen - south
            | H4
            | H3
            // bishop
            | E2
            | D3
            | C4
            | B5
            | A6
            // king
            | D1
            | E2;

        let white_targets = generate_attack_targets(&board, Color::White, &mut targets);
        println!(
            "expected white targets:\n{}",
            render_occupied(expected_white_targets)
        );
        println!("actual white targets:\n{}", render_occupied(white_targets));
        assert_eq!(expected_white_targets, white_targets);
    }
}
