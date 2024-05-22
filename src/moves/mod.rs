use crate::board::square::*;
pub mod board;
pub mod chess_move;
pub mod ray_table;
pub mod targets;

use crate::board::bitboard::{A_FILE, H_FILE, RANK_1, RANK_8};
use crate::board::color::Color;
use crate::board::error::BoardError;
use crate::board::piece::Piece;
use crate::board::{
    Board, BLACK_KINGSIDE_RIGHTS, BLACK_QUEENSIDE_RIGHTS, WHITE_KINGSIDE_RIGHTS,
    WHITE_QUEENSIDE_RIGHTS,
};
use chess_move::ChessMove;
use targets::{PieceTarget, Targets};

pub const PAWN_PROMOTIONS: [Piece; 4] = [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight];

pub fn generate(board: &mut Board, color: Color, targets: &mut Targets) -> Vec<ChessMove> {
    let mut moves = vec![];

    moves.append(&mut generate_knight_moves(board, color, targets));
    moves.append(&mut generate_bishop_moves(board, color, targets));
    moves.append(&mut generate_pawn_moves(board, color));
    moves.append(&mut generate_rook_moves(board, color, targets));
    moves.append(&mut generate_queen_moves(board, color, targets));
    moves.append(&mut generate_castle_moves(board, color, targets));
    moves.append(&mut generate_king_moves(board, color, targets));

    moves = remove_invalid_moves(moves, board, color, targets);

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

fn generate_knight_moves(board: &Board, color: Color, targets: &Targets) -> Vec<ChessMove> {
    expand_piece_targets(
        board,
        color,
        targets::generate_piece_targets(board, color, Piece::Knight, targets),
    )
}

pub fn generate_rook_moves(board: &Board, color: Color, targets: &Targets) -> Vec<ChessMove> {
    let piece_targets = targets::generate_rook_targets(board, color, targets);
    expand_piece_targets(board, color, piece_targets)
}

fn generate_bishop_moves(board: &Board, color: Color, targets: &Targets) -> Vec<ChessMove> {
    let piece_targets = targets::generate_bishop_targets(board, color, targets);
    expand_piece_targets(board, color, piece_targets)
}

fn generate_queen_moves(board: &Board, color: Color, targets: &Targets) -> Vec<ChessMove> {
    let piece_targets = targets::generate_queen_targets(board, color, targets);
    expand_piece_targets(board, color, piece_targets)
}

fn expand_piece_targets(
    board: &Board,
    color: Color,
    piece_targets: Vec<PieceTarget>,
) -> Vec<ChessMove> {
    let mut moves: Vec<ChessMove> = vec![];
    for (piece, target_squares) in piece_targets {
        let piece_sq = assert(piece);
        for &target in &ORDERED {
            if target_squares & target == 0 {
                continue;
            }

            let capture = match board.pieces(color.opposite()).get(target) {
                Some(piece) => Some((piece, color.opposite())),
                None => None,
            };

            moves.push(ChessMove::new(piece_sq, target, capture));
        }
    }
    moves
}

fn generate_king_moves(board: &Board, color: Color, targets: &Targets) -> Vec<ChessMove> {
    expand_piece_targets(
        board,
        color,
        targets::generate_piece_targets(board, color, Piece::King, targets),
    )
}

fn generate_castle_moves(board: &Board, color: Color, targets: &mut Targets) -> Vec<ChessMove> {
    let mut moves: Vec<ChessMove> = vec![];
    let attacked_squares = targets::generate_attack_targets(board, color.opposite(), targets);

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
    targets: &mut Targets,
) -> Vec<ChessMove> {
    let mut moves = vec![];

    // simulate each chessmove and see if it leaves the player's king in check.
    // if it does, it's invalid.
    for chessmove in candidates {
        board
            .apply(chessmove)
            .map_err(|e| enrich_error(board, chessmove, e))
            .unwrap();

        let king = board.pieces(color).locate(Piece::King);
        let attacked_squares = targets::generate_attack_targets(board, color.opposite(), targets);

        if king & attacked_squares == 0 {
            moves.push(chessmove);
        }

        board
            .undo(chessmove)
            .map_err(|e| enrich_error(board, chessmove, e))
            .unwrap();
    }

    moves
}

fn enrich_error(board: &Board, chessmove: ChessMove, error: BoardError) -> String {
    let enriched_error = format!(
        "error: {}\nmove:{}\nboard:\n{}fen:\n{}",
        error,
        chessmove,
        board,
        board.to_fen()
    );
    enriched_error
}

pub fn count_positions(depth: u8, board: &mut Board, targets: &mut Targets, color: Color) -> usize {
    let candidates = generate(board, color, targets);
    let mut count = candidates.len();

    if depth == 0 {
        return count;
    }

    let next_color = color.opposite();

    for chessmove in candidates {
        board.apply(chessmove).unwrap();
        count += count_positions(depth - 1, board, targets, next_color);
        board.undo(chessmove).unwrap();
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess_move;

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

        let mut expected_white_moves: Vec<ChessMove> = vec![
            chess_move!(D2, D3),
            chess_move!(D2, D4),
            chess_move!(G6, G7),
            chess_move!(G6, H7, (Piece::Pawn, Color::Black)),
            ChessMove::promote(B7, B8, None, Piece::Queen),
            ChessMove::promote(B7, B8, None, Piece::Rook),
            ChessMove::promote(B7, B8, None, Piece::Knight),
            ChessMove::promote(B7, B8, None, Piece::Bishop),
            ChessMove::promote(B7, C8, Some((Piece::Rook, Color::Black)), Piece::Queen),
            ChessMove::promote(B7, C8, Some((Piece::Rook, Color::Black)), Piece::Rook),
            ChessMove::promote(B7, C8, Some((Piece::Rook, Color::Black)), Piece::Knight),
            ChessMove::promote(B7, C8, Some((Piece::Rook, Color::Black)), Piece::Bishop),
        ];
        expected_white_moves.sort();

        let mut expected_black_moves: Vec<ChessMove> = vec![
            chess_move!(D7, D6),
            chess_move!(D7, D5),
            chess_move!(H7, H6),
            chess_move!(H7, H5),
            chess_move!(H7, G6, (Piece::Pawn, Color::White)),
            ChessMove::promote(A2, A1, None, Piece::Queen),
            ChessMove::promote(A2, A1, None, Piece::Rook),
            ChessMove::promote(A2, A1, None, Piece::Knight),
            ChessMove::promote(A2, A1, None, Piece::Bishop),
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
    fn test_generate_pawn_double_moves() {
        let mut board = Board::new();
        board.put(B2, Piece::Pawn, Color::White).unwrap();
        board.put(C2, Piece::Pawn, Color::White).unwrap();
        board.put(C3, Piece::Pawn, Color::White).unwrap();
        board.put(E2, Piece::Pawn, Color::White).unwrap();
        board.put(E3, Piece::Pawn, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        let mut expected_moves = vec![
            chess_move!(B2, B3),
            chess_move!(B2, B4),
            chess_move!(C3, C4),
        ];
        expected_moves.sort();

        let mut moves = generate_pawn_moves(&board, Color::White);
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

        let mut expected_white_moves: Vec<ChessMove> = vec![
            chess_move!(C3, D5, (Piece::Pawn, Color::Black)),
            chess_move!(C3, E2),
            chess_move!(C3, D1),
            chess_move!(C3, B5),
            chess_move!(C3, A4),
            chess_move!(C3, A2),
            chess_move!(C3, B1),
        ];
        expected_white_moves.sort();

        let mut expected_black_moves: Vec<ChessMove> = vec![
            chess_move!(H6, G8),
            chess_move!(H6, F7),
            chess_move!(H6, F5),
            chess_move!(H6, G4),
        ];
        expected_black_moves.sort();

        let mut white_moves = generate_knight_moves(&board, Color::White, &targets);
        white_moves.sort();
        assert_eq!(expected_white_moves, white_moves);

        let mut black_moves = generate_knight_moves(&board, Color::Black, &targets);
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

        let mut expected_moves: Vec<ChessMove> = vec![
            chess_move!(C3, C2),
            chess_move!(C3, C4),
            chess_move!(C3, C5),
            chess_move!(C3, C6),
            chess_move!(C3, B3),
            chess_move!(C3, D3),
            chess_move!(C3, E3),
            chess_move!(C3, F3),
            chess_move!(C3, G3),
            chess_move!(C3, H3, (Piece::Pawn, Color::Black)),
        ];
        expected_moves.sort();

        let mut moves = generate_rook_moves(&board, Color::White, &Targets::new());
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

        let mut expected_moves: Vec<ChessMove> = vec![chess_move!(A2, A1), chess_move!(A2, A3)];
        expected_moves.sort();

        let mut moves = generate_rook_moves(&board, Color::White, &Targets::new());
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

        let mut expected_moves: Vec<ChessMove> = vec![
            chess_move!(E5, D4),
            chess_move!(E5, D6),
            chess_move!(E5, F4),
            chess_move!(E5, F6),
            chess_move!(E5, G3),
            chess_move!(E5, G7, (Piece::Pawn, Color::Black)),
            chess_move!(E5, H2),
        ];
        expected_moves.sort();

        let mut moves = generate_bishop_moves(&board, Color::White, &Targets::new());
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

        let mut expected_moves: Vec<ChessMove> = vec![
            // North - no moves
            // NorthEast
            chess_move!(E5, F6),
            chess_move!(E5, G7),
            chess_move!(E5, H8, (Piece::Pawn, Color::Black)),
            // East
            chess_move!(E5, F5),
            chess_move!(E5, G5),
            chess_move!(E5, H5),
            // SouthEast
            chess_move!(E5, F4),
            chess_move!(E5, G3, (Piece::Pawn, Color::Black)),
            // South
            chess_move!(E5, E4),
            chess_move!(E5, E3),
            chess_move!(E5, E2),
            chess_move!(E5, E1),
            // SouthWest
            chess_move!(E5, D4),
            chess_move!(E5, C3),
            // West
            chess_move!(E5, D5),
            chess_move!(E5, C5),
            // NorthWest
            chess_move!(E5, D6),
            chess_move!(E5, C7),
            chess_move!(E5, B8),
        ];
        expected_moves.sort();

        let mut moves = generate_queen_moves(&board, Color::White, &Targets::new());
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_king_moves_corner() {
        let mut board = Board::new();
        board.put(A1, Piece::King, Color::White).unwrap();
        println!("Testing board:\n{}", board);

        let mut expected_moves: Vec<ChessMove> = vec![
            chess_move!(A1, A2),
            chess_move!(A1, B1),
            chess_move!(A1, B2),
        ];
        expected_moves.sort();

        let mut moves = generate_king_moves(&board, Color::White, &Targets::new());
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_king_moves_edge_south() {
        let mut board = Board::new();
        board.put(E1, Piece::King, Color::White).unwrap();
        board.put(D2, Piece::Pawn, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        let mut expected_moves: Vec<ChessMove> = vec![
            chess_move!(E1, D1),
            chess_move!(E1, D2, (Piece::Pawn, Color::Black)),
            chess_move!(E1, E2),
            chess_move!(E1, F1),
            chess_move!(E1, F2),
        ];
        expected_moves.sort();

        let mut moves = generate_king_moves(&board, Color::White, &Targets::new());
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

        let mut expected_moves: Vec<ChessMove> = vec![
            chess_move!(E5, D4),
            chess_move!(E5, D5),
            chess_move!(E5, D6),
            chess_move!(E5, E4, (Piece::Pawn, Color::Black)),
            chess_move!(E5, F4),
            chess_move!(E5, F5),
            chess_move!(E5, F6),
        ];
        expected_moves.sort();

        let mut moves = generate_king_moves(&board, Color::White, &Targets::new());
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_en_passant_moves() {
        let mut board = Board::new();
        board.put(C2, Piece::Pawn, Color::White).unwrap();
        board.put(D4, Piece::Pawn, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        let move_that_reveals_en_passant_target = chess_move!(C2, C4);
        board.apply(move_that_reveals_en_passant_target).unwrap();
        assert_eq!(C3, board.peek_en_passant_target());

        let mut expected_black_moves: Vec<ChessMove> = vec![
            chess_move!(D4, D3),
            ChessMove::en_passant(D4, C3, (Piece::Pawn, Color::White)),
        ];
        expected_black_moves.sort();

        let mut moves = generate_pawn_moves(&mut board, Color::Black);
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

        let mut targets = Targets::new();

        let mut white_moves = generate_castle_moves(&mut board, Color::White, &mut targets);
        white_moves.sort();

        let mut black_moves = generate_castle_moves(&mut board, Color::Black, &mut targets);
        black_moves.sort();

        assert_eq!(expected_white_moves, white_moves);
        assert_eq!(expected_black_moves, black_moves);
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

        let mut expected_white_moves: Vec<ChessMove> =
            vec![ChessMove::castle_kingside(Color::White)];
        expected_white_moves.sort();

        let mut expected_black_moves: Vec<ChessMove> =
            vec![ChessMove::castle_queenside(Color::Black)];
        expected_black_moves.sort();

        let mut targets = Targets::new();
        targets::generate_attack_targets(&board, Color::Black, &mut targets);

        let mut white_moves = generate_castle_moves(&mut board, Color::White, &mut targets);
        white_moves.sort();
        let mut black_moves = generate_castle_moves(&mut board, Color::Black, &mut targets);
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

        let expected_white_moves: Vec<ChessMove> = vec![];
        let white_moves = generate_castle_moves(&mut board, Color::White, &mut Targets::new());

        assert_eq!(expected_white_moves, white_moves);
    }
}
