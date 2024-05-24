use crate::board::square::*;
pub mod ray_table;
pub mod targets;

use crate::board::bitboard::{A_FILE, H_FILE, RANK_1, RANK_8};
use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::{
    Board, BLACK_KINGSIDE_RIGHTS, BLACK_QUEENSIDE_RIGHTS, WHITE_KINGSIDE_RIGHTS,
    WHITE_QUEENSIDE_RIGHTS,
};
use crate::chess_move::castle::CastleChessMove;
use crate::chess_move::chess_move_collection::ChessMoveCollection;
use crate::chess_move::en_passant::EnPassantChessMove;
use crate::chess_move::pawn_promotion::PawnPromotionChessMove;
use crate::chess_move::standard::StandardChessMove;
use targets::{PieceTarget, Targets};

pub const PAWN_PROMOTIONS: [Piece; 4] = [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight];

pub fn generate_valid_moves(
    board: &mut Board,
    color: Color,
    targets: &mut Targets,
) -> ChessMoveCollection {
    let mut moves = ChessMoveCollection::new();

    generate_knight_moves(&mut moves, board, color, targets);
    generate_rook_moves(&mut moves, board, color, targets);
    generate_bishop_moves(&mut moves, board, color, targets);
    generate_queen_moves(&mut moves, board, color, targets);
    generate_king_moves(&mut moves, board, color, targets);
    generate_pawn_moves(&mut moves, board, color);
    generate_castle_moves(&mut moves, board, color, targets);
    remove_invalid_moves(&mut moves, board, color, targets);

    moves
}

fn generate_pawn_moves(moves: &mut ChessMoveCollection, board: &Board, color: Color) {
    // `generate_pawn_targets` blindly generates all pawn "targets": squares
    // that pawns can either move or capture. to get promotions, we will apply
    // some special logic to find the targets that are at the end of the board,
    // and then expand those targets into the candidate promotion pieces.
    let piece_targets = targets::generate_pawn_targets(board, color);
    let mut all_pawn_moves = ChessMoveCollection::new();
    expand_piece_targets(&mut all_pawn_moves, board, color, piece_targets);

    let (mut standard_pawn_moves, promotable_pawn_moves) = all_pawn_moves.partition(|chess_move| {
        let to_square = chess_move.to_square();
        let promotion_rank = match color {
            Color::White => RANK_8,
            Color::Black => RANK_1,
        };
        to_square & promotion_rank == 0
    });

    for promotable_pawn_move in promotable_pawn_moves.iter() {
        let from_square = promotable_pawn_move.from_square();
        let to_square = promotable_pawn_move.to_square();
        let capture = promotable_pawn_move.capture();
        for &promotion in &PAWN_PROMOTIONS {
            moves.push(Box::new(PawnPromotionChessMove::new(
                from_square,
                to_square,
                capture,
                promotion,
            )));
        }
    }
    moves.append(&mut standard_pawn_moves);
    generate_en_passant_moves(moves, board, color);
}

fn generate_en_passant_moves(moves: &mut ChessMoveCollection, board: &Board, color: Color) {
    let en_passant_target = board.peek_en_passant_target();

    if en_passant_target == 0 {
        return;
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
        moves.push(Box::new(EnPassantChessMove::new(
            from_square,
            en_passant_target,
        )));
    }

    if attacks_east & en_passant_target > 0 {
        let from_square = match color {
            Color::White => en_passant_target >> 7,
            Color::Black => en_passant_target << 9,
        };
        moves.push(Box::new(EnPassantChessMove::new(
            from_square,
            en_passant_target,
        )));
    }
}

fn generate_knight_moves(
    moves: &mut ChessMoveCollection,
    board: &Board,
    color: Color,
    targets: &Targets,
) {
    expand_piece_targets(
        moves,
        board,
        color,
        targets::generate_piece_targets(board, color, Piece::Knight, targets),
    )
}

pub fn generate_rook_moves(
    moves: &mut ChessMoveCollection,
    board: &Board,
    color: Color,
    targets: &Targets,
) {
    let piece_targets = targets::generate_rook_targets(board, color, targets);
    expand_piece_targets(moves, board, color, piece_targets)
}

fn generate_bishop_moves(
    moves: &mut ChessMoveCollection,
    board: &Board,
    color: Color,
    targets: &Targets,
) {
    let piece_targets = targets::generate_bishop_targets(board, color, targets);
    expand_piece_targets(moves, board, color, piece_targets)
}

fn generate_queen_moves(
    moves: &mut ChessMoveCollection,
    board: &Board,
    color: Color,
    targets: &Targets,
) {
    let piece_targets = targets::generate_queen_targets(board, color, targets);
    expand_piece_targets(moves, board, color, piece_targets)
}

fn expand_piece_targets(
    moves: &mut ChessMoveCollection,
    board: &Board,
    color: Color,
    piece_targets: Vec<PieceTarget>,
) {
    // TODO(codyjk): Do we need to loop over every square?
    for (piece, target_squares) in piece_targets {
        let piece_sq = assert(piece);
        for &target in &ORDERED {
            if target_squares & target == 0 {
                continue;
            }

            let capture = board
                .pieces(color.opposite())
                .get(target)
                .map(|piece| (piece, color.opposite()));

            moves.push(Box::new(StandardChessMove::new(piece_sq, target, capture)));
        }
    }
}

fn generate_king_moves(
    moves: &mut ChessMoveCollection,
    board: &Board,
    color: Color,
    targets: &Targets,
) {
    expand_piece_targets(
        moves,
        board,
        color,
        targets::generate_piece_targets(board, color, Piece::King, targets),
    )
}

fn generate_castle_moves(
    moves: &mut ChessMoveCollection,
    board: &Board,
    color: Color,
    targets: &mut Targets,
) {
    let attacked_squares = targets::generate_attack_targets(board, color.opposite(), targets);

    if board.pieces(color).locate(Piece::King) & attacked_squares > 0 {
        return;
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
        Color::White => (F1, D1),
        Color::Black => (F8, D8),
    };

    let queenside_rook_transit_square = match color {
        Color::White => B1,
        Color::Black => B8,
    };

    let (kingside_target_square, queenside_target_square) = match color {
        Color::White => (G1, C1),
        Color::Black => (G8, C8),
    };

    let occupied = board.occupied();

    if kingside_rights > 0
        && board.get(kingside_transit_square).is_none()
        && kingside_transit_square & attacked_squares == 0
        && kingside_transit_square & occupied == 0
        && kingside_target_square & occupied == 0
    {
        moves.push(Box::new(CastleChessMove::castle_kingside(color)));
    }

    if queenside_rights > 0
        && board.get(queenside_transit_square).is_none()
        && queenside_transit_square & attacked_squares == 0
        && queenside_transit_square & occupied == 0
        && queenside_rook_transit_square & occupied == 0
        && queenside_target_square & occupied == 0
    {
        moves.push(Box::new(CastleChessMove::castle_queenside(color)));
    }
}

fn remove_invalid_moves(
    candidates: &mut ChessMoveCollection,
    board: &mut Board,
    color: Color,
    targets: &mut Targets,
) {
    let mut valid_moves = ChessMoveCollection::new();

    // simulate each chess_move and see if it leaves the player's king in check.
    // if it does, it's invalid.
    for chess_move in candidates.drain() {
        chess_move.apply(board).unwrap();
        let king = board.pieces(color).locate(Piece::King);
        let attacked_squares = targets::generate_attack_targets(board, color.opposite(), targets);
        chess_move.undo(board).unwrap();

        if king & attacked_squares == 0 {
            valid_moves.push(chess_move);
        }
    }

    candidates.append(&mut valid_moves);
}

pub fn count_positions(depth: u8, board: &mut Board, targets: &mut Targets, color: Color) -> usize {
    let candidates = generate_valid_moves(board, color, targets);
    let mut count = candidates.len();

    if depth == 0 {
        return count;
    }

    let next_color = color.opposite();

    for chess_move in candidates.iter() {
        chess_move.apply(board).unwrap();
        count += count_positions(depth - 1, board, targets, next_color);
        chess_move.undo(board).unwrap();
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess_move::ChessMove;
    use crate::{castle_kingside, castle_queenside, chess_moves, promotion, std_move};

    #[test]
    fn test_generate_pawn_moves() {
        let mut board = Board::new();
        board.put(A4, Piece::Pawn, Color::White).unwrap();
        board.put(A5, Piece::Pawn, Color::Black).unwrap();
        board.put(D2, Piece::Pawn, Color::White).unwrap();
        board.put(D7, Piece::Pawn, Color::Black).unwrap();
        board.put(G6, Piece::Pawn, Color::White).unwrap();
        board.put(H7, Piece::Pawn, Color::Black).unwrap();
        board.put(B7, Piece::Pawn, Color::White).unwrap();
        board.put(C8, Piece::Rook, Color::Black).unwrap();
        board.put(A2, Piece::Pawn, Color::Black).unwrap();
        board.put(F2, Piece::Pawn, Color::White).unwrap();
        board.put(F3, Piece::Rook, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        let mut expected_white_moves = chess_moves![
            std_move!(D2, D3),
            std_move!(D2, D4),
            std_move!(G6, G7),
            std_move!(G6, H7, (Piece::Pawn, Color::Black)),
            promotion!(B7, B8, None, Piece::Queen),
            promotion!(B7, B8, None, Piece::Rook),
            promotion!(B7, B8, None, Piece::Knight),
            promotion!(B7, B8, None, Piece::Bishop),
            promotion!(B7, C8, Some((Piece::Rook, Color::Black)), Piece::Queen),
            promotion!(B7, C8, Some((Piece::Rook, Color::Black)), Piece::Rook),
            promotion!(B7, C8, Some((Piece::Rook, Color::Black)), Piece::Knight),
            promotion!(B7, C8, Some((Piece::Rook, Color::Black)), Piece::Bishop),
        ];
        expected_white_moves.sort();

        let mut expected_black_moves = chess_moves![
            std_move!(D7, D6),
            std_move!(D7, D5),
            std_move!(H7, H6),
            std_move!(H7, H5),
            std_move!(H7, G6, (Piece::Pawn, Color::White)),
            promotion!(A2, A1, None, Piece::Queen),
            promotion!(A2, A1, None, Piece::Rook),
            promotion!(A2, A1, None, Piece::Knight),
            promotion!(A2, A1, None, Piece::Bishop),
        ];
        expected_black_moves.sort();

        let mut white_moves = ChessMoveCollection::new();
        generate_pawn_moves(&mut white_moves, &board, Color::White);
        white_moves.sort();
        assert_eq!(expected_white_moves, white_moves);

        let mut black_moves = ChessMoveCollection::new();
        generate_pawn_moves(&mut black_moves, &board, Color::Black);
        black_moves.sort();
        assert_eq!(expected_black_moves, black_moves);
    }

    #[test]
    fn test_generate_pawn_double_moves() {
        let mut board = Board::new();
        board.put(B2, Piece::Pawn, Color::White).unwrap();
        board.put(C2, Piece::Pawn, Color::White).unwrap();
        board.put(C3, Piece::Pawn, Color::White).unwrap();
        board.put(E2, Piece::Pawn, Color::White).unwrap();
        board.put(E3, Piece::Pawn, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        let mut expected_moves =
            chess_moves![std_move!(B2, B3), std_move!(B2, B4), std_move!(C3, C4),];
        expected_moves.sort();

        let mut moves = ChessMoveCollection::new();
        generate_pawn_moves(&mut moves, &board, Color::White);
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_knight_moves() {
        let mut board = Board::new();
        let targets = Targets::new();

        board.put(C3, Piece::Knight, Color::White).unwrap();
        board.put(E4, Piece::Pawn, Color::White).unwrap();
        board.put(D5, Piece::Pawn, Color::Black).unwrap();
        board.put(H6, Piece::Knight, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        let mut expected_white_moves = chess_moves![
            std_move!(C3, D5, (Piece::Pawn, Color::Black)),
            std_move!(C3, E2),
            std_move!(C3, D1),
            std_move!(C3, B5),
            std_move!(C3, A4),
            std_move!(C3, A2),
            std_move!(C3, B1),
        ];
        expected_white_moves.sort();

        let mut expected_black_moves = chess_moves![
            std_move!(H6, G8),
            std_move!(H6, F7),
            std_move!(H6, F5),
            std_move!(H6, G4),
        ];
        expected_black_moves.sort();

        let mut white_moves = ChessMoveCollection::new();
        generate_knight_moves(&mut white_moves, &board, Color::White, &targets);
        white_moves.sort();
        assert_eq!(expected_white_moves, white_moves);

        let mut black_moves = ChessMoveCollection::new();
        generate_knight_moves(&mut black_moves, &board, Color::Black, &targets);
        black_moves.sort();
        assert_eq!(expected_black_moves, black_moves);
    }

    #[test]
    fn test_generate_rook_moves_1() {
        let mut board = Board::new();
        board.put(A3, Piece::Pawn, Color::White).unwrap();
        board.put(H3, Piece::Pawn, Color::Black).unwrap();
        board.put(C3, Piece::Rook, Color::White).unwrap();
        board.put(C1, Piece::King, Color::White).unwrap();
        board.put(C7, Piece::Pawn, Color::White).unwrap();
        println!("Testing board:\n{}", board);

        let mut expected_moves = chess_moves![
            std_move!(C3, C2),
            std_move!(C3, C4),
            std_move!(C3, C5),
            std_move!(C3, C6),
            std_move!(C3, B3),
            std_move!(C3, D3),
            std_move!(C3, E3),
            std_move!(C3, F3),
            std_move!(C3, G3),
            std_move!(C3, H3, (Piece::Pawn, Color::Black)),
        ];
        expected_moves.sort();

        let mut moves = ChessMoveCollection::new();
        generate_rook_moves(&mut moves, &board, Color::White, &Targets::new());
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_rook_moves_2() {
        let mut board = Board::new();
        board.put(A4, Piece::Pawn, Color::White).unwrap();
        board.put(A2, Piece::Rook, Color::White).unwrap();
        board.put(B2, Piece::Pawn, Color::White).unwrap();
        println!("Testing board:\n{}", board);

        let mut expected_moves = chess_moves![std_move!(A2, A1), std_move!(A2, A3)];
        expected_moves.sort();

        let mut moves = ChessMoveCollection::new();
        generate_rook_moves(&mut moves, &board, Color::White, &Targets::new());
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_bishop_moves() {
        let mut board = Board::new();
        board.put(E5, Piece::Bishop, Color::White).unwrap();
        board.put(A1, Piece::Pawn, Color::White).unwrap();
        board.put(C3, Piece::Pawn, Color::White).unwrap();
        board.put(C7, Piece::Pawn, Color::White).unwrap();
        board.put(G7, Piece::Pawn, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        let mut expected_moves = chess_moves![
            std_move!(E5, D4),
            std_move!(E5, D6),
            std_move!(E5, F4),
            std_move!(E5, F6),
            std_move!(E5, G3),
            std_move!(E5, G7, (Piece::Pawn, Color::Black)),
            std_move!(E5, H2),
        ];
        expected_moves.sort();

        let mut moves = ChessMoveCollection::new();
        generate_bishop_moves(&mut moves, &board, Color::White, &Targets::new());
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_queen_moves() {
        let mut board = Board::new();
        board.put(E5, Piece::Queen, Color::White).unwrap();
        board.put(E6, Piece::Pawn, Color::White).unwrap();
        board.put(E7, Piece::Pawn, Color::Black).unwrap();
        board.put(H8, Piece::Pawn, Color::Black).unwrap();
        board.put(B2, Piece::Pawn, Color::White).unwrap();
        board.put(B5, Piece::Pawn, Color::White).unwrap();
        board.put(G3, Piece::Pawn, Color::Black).unwrap();
        board.put(H2, Piece::Pawn, Color::White).unwrap();
        println!("Testing board:\n{}", board);

        let mut expected_moves = chess_moves![
            // North - no moves
            // NorthEast
            std_move!(E5, F6),
            std_move!(E5, G7),
            std_move!(E5, H8, (Piece::Pawn, Color::Black)),
            // East
            std_move!(E5, F5),
            std_move!(E5, G5),
            std_move!(E5, H5),
            // SouthEast
            std_move!(E5, F4),
            std_move!(E5, G3, (Piece::Pawn, Color::Black)),
            // South
            std_move!(E5, E4),
            std_move!(E5, E3),
            std_move!(E5, E2),
            std_move!(E5, E1),
            // SouthWest
            std_move!(E5, D4),
            std_move!(E5, C3),
            // West
            std_move!(E5, D5),
            std_move!(E5, C5),
            // NorthWest
            std_move!(E5, D6),
            std_move!(E5, C7),
            std_move!(E5, B8),
        ];
        expected_moves.sort();

        let mut moves = ChessMoveCollection::new();
        generate_queen_moves(&mut moves, &board, Color::White, &Targets::new());
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_king_moves_corner() {
        let mut board = Board::new();
        board.put(A1, Piece::King, Color::White).unwrap();
        println!("Testing board:\n{}", board);

        let mut expected_moves =
            chess_moves![std_move!(A1, A2), std_move!(A1, B1), std_move!(A1, B2),];
        expected_moves.sort();

        let mut moves = ChessMoveCollection::new();
        generate_king_moves(&mut moves, &board, Color::White, &Targets::new());
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_king_moves_edge_south() {
        let mut board = Board::new();
        board.put(E1, Piece::King, Color::White).unwrap();
        board.put(D2, Piece::Pawn, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        let mut expected_moves = chess_moves![
            std_move!(E1, D1),
            std_move!(E1, D2, (Piece::Pawn, Color::Black)),
            std_move!(E1, E2),
            std_move!(E1, F1),
            std_move!(E1, F2),
        ];
        expected_moves.sort();

        let mut moves = ChessMoveCollection::new();
        generate_king_moves(&mut moves, &board, Color::White, &Targets::new());
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_king_moves_middle() {
        let mut board = Board::new();
        board.put(E5, Piece::King, Color::White).unwrap();
        board.put(E6, Piece::Pawn, Color::White).unwrap();
        board.put(E4, Piece::Pawn, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        let mut expected_moves = chess_moves![
            std_move!(E5, D4),
            std_move!(E5, D5),
            std_move!(E5, D6),
            std_move!(E5, E4, (Piece::Pawn, Color::Black)),
            std_move!(E5, F4),
            std_move!(E5, F5),
            std_move!(E5, F6),
        ];
        expected_moves.sort();

        let mut moves = ChessMoveCollection::new();
        generate_king_moves(&mut moves, &board, Color::White, &Targets::new());
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_en_passant_moves() {
        let mut board = Board::new();
        board.put(C2, Piece::Pawn, Color::White).unwrap();
        board.put(D4, Piece::Pawn, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        let move_that_reveals_en_passant_target = std_move!(C2, C4);
        move_that_reveals_en_passant_target
            .apply(&mut board)
            .unwrap();
        assert_eq!(C3, board.peek_en_passant_target());

        let mut expected_black_moves =
            chess_moves![std_move!(D4, D3), EnPassantChessMove::new(D4, C3)];
        expected_black_moves.sort();

        let mut moves = ChessMoveCollection::new();
        generate_pawn_moves(&mut moves, &board, Color::Black);
        moves.sort();

        assert_eq!(expected_black_moves, moves);
    }

    #[test]
    fn test_generate_castle_moves_with_all_rights() {
        let mut board = Board::new();
        board.put(E1, Piece::King, Color::White).unwrap();
        board.put(A1, Piece::Rook, Color::White).unwrap();
        board.put(H1, Piece::Rook, Color::White).unwrap();
        board.put(E8, Piece::King, Color::Black).unwrap();
        board.put(A8, Piece::Rook, Color::Black).unwrap();
        board.put(H8, Piece::Rook, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        let mut expected_white_moves = chess_moves![
            castle_kingside!(Color::White),
            castle_queenside!(Color::White),
        ];
        expected_white_moves.sort();

        let mut expected_black_moves = chess_moves![
            castle_kingside!(Color::Black),
            castle_queenside!(Color::Black),
        ];
        expected_black_moves.sort();

        let mut targets = Targets::new();

        let mut white_moves = ChessMoveCollection::new();
        generate_castle_moves(&mut white_moves, &board, Color::White, &mut targets);
        white_moves.sort();

        let mut black_moves = ChessMoveCollection::new();
        generate_castle_moves(&mut black_moves, &board, Color::Black, &mut targets);
        black_moves.sort();

        assert_eq!(
            expected_white_moves, white_moves,
            "failed to generate white castling moves"
        );
        assert_eq!(
            expected_black_moves, black_moves,
            "failed to generate black castling moves"
        );
    }

    #[test]
    fn test_generate_castle_moves_under_attack() {
        let mut board = Board::new();
        board.put(E1, Piece::King, Color::White).unwrap();
        board.put(A1, Piece::Rook, Color::White).unwrap();
        board.put(H1, Piece::Rook, Color::White).unwrap();
        board.put(D7, Piece::Rook, Color::Black).unwrap(); // this makes white queenside castle impossible

        board.put(E8, Piece::King, Color::Black).unwrap();
        board.put(A8, Piece::Rook, Color::Black).unwrap();
        board.put(H8, Piece::Rook, Color::Black).unwrap();
        board.put(A3, Piece::Bishop, Color::White).unwrap(); // this makes black kingside castle impossible

        println!("Testing board:\n{}", board);

        let mut expected_white_moves = chess_moves![castle_kingside!(Color::White)];
        expected_white_moves.sort();

        let mut expected_black_moves = chess_moves![castle_queenside!(Color::Black)];
        expected_black_moves.sort();

        let mut targets = Targets::new();
        targets::generate_attack_targets(&board, Color::Black, &mut targets);

        let mut white_moves = ChessMoveCollection::new();
        generate_castle_moves(&mut white_moves, &board, Color::White, &mut targets);
        white_moves.sort();

        let mut black_moves = ChessMoveCollection::new();
        generate_castle_moves(&mut black_moves, &board, Color::Black, &mut targets);
        black_moves.sort();

        assert_eq!(expected_white_moves, white_moves);
        assert_eq!(expected_black_moves, black_moves);
    }

    #[test]
    pub fn test_generate_castle_moves_blocked() {
        let mut board = Board::new();
        board.put(E1, Piece::King, Color::White).unwrap();
        board.put(A1, Piece::Rook, Color::White).unwrap();
        board.put(H1, Piece::Rook, Color::White).unwrap();
        board.put(B1, Piece::Bishop, Color::White).unwrap();
        board.put(G1, Piece::Knight, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        let expected_white_moves = ChessMoveCollection::new();
        let mut white_moves = ChessMoveCollection::new();
        generate_castle_moves(&mut white_moves, &board, Color::White, &mut Targets::new());

        assert_eq!(expected_white_moves, white_moves);
    }
}
