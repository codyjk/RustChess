use crate::board::piece::Piece;
use crate::board::Board;
use crate::chess_move::chess_move::ChessMove;
use crate::chess_move::chess_move_effect::ChessMoveEffect;
use crate::move_generator::ChessMoveList;

/// Sorts a list of chess moves according to the following priority (from first to last):
/// 1. Checkmates
/// 2. Checks
/// 3. Captures
/// 4. Pawn promotions
/// 5. Rook moves
/// 6. Knight moves
/// 7. Bishop moves
/// 8. Pawn moves
/// 9. Other moves (including castling)
///
/// Sorting in this way will likely put the "best" moves first, which will greatly
/// improve the efficiency of alpha-beta search, by allowing later search nodes to
/// be pruned early.
pub fn sort_chess_moves(moves: &mut ChessMoveList, board: &Board) {
    moves.sort_by(|a, b| compare_moves(a, b, board));
}

fn compare_moves(a: &ChessMove, b: &ChessMove, board: &Board) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    match (effect_priority(a), effect_priority(b)) {
        (x, y) if x != y => x.cmp(&y),
        _ => match (is_capture(a), is_capture(b)) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => compare_move_types(a, b, board),
        },
    }
}

fn effect_priority(chess_move: &ChessMove) -> u8 {
    match chess_move.effect() {
        Some(ChessMoveEffect::Checkmate) => 0,
        Some(ChessMoveEffect::Check) => 1,
        _ => 2,
    }
}

fn is_capture(chess_move: &ChessMove) -> bool {
    chess_move.captures().is_some()
}

fn compare_move_types(a: &ChessMove, b: &ChessMove, board: &Board) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    match (a, b) {
        (ChessMove::PawnPromotion(_), ChessMove::PawnPromotion(_)) => Ordering::Equal,
        (ChessMove::PawnPromotion(_), _) => Ordering::Less,
        (_, ChessMove::PawnPromotion(_)) => Ordering::Greater,
        _ => compare_piece_types(get_piece_type(a, board), get_piece_type(b, board)),
    }
}

fn get_piece_type(chess_move: &ChessMove, board: &Board) -> Option<Piece> {
    match chess_move {
        ChessMove::Standard(m) => board.get(m.from_square()).map(|(piece, _)| piece),
        ChessMove::PawnPromotion(_) => Some(Piece::Pawn),
        ChessMove::EnPassant(_) => Some(Piece::Pawn),
        ChessMove::Castle(_) => Some(Piece::King),
    }
}

fn compare_piece_types(a: Option<Piece>, b: Option<Piece>) -> std::cmp::Ordering {
    let piece_priority = |piece: Option<Piece>| match piece {
        Some(Piece::Rook) => 0,
        Some(Piece::Knight) => 1,
        Some(Piece::Bishop) => 2,
        Some(Piece::Pawn) => 3,
        _ => 4,
    };

    piece_priority(a).cmp(&piece_priority(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::color::Color;
    use crate::board::piece::Piece;
    use crate::chess_move::capture::Capture;
    use crate::chess_move::castle::CastleChessMove;
    use crate::chess_move::chess_move_effect::ChessMoveEffect;
    use crate::chess_move::en_passant::EnPassantChessMove;
    use crate::chess_move::pawn_promotion::PawnPromotionChessMove;
    use crate::chess_move::standard::StandardChessMove;
    use crate::{
        castle_kingside, check_move, checkmate_move, chess_position, en_passant_move, promotion,
        std_move,
    };
    use common::bitboard::bitboard::Bitboard;
    use common::bitboard::square::*;

    fn create_test_board() -> Board {
        chess_position! {
            r...k..r
            ....p...
            ..n.....
            ...p....
            ....P...
            ..B..N..
            ........
            R...QK..
        }
    }

    #[test]
    fn test_sort_chess_moves() {
        let board = create_test_board();
        let mut moves = ChessMoveList::new();

        // Create various types of moves
        moves.push(std_move!(E4, E5));
        moves.push(std_move!(E4, D5, Capture(Piece::Pawn)));
        moves.push(check_move!(std_move!(D1, D5, Capture(Piece::Pawn))));
        moves.push(checkmate_move!(std_move!(D1, E8, Capture(Piece::Queen))));
        moves.push(promotion!(E7, E8, None, Piece::Queen));
        moves.push(en_passant_move!(E5, D6));
        moves.push(castle_kingside!(Color::White));
        moves.push(std_move!(A1, A3));
        moves.push(std_move!(F3, G5));
        moves.push(std_move!(C3, E5));
        moves.push(std_move!(E2, E4));

        // Sort the moves
        println!("before sort: {:?}", moves);
        sort_chess_moves(&mut moves, &board);
        println!("after sort: {:?}", moves);

        // Check if the moves are in the correct order
        assert_eq!(moves[0].effect(), Some(ChessMoveEffect::Checkmate));
        assert_eq!(moves[1].effect(), Some(ChessMoveEffect::Check));
        assert!(moves[2].captures().is_some());
        assert!(matches!(moves[3], ChessMove::EnPassant(_)));
        assert!(matches!(moves[4], ChessMove::PawnPromotion(_)));
        assert!(
            matches!(&moves[5], ChessMove::Standard(m) if board.get(m.from_square()).unwrap().0 == Piece::Rook)
        );
        assert!(
            matches!(&moves[6], ChessMove::Standard(m) if board.get(m.from_square()).unwrap().0 == Piece::Knight)
        );
        assert!(
            matches!(&moves[7], ChessMove::Standard(m) if board.get(m.from_square()).unwrap().0 == Piece::Bishop)
        );
        assert!(
            matches!(&moves[8], ChessMove::Standard(m) if board.get(m.from_square()).unwrap().0 == Piece::Pawn)
        );
        assert!(matches!(moves[9], ChessMove::Castle(_)));
        assert!(matches!(moves[10], ChessMove::Standard(_)));
    }

    #[test]
    fn test_sort_only_standard_moves() {
        let board = create_test_board();
        let mut moves = ChessMoveList::new();

        // Create only standard moves
        moves.push(std_move!(E4, E5));
        moves.push(std_move!(F3, G5));
        moves.push(std_move!(A1, A3));
        moves.push(std_move!(C3, E5));

        println!("before sort: {:?}", moves);
        sort_chess_moves(&mut moves, &board);
        println!("after sort: {:?}", moves);

        // Check if the moves are in the correct order: Rook, Knight, Bishop, Pawn
        assert!(
            matches!(&moves[0], ChessMove::Standard(m) if board.get(m.from_square()).unwrap().0 == Piece::Rook)
        );
        assert!(
            matches!(&moves[1], ChessMove::Standard(m) if board.get(m.from_square()).unwrap().0 == Piece::Knight)
        );
        assert!(
            matches!(&moves[2], ChessMove::Standard(m) if board.get(m.from_square()).unwrap().0 == Piece::Bishop)
        );
        assert!(
            matches!(&moves[3], ChessMove::Standard(m) if board.get(m.from_square()).unwrap().0 == Piece::Pawn)
        );
    }

    #[test]
    fn test_sort_with_multiple_checks_and_captures() {
        let board = create_test_board();
        let mut moves = ChessMoveList::new();

        moves.push(std_move!(F3, D5, Capture(Piece::Pawn)));
        moves.push(check_move!(std_move!(D1, D5, Capture(Piece::Pawn))));
        moves.push(std_move!(E4, D5, Capture(Piece::Pawn)));
        moves.push(check_move!(std_move!(G3, E5)));

        println!("before sort: {:?}", moves);
        sort_chess_moves(&mut moves, &board);
        println!("after sort: {:?}", moves);

        // Checks should come before captures, but order within checks and captures should be preserved
        assert_eq!(moves[0].effect(), Some(ChessMoveEffect::Check));
        assert_eq!(moves[1].effect(), Some(ChessMoveEffect::Check));
        assert!(moves[2].captures().is_some());
        assert!(moves[3].captures().is_some());
        assert_eq!(
            moves[0].from_square(),
            D1,
            "expected:\n{}\nbut got:\n{}",
            D1,
            moves[0].from_square()
        );
        assert_eq!(
            moves[1].from_square(),
            G3,
            "expected:\n{}\nbut got:\n{}",
            G3,
            moves[1].from_square()
        );
        assert_eq!(
            moves[2].from_square(),
            F3,
            "expected:\n{}\nbut got:\n{}",
            F3,
            moves[2].from_square()
        );
        assert_eq!(
            moves[3].from_square(),
            E4,
            "expected:\n{}\nbut got:\n{}",
            E4,
            moves[3].from_square()
        );
    }
}
