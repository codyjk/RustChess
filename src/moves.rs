pub mod board;
pub mod chess_move;
pub mod ray_table;
mod targets;

use crate::board::bitboard::{A_FILE, H_FILE, RANK_1, RANK_8};
use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::square;
use crate::board::{
    Board, BLACK_KINGSIDE_RIGHTS, BLACK_QUEENSIDE_RIGHTS, WHITE_KINGSIDE_RIGHTS,
    WHITE_QUEENSIDE_RIGHTS,
};
use chess_move::ChessMove;
use ray_table::RayTable;
use targets::PieceTarget;

pub const PAWN_PROMOTIONS: [Piece; 4] = [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight];

pub fn generate(board: &mut Board, color: Color, ray_table: &RayTable) -> Vec<ChessMove> {
    let mut moves = vec![];

    moves.append(&mut generate_pawn_moves(board, color));
    moves.append(&mut generate_knight_moves(board, color));
    moves.append(&mut generate_king_moves(board, color));
    moves.append(&mut generate_castle_moves(board, color, ray_table));
    moves.append(&mut generate_rook_moves(board, color, ray_table));
    moves.append(&mut generate_bishop_moves(board, color, ray_table));
    moves.append(&mut generate_queen_moves(board, color, ray_table));

    moves = remove_invalid_moves(moves, board, color, ray_table);

    moves
}

fn generate_pawn_moves(board: &Board, color: Color) -> Vec<ChessMove> {
    let mut moves = vec![];

    // `generate_pawn_targets` blindly generates all pawn "targets": squares
    // that pawns can either move or capture. to get promotions, we will apply
    // some special logic to find the targets that are at the end of the board,
    // and then expand those targets into the candidate promotion pieces.
    let piece_targets = targets::generate_pawn_targets(board, color);
    let all_pawn_moves = expand_piece_targets(board, color, piece_targets);

    let promotion_rank = match color {
        Color::White => RANK_8,
        Color::Black => RANK_1,
    };
    let (partial_promotions, mut standard_pawn_moves): (Vec<ChessMove>, Vec<ChessMove>) =
        all_pawn_moves
            .iter()
            .partition(|&chessmove| chessmove.to_square() & promotion_rank > 0);

    for pawn_move in partial_promotions.iter() {
        for promote_to_piece in &PAWN_PROMOTIONS {
            moves.push(ChessMove::promote(
                pawn_move.from_square(),
                pawn_move.to_square(),
                pawn_move.capture(),
                *promote_to_piece,
            ));
        }
    }

    moves.append(&mut standard_pawn_moves);
    moves.append(&mut generate_en_passant_moves(board, color));

    moves
}

fn generate_en_passant_moves(board: &Board, color: Color) -> Vec<ChessMove> {
    let mut moves: Vec<ChessMove> = vec![];
    let en_passant_target = board.peek_en_passant_target();

    if en_passant_target == 0 {
        return moves;
    }

    let pawns = board.pieces(color).locate(Piece::Pawn);

    let attacks_west = match color {
        Color::White => (pawns << 9) & !A_FILE,
        Color::Black => (pawns >> 7) & !A_FILE,
    };

    let attacks_east = match color {
        Color::White => (pawns << 7) & !H_FILE,
        Color::Black => (pawns >> 9) & !H_FILE,
    };

    if attacks_west & en_passant_target > 0 {
        let from_square = match color {
            Color::White => en_passant_target >> 9,
            Color::Black => en_passant_target << 7,
        };
        moves.push(ChessMove::en_passant(
            from_square,
            en_passant_target,
            (Piece::Pawn, color.opposite()),
        ));
    }

    if attacks_east & en_passant_target > 0 {
        let from_square = match color {
            Color::White => en_passant_target >> 7,
            Color::Black => en_passant_target << 9,
        };
        moves.push(ChessMove::en_passant(
            from_square,
            en_passant_target,
            (Piece::Pawn, color.opposite()),
        ));
    }

    moves
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

fn generate_castle_moves(board: &Board, color: Color, ray_table: &RayTable) -> Vec<ChessMove> {
    let mut moves: Vec<ChessMove> = vec![];
    let attacked_squares = targets::generate_attack_targets(board, color.opposite(), ray_table);

    if board.pieces(color).locate(Piece::King) & attacked_squares > 0 {
        return moves;
    }

    let castle_rights = board.peek_castle_rights();
    let (kingside_rights, queenside_rights) = match color {
        Color::White => (
            WHITE_KINGSIDE_RIGHTS & castle_rights,
            WHITE_QUEENSIDE_RIGHTS & castle_rights,
        ),
        Color::Black => (
            BLACK_KINGSIDE_RIGHTS & castle_rights,
            BLACK_QUEENSIDE_RIGHTS & castle_rights,
        ),
    };

    let (kingside_transit_square, queenside_transit_square) = match color {
        Color::White => (square::F1, square::D1),
        Color::Black => (square::F8, square::D8),
    };

    let queenside_rook_transit_square = match color {
        Color::White => square::B1,
        Color::Black => square::B8,
    };

    let (kingside_target_square, queenside_target_square) = match color {
        Color::White => (square::G1, square::C1),
        Color::Black => (square::G8, square::C8),
    };

    let occupied = board.occupied();

    if kingside_rights > 0
        && board.get(kingside_transit_square).is_none()
        && kingside_transit_square & attacked_squares == 0
        && kingside_transit_square & occupied == 0
        && kingside_target_square & occupied == 0
    {
        moves.push(ChessMove::castle_kingside(color));
    }

    if queenside_rights > 0
        && board.get(queenside_transit_square).is_none()
        && queenside_transit_square & attacked_squares == 0
        && queenside_transit_square & occupied == 0
        && queenside_rook_transit_square & occupied == 0
        && queenside_target_square & occupied == 0
    {
        moves.push(ChessMove::castle_queenside(color));
    }

    moves
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
        board.put(square::B7, Piece::Pawn, Color::White).unwrap();
        board.put(square::C8, Piece::Rook, Color::Black).unwrap();
        board.put(square::A2, Piece::Pawn, Color::Black).unwrap();
        board.put(square::F2, Piece::Pawn, Color::White).unwrap();
        board.put(square::F3, Piece::Rook, Color::Black).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let mut expected_white_moves: Vec<ChessMove> = vec![
            ChessMove::new(square::D2, square::D3, None),
            ChessMove::new(square::D2, square::D4, None),
            ChessMove::new(square::G6, square::G7, None),
            ChessMove::new(square::G6, square::H7, Some((Piece::Pawn, Color::Black))),
            ChessMove::promote(square::B7, square::B8, None, Piece::Queen),
            ChessMove::promote(square::B7, square::B8, None, Piece::Rook),
            ChessMove::promote(square::B7, square::B8, None, Piece::Knight),
            ChessMove::promote(square::B7, square::B8, None, Piece::Bishop),
            ChessMove::promote(
                square::B7,
                square::C8,
                Some((Piece::Rook, Color::Black)),
                Piece::Queen,
            ),
            ChessMove::promote(
                square::B7,
                square::C8,
                Some((Piece::Rook, Color::Black)),
                Piece::Rook,
            ),
            ChessMove::promote(
                square::B7,
                square::C8,
                Some((Piece::Rook, Color::Black)),
                Piece::Knight,
            ),
            ChessMove::promote(
                square::B7,
                square::C8,
                Some((Piece::Rook, Color::Black)),
                Piece::Bishop,
            ),
        ];
        expected_white_moves.sort();

        let mut expected_black_moves: Vec<ChessMove> = vec![
            ChessMove::new(square::D7, square::D6, None),
            ChessMove::new(square::D7, square::D5, None),
            ChessMove::new(square::H7, square::H6, None),
            ChessMove::new(square::H7, square::H5, None),
            ChessMove::new(square::H7, square::G6, Some((Piece::Pawn, Color::White))),
            ChessMove::promote(square::A2, square::A1, None, Piece::Queen),
            ChessMove::promote(square::A2, square::A1, None, Piece::Rook),
            ChessMove::promote(square::A2, square::A1, None, Piece::Knight),
            ChessMove::promote(square::A2, square::A1, None, Piece::Bishop),
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
    fn test_generate_king_moves_edge_south() {
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

    #[test]
    fn test_generate_en_passant_moves() {
        let mut board = Board::new();
        board.put(square::C2, Piece::Pawn, Color::White).unwrap();
        board.put(square::D4, Piece::Pawn, Color::Black).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let move_that_reveals_en_passant_target = ChessMove::new(square::C2, square::C4, None);
        board.apply(move_that_reveals_en_passant_target).unwrap();
        assert_eq!(square::C3, board.peek_en_passant_target());

        let mut expected_black_moves: Vec<ChessMove> = vec![
            ChessMove::new(square::D4, square::D3, None),
            ChessMove::en_passant(square::D4, square::C3, (Piece::Pawn, Color::White)),
        ];
        expected_black_moves.sort();

        let mut moves = generate_pawn_moves(&mut board, Color::Black);
        moves.sort();

        assert_eq!(expected_black_moves, moves);
    }

    #[test]
    fn test_generate_castle_moves_with_all_rights() {
        let mut board = Board::new();
        board.put(square::E1, Piece::King, Color::White).unwrap();
        board.put(square::A1, Piece::Rook, Color::White).unwrap();
        board.put(square::H1, Piece::Rook, Color::White).unwrap();
        board.put(square::E8, Piece::King, Color::Black).unwrap();
        board.put(square::A8, Piece::Rook, Color::Black).unwrap();
        board.put(square::H8, Piece::Rook, Color::Black).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let mut expected_white_moves: Vec<ChessMove> = vec![
            ChessMove::castle_kingside(Color::White),
            ChessMove::castle_queenside(Color::White),
        ];
        expected_white_moves.sort();

        let mut expected_black_moves: Vec<ChessMove> = vec![
            ChessMove::castle_kingside(Color::Black),
            ChessMove::castle_queenside(Color::Black),
        ];
        expected_black_moves.sort();

        let mut ray_table = RayTable::new();
        ray_table.populate();

        let mut white_moves = generate_castle_moves(&mut board, Color::White, &ray_table);
        white_moves.sort();

        let mut black_moves = generate_castle_moves(&mut board, Color::Black, &ray_table);
        black_moves.sort();

        assert_eq!(expected_white_moves, white_moves);
        assert_eq!(expected_black_moves, black_moves);
    }

    #[test]
    fn test_generate_castle_moves_under_attack() {
        let mut board = Board::new();
        board.put(square::E1, Piece::King, Color::White).unwrap();
        board.put(square::A1, Piece::Rook, Color::White).unwrap();
        board.put(square::H1, Piece::Rook, Color::White).unwrap();
        board.put(square::D7, Piece::Rook, Color::Black).unwrap(); // this makes white queenside castle impossible

        board.put(square::E8, Piece::King, Color::Black).unwrap();
        board.put(square::A8, Piece::Rook, Color::Black).unwrap();
        board.put(square::H8, Piece::Rook, Color::Black).unwrap();
        board.put(square::A3, Piece::Bishop, Color::White).unwrap(); // this makes black kingside castle impossible

        println!("Testing board:\n{}", board.to_ascii());

        let mut expected_white_moves: Vec<ChessMove> =
            vec![ChessMove::castle_kingside(Color::White)];
        expected_white_moves.sort();

        let mut expected_black_moves: Vec<ChessMove> =
            vec![ChessMove::castle_queenside(Color::Black)];
        expected_black_moves.sort();

        let mut ray_table = RayTable::new();
        ray_table.populate();
        targets::generate_attack_targets(&board, Color::Black, &ray_table);

        let mut white_moves = generate_castle_moves(&mut board, Color::White, &ray_table);
        white_moves.sort();
        let mut black_moves = generate_castle_moves(&mut board, Color::Black, &ray_table);
        black_moves.sort();

        assert_eq!(expected_white_moves, white_moves);
        assert_eq!(expected_black_moves, black_moves);
    }

    #[test]
    pub fn test_generate_castle_moves_blocked() {
        let mut board = Board::new();
        board.put(square::E1, Piece::King, Color::White).unwrap();
        board.put(square::A1, Piece::Rook, Color::White).unwrap();
        board.put(square::H1, Piece::Rook, Color::White).unwrap();
        board.put(square::B1, Piece::Bishop, Color::White).unwrap();
        board.put(square::G1, Piece::Knight, Color::Black).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        let expected_white_moves: Vec<ChessMove> = vec![];
        let white_moves =
            generate_castle_moves(&mut board, Color::White, &RayTable::new().populate());

        assert_eq!(expected_white_moves, white_moves);
    }
}
