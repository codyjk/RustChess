mod board;
mod debug;
pub mod ray_table;
mod targets;

use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::square;
use crate::board::Board;
use ray_table::RayTable;
use targets::PieceTarget;

type Capture = (Piece, Color);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChessMove {
    from_square: u64,
    to_square: u64,
    capture: Option<Capture>,
}

impl ChessMove {
    pub fn new(from_square: u64, to_square: u64, capture: Option<Capture>) -> Self {
        Self {
            from_square: from_square,
            to_square: to_square,
            capture: capture,
        }
    }

    pub fn from_square(self) -> u64 {
        self.from_square
    }

    pub fn to_square(self) -> u64 {
        self.to_square
    }

    pub fn capture(self) -> Option<Capture> {
        self.capture
    }
}

pub fn generate(board: &mut Board, color: Color, ray_table: &RayTable) -> Vec<ChessMove> {
    let mut moves = vec![];

    moves.append(&mut generate_pawn_moves(board, color));
    moves.append(&mut generate_knight_moves(board, color));
    moves.append(&mut generate_king_moves(board, color));
    moves.append(&mut generate_rook_moves(board, color, ray_table));
    moves.append(&mut generate_bishop_moves(board, color, ray_table));
    moves.append(&mut generate_queen_moves(board, color, ray_table));

    moves = remove_invalid_moves(moves, board, color, ray_table);

    moves
}

fn generate_pawn_moves(board: &Board, color: Color) -> Vec<ChessMove> {
    let mut piece_targets: Vec<PieceTarget> = vec![];

    piece_targets.append(&mut targets::generate_pawn_move_targets(board, color));
    piece_targets.append(&mut targets::generate_pawn_attack_targets(board, color));

    expand_piece_targets(board, color, piece_targets)
}

fn generate_knight_moves(board: &Board, color: Color) -> Vec<ChessMove> {
    let piece_targets = targets::generate_knight_targets(board, color);
    expand_piece_targets(board, color, piece_targets)
}

fn generate_rook_moves(board: &Board, color: Color, ray_table: &RayTable) -> Vec<ChessMove> {
    let piece_targets = targets::generate_rook_targets(board, color, ray_table);
    expand_piece_targets(board, color, piece_targets)
}

fn generate_bishop_moves(board: &Board, color: Color, ray_table: &RayTable) -> Vec<ChessMove> {
    let piece_targets = targets::generate_bishop_targets(board, color, ray_table);
    expand_piece_targets(board, color, piece_targets)
}

fn generate_queen_moves(board: &Board, color: Color, ray_table: &RayTable) -> Vec<ChessMove> {
    let piece_targets = targets::generate_queen_targets(board, color, ray_table);
    expand_piece_targets(board, color, piece_targets)
}

fn expand_piece_targets(
    board: &Board,
    color: Color,
    piece_targets: Vec<PieceTarget>,
) -> Vec<ChessMove> {
    let mut moves: Vec<ChessMove> = vec![];
    for (piece, target_squares) in piece_targets {
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

fn generate_king_moves(board: &Board, color: Color) -> Vec<ChessMove> {
    let targets = targets::generate_king_targets(board, color);
    expand_piece_targets(board, color, targets)
}

fn remove_invalid_moves(
    candidates: Vec<ChessMove>,
    board: &mut Board,
    color: Color,
    ray_table: &RayTable,
) -> Vec<ChessMove> {
    let mut moves = vec![];

    // simulate each chessmove and see if it leaves the player's king in check.
    // if it does, it's invalid.
    for chessmove in candidates {
        board.apply(chessmove).unwrap();

        let king = board.pieces(color).locate(Piece::King);
        let attacked_squares = targets::generate_attack_targets(board, color.opposite(), ray_table);

        if king & attacked_squares == 0 {
            moves.push(chessmove);
        }

        board.undo(chessmove).unwrap();
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

        let mut expected_white_moves: Vec<ChessMove> = vec![
            ChessMove::new(square::D2, square::D3, None),
            ChessMove::new(square::D2, square::D4, None),
            ChessMove::new(square::G6, square::G7, None),
            ChessMove::new(square::G6, square::H7, Some((Piece::Pawn, Color::Black))),
        ];
        expected_white_moves.sort();

        let mut expected_black_moves: Vec<ChessMove> = vec![
            ChessMove::new(square::D7, square::D6, None),
            ChessMove::new(square::D7, square::D5, None),
            ChessMove::new(square::H7, square::H6, None),
            ChessMove::new(square::H7, square::H5, None),
            ChessMove::new(square::H7, square::G6, Some((Piece::Pawn, Color::White))),
        ];
        expected_black_moves.sort();

        let mut white_moves = generate_pawn_moves(&board, Color::White);
        white_moves.sort();
        assert_eq!(expected_white_moves, white_moves);

        let mut black_moves = generate_pawn_moves(&board, Color::Black);
        black_moves.sort();
        assert_eq!(expected_black_moves, black_moves);
    }

    #[test]
    fn test_generate_knight_moves() {
        let mut board = Board::new();
        board.put(square::C3, Piece::Knight, Color::White).unwrap();
        board.put(square::E4, Piece::Pawn, Color::White).unwrap();
        board.put(square::D5, Piece::Pawn, Color::Black).unwrap();
        board.put(square::H6, Piece::Knight, Color::Black).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let expected_white_moves: Vec<ChessMove> = vec![
            ChessMove::new(square::C3, square::D5, Some((Piece::Pawn, Color::Black))),
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
