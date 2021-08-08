mod board;
mod debug;
mod fen;
mod ray_table;

use crate::board::bitboard::{Bitboard, A_FILE, B_FILE, EMPTY, G_FILE, H_FILE, RANK_4, RANK_5};
use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::square::Square;
use crate::board::Board;
use ray_table::{Direction, RayTable, ROOK_DIRS};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChessMove {
    pub from_square: Square,
    pub to_square: Square,
}

impl ChessMove {
    pub fn new(from_square: Square, to_square: Square) -> Self {
        Self {
            from_square: from_square,
            to_square: to_square,
        }
    }
}

pub fn generate(board: &Board, color: Color) -> Vec<ChessMove> {
    let mut moves = vec![];

    moves.append(&mut generate_pawn_moves(board, color));
    moves.append(&mut generate_knight_moves(board, color));
    moves.append(&mut generate_rook_moves(board, color));

    moves
}

fn generate_pawn_moves(board: &Board, color: Color) -> Vec<ChessMove> {
    let pawns = board.pieces(color).locate(Piece::Pawn);
    let occupied = board.occupied();

    let single_move_targets = match color {
        Color::White => pawns << 8, // move 1 rank up the board
        Color::Black => pawns >> 8, // move 1 rank down the board
    };
    let double_move_targets = match color {
        Color::White => RANK_4, // rank 4
        Color::Black => RANK_5, // rank 5
    };
    let move_targets = (single_move_targets | double_move_targets) & !occupied;
    let attack_targets = board.pieces(color.opposite()).occupied();

    let mut moves: Vec<ChessMove> = vec![];

    for x in 0..64 {
        let pawn = 1 << x;
        if pawns & pawn == 0 {
            continue;
        }

        let single_move = match color {
            Color::White => pawn << 8,
            Color::Black => pawn >> 8,
        };
        if single_move & move_targets > 0 {
            let mv = ChessMove::new(
                Square::from_bitboard(pawn),
                Square::from_bitboard(single_move),
            );
            moves.push(mv);
        }

        let double_move = match color {
            Color::White => single_move << 8,
            Color::Black => single_move >> 8,
        };
        if double_move & move_targets > 0 {
            let mv = ChessMove::new(
                Square::from_bitboard(pawn),
                Square::from_bitboard(double_move),
            );
            moves.push(mv);
        }

        let attack_west = match color {
            Color::White => (pawn << 9) & !A_FILE,
            Color::Black => (pawn >> 7) & !A_FILE,
        };
        if attack_west & attack_targets > 0 {
            let mv = ChessMove::new(
                Square::from_bitboard(pawn),
                Square::from_bitboard(attack_west),
            );
            moves.push(mv);
        }

        let attack_east = match color {
            Color::White => (pawn << 7) & !H_FILE,
            Color::Black => (pawn >> 9) & !H_FILE,
        };
        if attack_east & attack_targets > 0 {
            let mv = ChessMove::new(
                Square::from_bitboard(pawn),
                Square::from_bitboard(attack_east),
            );
            moves.push(mv);
        }
    }

    moves
}

fn generate_knight_moves(board: &Board, color: Color) -> Vec<ChessMove> {
    let mut intermediates: Vec<(Bitboard, Bitboard)> = vec![];
    let mut moves: Vec<ChessMove> = vec![];
    let knights = board.pieces(color).locate(Piece::Knight);

    for x in 0..64 {
        let knight = 1 << x;
        if knights & knight == 0 {
            continue;
        }

        // nne = north-north-east, nee = north-east-east, etc..
        let move_nne = knight << 17 & !A_FILE;
        let move_nee = knight << 10 & !A_FILE & !B_FILE;
        let move_see = knight >> 6 & !A_FILE & !B_FILE;
        let move_sse = knight >> 15 & !A_FILE;
        let move_nnw = knight << 15 & !H_FILE;
        let move_nww = knight << 6 & !G_FILE & !H_FILE;
        let move_sww = knight >> 10 & !G_FILE & !H_FILE;
        let move_ssw = knight >> 17 & !H_FILE;

        intermediates.push((knight, move_nne));
        intermediates.push((knight, move_nee));
        intermediates.push((knight, move_see));
        intermediates.push((knight, move_sse));
        intermediates.push((knight, move_nnw));
        intermediates.push((knight, move_nww));
        intermediates.push((knight, move_sww));
        intermediates.push((knight, move_ssw));
    }

    for (knight, target) in intermediates {
        if target == 0 {
            continue;
        }

        let mv = ChessMove::new(Square::from_bitboard(knight), Square::from_bitboard(target));
        moves.push(mv);
    }

    moves
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

fn generate_rook_moves(board: &Board, color: Color) -> Vec<ChessMove> {
    let mut moves: Vec<ChessMove> = vec![];
    let mut intermediates: Vec<(Bitboard, Bitboard)> = vec![];

    // TODO: cache table somewhere else
    let mut ray_table = RayTable::new();
    ray_table.populate();

    let pieces = board.pieces(color);
    let rooks = pieces.locate(Piece::Rook);
    let occupied = board.occupied();

    for x in 0..64 {
        let rook = 1 << x;
        if rooks & rook == 0 {
            continue;
        }

        let sq = Square::from_bitboard(rook);
        let mut target_squares = EMPTY;

        for dir in ROOK_DIRS.iter() {
            let ray = ray_table.get(sq, *dir);
            if ray == 0 {
                continue;
            }

            let intercepts = ray & occupied;

            if intercepts == 0 {
                intermediates.push((rook, ray));
                continue;
            }

            // intercept = where the rook's ray is terminated
            let intercept = match dir {
                // the lowest/rightmost square in the file => the rightmost bit
                Direction::North => rightmost_bit(intercepts),
                Direction::East => rightmost_bit(intercepts),
                // the highest/leftmost square in the file => the leftmost bit
                Direction::South => leftmost_bit(intercepts),
                Direction::West => leftmost_bit(intercepts),
            };

            let blocked_squares = ray_table.get(Square::from_bitboard(intercept), *dir);

            target_squares |= ray ^ blocked_squares;

            // if the intercept is the same color piece, remove it from the targets.
            // otherwise, it is a target square because it belongs to the other
            // color and can therefore be captured
            if intercept & pieces.occupied() > 0 {
                target_squares ^= intercept;
            }
        }

        intermediates.push((rook, target_squares));
    }

    for (rook, target_squares) in intermediates {
        let rook_sq = Square::from_bitboard(rook);
        for x in 0..64 {
            let target = 1 << x;
            if target_squares & target == 0 {
                continue;
            }

            moves.push(ChessMove::new(rook_sq, Square::from_bitboard(target)));
        }
    }

    moves
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_pawn_moves() {
        let mut board = Board::new();
        board.put(Square::A4, Piece::Pawn, Color::White).unwrap();
        board.put(Square::A5, Piece::Pawn, Color::Black).unwrap();
        board.put(Square::D2, Piece::Pawn, Color::White).unwrap();
        board.put(Square::D7, Piece::Pawn, Color::Black).unwrap();
        board.put(Square::G6, Piece::Pawn, Color::White).unwrap();
        board.put(Square::H7, Piece::Pawn, Color::Black).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let expected_white_moves: Vec<ChessMove> = vec![
            ChessMove::new(Square::D2, Square::D3),
            ChessMove::new(Square::D2, Square::D4),
            ChessMove::new(Square::G6, Square::G7),
            ChessMove::new(Square::G6, Square::H7),
        ];

        let expected_black_moves: Vec<ChessMove> = vec![
            ChessMove::new(Square::D7, Square::D6),
            ChessMove::new(Square::D7, Square::D5),
            ChessMove::new(Square::H7, Square::H6),
            ChessMove::new(Square::H7, Square::H5),
            ChessMove::new(Square::H7, Square::G6),
        ];

        let white_moves = generate_pawn_moves(&board, Color::White);
        assert_eq!(expected_white_moves, white_moves);

        let black_moves = generate_pawn_moves(&board, Color::Black);
        assert_eq!(expected_black_moves, black_moves);
    }

    #[test]
    fn test_generate_knight_moves() {
        let mut board = Board::new();
        board.put(Square::C3, Piece::Knight, Color::White).unwrap();
        board.put(Square::H6, Piece::Knight, Color::Black).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let expected_white_moves: Vec<ChessMove> = vec![
            ChessMove::new(Square::C3, Square::D5),
            ChessMove::new(Square::C3, Square::E4),
            ChessMove::new(Square::C3, Square::E2),
            ChessMove::new(Square::C3, Square::D1),
            ChessMove::new(Square::C3, Square::B5),
            ChessMove::new(Square::C3, Square::A4),
            ChessMove::new(Square::C3, Square::A2),
            ChessMove::new(Square::C3, Square::B1),
        ];

        let expected_black_moves: Vec<ChessMove> = vec![
            ChessMove::new(Square::H6, Square::G8),
            ChessMove::new(Square::H6, Square::F7),
            ChessMove::new(Square::H6, Square::F5),
            ChessMove::new(Square::H6, Square::G4),
        ];

        let white_moves = generate_knight_moves(&board, Color::White);
        assert_eq!(expected_white_moves, white_moves);

        let black_moves = generate_knight_moves(&board, Color::Black);
        assert_eq!(expected_black_moves, black_moves);
    }

    #[test]
    fn test_generate_rook_moves_1() {
        let mut board = Board::new();
        board.put(Square::A3, Piece::Pawn, Color::White).unwrap();
        board.put(Square::H3, Piece::Pawn, Color::Black).unwrap();
        board.put(Square::C3, Piece::Rook, Color::White).unwrap();
        board.put(Square::C1, Piece::King, Color::White).unwrap();
        board.put(Square::C7, Piece::Pawn, Color::White).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let mut expected_moves: Vec<ChessMove> = vec![
            ChessMove::new(Square::C3, Square::C2),
            ChessMove::new(Square::C3, Square::C4),
            ChessMove::new(Square::C3, Square::C5),
            ChessMove::new(Square::C3, Square::C6),
            ChessMove::new(Square::C3, Square::B3),
            ChessMove::new(Square::C3, Square::D3),
            ChessMove::new(Square::C3, Square::E3),
            ChessMove::new(Square::C3, Square::F3),
            ChessMove::new(Square::C3, Square::G3),
            ChessMove::new(Square::C3, Square::H3),
        ];
        expected_moves.sort();

        let mut moves = generate_rook_moves(&board, Color::White);
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_rook_moves_2() {
        let mut board = Board::new();
        board.put(Square::A4, Piece::Pawn, Color::White).unwrap();
        board.put(Square::A2, Piece::Rook, Color::White).unwrap();
        board.put(Square::B2, Piece::Pawn, Color::White).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let mut expected_moves: Vec<ChessMove> = vec![
            ChessMove::new(Square::A2, Square::A1),
            ChessMove::new(Square::A2, Square::A3),
        ];
        expected_moves.sort();

        let mut moves = generate_rook_moves(&board, Color::White);
        moves.sort();

        assert_eq!(expected_moves, moves);
    }
}
