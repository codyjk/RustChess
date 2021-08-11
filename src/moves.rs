mod board;
mod debug;
pub mod ray_table;

use crate::board::bitboard::{
    Bitboard, A_FILE, B_FILE, EMPTY, G_FILE, H_FILE, RANK_1, RANK_4, RANK_5, RANK_8,
};
use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::square;
use crate::board::Board;
use ray_table::{Direction, RayTable, BISHOP_DIRS, ROOK_DIRS};

type Capture = (Piece, Color);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChessMove {
    pub from_square: u64,
    pub to_square: u64,
    pub capture: Option<Capture>,
}

impl ChessMove {
    pub fn new(from_square: u64, to_square: u64, capture: Option<Capture>) -> Self {
        Self {
            from_square: from_square,
            to_square: to_square,
            capture: capture,
        }
    }
}

pub fn generate(board: &Board, color: Color, ray_table: &RayTable) -> Vec<ChessMove> {
    let mut moves = vec![];

    moves.append(&mut generate_pawn_moves(board, color));
    moves.append(&mut generate_knight_moves(board, color));
    moves.append(&mut generate_king_moves(board, color));
    moves.append(&mut generate_rook_moves(board, color, ray_table));
    moves.append(&mut generate_bishop_moves(board, color, ray_table));
    moves.append(&mut generate_queen_moves(board, color, ray_table));

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
            let mv = ChessMove::new(square::assert(pawn), square::assert(single_move), None);
            moves.push(mv);
        }

        let double_move = match color {
            Color::White => single_move << 8,
            Color::Black => single_move >> 8,
        };
        if double_move & move_targets > 0 {
            let mv = ChessMove::new(square::assert(pawn), square::assert(double_move), None);
            moves.push(mv);
        }

        let attack_west = match color {
            Color::White => (pawn << 9) & !A_FILE,
            Color::Black => (pawn >> 7) & !A_FILE,
        };
        if attack_west & attack_targets > 0 {
            let captured_piece = board
                .pieces(color.opposite())
                .get(square::assert(attack_west))
                .unwrap();
            let capture = (captured_piece, color.opposite());
            let mv = ChessMove::new(
                square::assert(pawn),
                square::assert(attack_west),
                Some(capture),
            );
            moves.push(mv);
        }

        let attack_east = match color {
            Color::White => (pawn << 7) & !H_FILE,
            Color::Black => (pawn >> 9) & !H_FILE,
        };
        if attack_east & attack_targets > 0 {
            let captured_piece = board
                .pieces(color.opposite())
                .get(square::assert(attack_east))
                .unwrap();
            let capture = (captured_piece, color.opposite());
            let mv = ChessMove::new(
                square::assert(pawn),
                square::assert(attack_east),
                Some(capture),
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

        let mv = ChessMove::new(square::assert(knight), square::assert(target), None);
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

fn generate_ray_moves(
    board: &Board,
    color: Color,
    ray_table: &RayTable,
    ray_piece: Piece,
    ray_dirs: [Direction; 4],
) -> Vec<ChessMove> {
    let pieces = board.pieces(color).locate(ray_piece);
    let occupied = board.occupied();

    let mut moves: Vec<ChessMove> = vec![];
    let mut intermediates: Vec<(Bitboard, Bitboard)> = vec![];

    for x in 0..64 {
        let piece = 1 << x;
        if pieces & piece == 0 {
            continue;
        }

        let sq = square::assert(piece);
        let mut target_squares = EMPTY;

        for dir in ray_dirs.iter() {
            let ray = ray_table.get(sq, *dir);
            if ray == 0 {
                continue;
            }

            let intercepts = ray & occupied;

            if intercepts == 0 {
                intermediates.push((piece, ray));
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
                Direction::NorthWest => leftmost_bit(intercepts),
                Direction::NorthEast => rightmost_bit(intercepts),
                Direction::SouthWest => leftmost_bit(intercepts),
                Direction::SouthEast => rightmost_bit(intercepts),
            };

            let blocked_squares = ray_table.get(square::assert(intercept), *dir);

            target_squares |= ray ^ blocked_squares;

            // if the intercept is the same color piece, remove it from the targets.
            // otherwise, it is a target square because it belongs to the other
            // color and can therefore be captured
            if intercept & board.pieces(color).occupied() > 0 {
                target_squares ^= intercept;
            }
        }

        intermediates.push((piece, target_squares));
    }

    for (piece, target_squares) in intermediates {
        let piece_sq = square::assert(piece);
        for x in 0..64 {
            let target = 1 << x;
            if target_squares & target == 0 {
                continue;
            }

            let target_sq = square::assert(target);
            let capture = match board.pieces(color.opposite()).get(target_sq) {
                Some(piece) => Some((piece, color.opposite())),
                None => None,
            };

            moves.push(ChessMove::new(piece_sq, target_sq, capture));
        }
    }

    moves
}

fn generate_rook_moves(board: &Board, color: Color, ray_table: &RayTable) -> Vec<ChessMove> {
    generate_ray_moves(board, color, ray_table, Piece::Rook, ROOK_DIRS)
}

fn generate_bishop_moves(board: &Board, color: Color, ray_table: &RayTable) -> Vec<ChessMove> {
    generate_ray_moves(board, color, ray_table, Piece::Bishop, BISHOP_DIRS)
}

fn generate_queen_moves(board: &Board, color: Color, ray_table: &RayTable) -> Vec<ChessMove> {
    let mut moves: Vec<ChessMove> = vec![];
    moves.append(&mut generate_ray_moves(
        board,
        color,
        ray_table,
        Piece::Queen,
        ROOK_DIRS,
    ));
    moves.append(&mut generate_ray_moves(
        board,
        color,
        ray_table,
        Piece::Queen,
        BISHOP_DIRS,
    ));
    moves
}

fn generate_king_moves(board: &Board, color: Color) -> Vec<ChessMove> {
    let mut moves: Vec<ChessMove> = vec![];
    let king = board.pieces(color).locate(Piece::King);
    let king_sq = square::assert(king);
    let occupied = board.pieces(color).occupied();

    let mut targets = EMPTY;

    // shift the king's position. in the event that it falls off of the boundary,
    // we want to negate the rank/file where the king would fall.
    targets |= (king >> 8) & !RANK_1 & !occupied; // north
    targets |= (king << 8) & !RANK_8 & !occupied; // south
    targets |= (king << 1) & !A_FILE & !occupied; // east
    targets |= (king >> 1) & !H_FILE & !occupied; // west
    targets |= (king >> 7) & !RANK_1 & !A_FILE & !occupied; // northeast
    targets |= (king >> 9) & !RANK_1 & !H_FILE & !occupied; // northwest
    targets |= (king << 9) & !RANK_8 & !A_FILE & !occupied; // southeast
    targets |= (king << 7) & !RANK_8 & !H_FILE & !occupied; // southwest

    for x in 0..64 {
        let target = 1 << x;
        if target & targets == 0 {
            continue;
        }

        let target_sq = square::assert(target);
        let capture = match board.pieces(color.opposite()).get(target_sq) {
            Some(piece) => Some((piece, color.opposite())),
            None => None,
        };

        moves.push(ChessMove::new(king_sq, square::assert(target), capture));
    }

    moves
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_pawn_moves() {
        let mut board = Board::new();
        board.put(square::A4, Piece::Pawn, Color::White).unwrap();
        board.put(square::A5, Piece::Pawn, Color::Black).unwrap();
        board.put(square::D2, Piece::Pawn, Color::White).unwrap();
        board.put(square::D7, Piece::Pawn, Color::Black).unwrap();
        board.put(square::G6, Piece::Pawn, Color::White).unwrap();
        board.put(square::H7, Piece::Pawn, Color::Black).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let expected_white_moves: Vec<ChessMove> = vec![
            ChessMove::new(square::D2, square::D3, None),
            ChessMove::new(square::D2, square::D4, None),
            ChessMove::new(square::G6, square::G7, None),
            ChessMove::new(square::G6, square::H7, Some((Piece::Pawn, Color::Black))),
        ];

        let expected_black_moves: Vec<ChessMove> = vec![
            ChessMove::new(square::D7, square::D6, None),
            ChessMove::new(square::D7, square::D5, None),
            ChessMove::new(square::H7, square::H6, None),
            ChessMove::new(square::H7, square::H5, None),
            ChessMove::new(square::H7, square::G6, Some((Piece::Pawn, Color::White))),
        ];

        let white_moves = generate_pawn_moves(&board, Color::White);
        assert_eq!(expected_white_moves, white_moves);

        let black_moves = generate_pawn_moves(&board, Color::Black);
        assert_eq!(expected_black_moves, black_moves);
    }

    #[test]
    fn test_generate_knight_moves() {
        let mut board = Board::new();
        board.put(square::C3, Piece::Knight, Color::White).unwrap();
        board.put(square::H6, Piece::Knight, Color::Black).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let expected_white_moves: Vec<ChessMove> = vec![
            ChessMove::new(square::C3, square::D5, None),
            ChessMove::new(square::C3, square::E4, None),
            ChessMove::new(square::C3, square::E2, None),
            ChessMove::new(square::C3, square::D1, None),
            ChessMove::new(square::C3, square::B5, None),
            ChessMove::new(square::C3, square::A4, None),
            ChessMove::new(square::C3, square::A2, None),
            ChessMove::new(square::C3, square::B1, None),
        ];

        let expected_black_moves: Vec<ChessMove> = vec![
            ChessMove::new(square::H6, square::G8, None),
            ChessMove::new(square::H6, square::F7, None),
            ChessMove::new(square::H6, square::F5, None),
            ChessMove::new(square::H6, square::G4, None),
        ];

        let white_moves = generate_knight_moves(&board, Color::White);
        assert_eq!(expected_white_moves, white_moves);

        let black_moves = generate_knight_moves(&board, Color::Black);
        assert_eq!(expected_black_moves, black_moves);
    }

    #[test]
    fn test_generate_rook_moves_1() {
        let mut board = Board::new();
        board.put(square::A3, Piece::Pawn, Color::White).unwrap();
        board.put(square::H3, Piece::Pawn, Color::Black).unwrap();
        board.put(square::C3, Piece::Rook, Color::White).unwrap();
        board.put(square::C1, Piece::King, Color::White).unwrap();
        board.put(square::C7, Piece::Pawn, Color::White).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let mut expected_moves: Vec<ChessMove> = vec![
            ChessMove::new(square::C3, square::C2, None),
            ChessMove::new(square::C3, square::C4, None),
            ChessMove::new(square::C3, square::C5, None),
            ChessMove::new(square::C3, square::C6, None),
            ChessMove::new(square::C3, square::B3, None),
            ChessMove::new(square::C3, square::D3, None),
            ChessMove::new(square::C3, square::E3, None),
            ChessMove::new(square::C3, square::F3, None),
            ChessMove::new(square::C3, square::G3, None),
            ChessMove::new(square::C3, square::H3, Some((Piece::Pawn, Color::Black))),
        ];
        expected_moves.sort();

        let mut moves = generate_rook_moves(&board, Color::White, RayTable::new().populate());
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_rook_moves_2() {
        let mut board = Board::new();
        board.put(square::A4, Piece::Pawn, Color::White).unwrap();
        board.put(square::A2, Piece::Rook, Color::White).unwrap();
        board.put(square::B2, Piece::Pawn, Color::White).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let mut expected_moves: Vec<ChessMove> = vec![
            ChessMove::new(square::A2, square::A1, None),
            ChessMove::new(square::A2, square::A3, None),
        ];
        expected_moves.sort();

        let mut moves = generate_rook_moves(&board, Color::White, RayTable::new().populate());
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_bishop_moves() {
        let mut board = Board::new();
        board.put(square::E5, Piece::Bishop, Color::White).unwrap();
        board.put(square::A1, Piece::Pawn, Color::White).unwrap();
        board.put(square::C3, Piece::Pawn, Color::White).unwrap();
        board.put(square::C7, Piece::Pawn, Color::White).unwrap();
        board.put(square::G7, Piece::Pawn, Color::Black).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let mut expected_moves: Vec<ChessMove> = vec![
            ChessMove::new(square::E5, square::D4, None),
            ChessMove::new(square::E5, square::D6, None),
            ChessMove::new(square::E5, square::F4, None),
            ChessMove::new(square::E5, square::F6, None),
            ChessMove::new(square::E5, square::G3, None),
            ChessMove::new(square::E5, square::G7, Some((Piece::Pawn, Color::Black))),
            ChessMove::new(square::E5, square::H2, None),
        ];
        expected_moves.sort();

        let mut moves = generate_bishop_moves(&board, Color::White, RayTable::new().populate());
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_queen_moves() {
        let mut board = Board::new();
        board.put(square::E5, Piece::Queen, Color::White).unwrap();
        board.put(square::E6, Piece::Pawn, Color::White).unwrap();
        board.put(square::E7, Piece::Pawn, Color::Black).unwrap();
        board.put(square::H8, Piece::Pawn, Color::Black).unwrap();
        board.put(square::B2, Piece::Pawn, Color::White).unwrap();
        board.put(square::B5, Piece::Pawn, Color::White).unwrap();
        board.put(square::G3, Piece::Pawn, Color::Black).unwrap();
        board.put(square::H2, Piece::Pawn, Color::White).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let mut expected_moves: Vec<ChessMove> = vec![
            // North - no moves
            // NorthEast
            ChessMove::new(square::E5, square::F6, None),
            ChessMove::new(square::E5, square::G7, None),
            ChessMove::new(square::E5, square::H8, Some((Piece::Pawn, Color::Black))),
            // East
            ChessMove::new(square::E5, square::F5, None),
            ChessMove::new(square::E5, square::G5, None),
            ChessMove::new(square::E5, square::H5, None),
            // SouthEast
            ChessMove::new(square::E5, square::F4, None),
            ChessMove::new(square::E5, square::G3, Some((Piece::Pawn, Color::Black))),
            // South
            ChessMove::new(square::E5, square::E4, None),
            ChessMove::new(square::E5, square::E3, None),
            ChessMove::new(square::E5, square::E2, None),
            ChessMove::new(square::E5, square::E1, None),
            // SouthWest
            ChessMove::new(square::E5, square::D4, None),
            ChessMove::new(square::E5, square::C3, None),
            // West
            ChessMove::new(square::E5, square::D5, None),
            ChessMove::new(square::E5, square::C5, None),
            // NorthWest
            ChessMove::new(square::E5, square::D6, None),
            ChessMove::new(square::E5, square::C7, None),
            ChessMove::new(square::E5, square::B8, None),
        ];
        expected_moves.sort();

        let mut moves = generate_queen_moves(&board, Color::White, RayTable::new().populate());
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_king_moves_corner() {
        let mut board = Board::new();
        board.put(square::A1, Piece::King, Color::White).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let mut expected_moves: Vec<ChessMove> = vec![
            ChessMove::new(square::A1, square::A2, None),
            ChessMove::new(square::A1, square::B1, None),
            ChessMove::new(square::A1, square::B2, None),
        ];
        expected_moves.sort();

        let mut moves = generate_king_moves(&board, Color::White);
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_king_moves_edge() {
        let mut board = Board::new();
        board.put(square::E1, Piece::King, Color::White).unwrap();
        board.put(square::D2, Piece::Pawn, Color::Black).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let mut expected_moves: Vec<ChessMove> = vec![
            ChessMove::new(square::E1, square::D1, None),
            ChessMove::new(square::E1, square::D2, Some((Piece::Pawn, Color::Black))),
            ChessMove::new(square::E1, square::E2, None),
            ChessMove::new(square::E1, square::F1, None),
            ChessMove::new(square::E1, square::F2, None),
        ];
        expected_moves.sort();

        let mut moves = generate_king_moves(&board, Color::White);
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_king_moves_middle() {
        let mut board = Board::new();
        board.put(square::E5, Piece::King, Color::White).unwrap();
        board.put(square::E6, Piece::Pawn, Color::White).unwrap();
        board.put(square::E4, Piece::Pawn, Color::Black).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let mut expected_moves: Vec<ChessMove> = vec![
            ChessMove::new(square::E5, square::D4, None),
            ChessMove::new(square::E5, square::D5, None),
            ChessMove::new(square::E5, square::D6, None),
            ChessMove::new(square::E5, square::E4, Some((Piece::Pawn, Color::Black))),
            ChessMove::new(square::E5, square::F4, None),
            ChessMove::new(square::E5, square::F5, None),
            ChessMove::new(square::E5, square::F6, None),
        ];
        expected_moves.sort();

        let mut moves = generate_king_moves(&board, Color::White);
        moves.sort();

        assert_eq!(expected_moves, moves);
    }
}
