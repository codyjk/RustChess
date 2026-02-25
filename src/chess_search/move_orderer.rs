//! Chess-specific move ordering for improved alpha-beta pruning.

use std::cell::RefCell;
use thread_local::ThreadLocal;

use crate::alpha_beta_searcher::{GameMove, MoveOrderer};
use crate::board::piece::Piece;
use crate::board::Board;
use crate::chess_move::chess_move::ChessMove;
use crate::evaluate::evaluation_tables::MATERIAL_VALUES;
use crate::prelude::*;

use super::history_table::HistoryTable;

static HISTORY_TABLE: ThreadLocal<RefCell<HistoryTable>> = ThreadLocal::new();

fn get_history_score(from: Square, to: Square) -> u32 {
    HISTORY_TABLE
        .get_or(|| RefCell::new(HistoryTable::new()))
        .borrow()
        .score(from, to)
}

/// Records a history cutoff for a quiet move.
pub fn record_history_cutoff(from: Square, to: Square, depth: u8) {
    HISTORY_TABLE
        .get_or(|| RefCell::new(HistoryTable::new()))
        .borrow_mut()
        .record_cutoff(from, to, depth);
}

/// Clears the history table.
pub fn clear_history() {
    if let Some(storage) = HISTORY_TABLE.get() {
        storage.borrow_mut().clear();
    }
}

/// Chess move orderer that prioritizes captures (MVV-LVA), promotions,
/// then uses history heuristic for quiet moves, then piece moves by type.
#[derive(Clone, Default, Debug)]
pub struct ChessMoveOrderer;

impl MoveOrderer<Board, ChessMove> for ChessMoveOrderer {
    #[inline]
    fn order_moves(&self, moves: &mut [ChessMove], state: &Board) {
        moves.sort_by(|a, b| compare_moves(a, b, state));
    }

    #[inline]
    fn pick_next(&self, moves: &mut [ChessMove], index: usize, state: &Board) {
        if index + 1 >= moves.len() {
            return;
        }
        let best_idx = (index..moves.len())
            .min_by(|&a, &b| compare_moves(&moves[a], &moves[b], state))
            .unwrap_or(index);
        if best_idx != index {
            moves.swap(index, best_idx);
        }
    }

    fn record_cutoff(&self, mv: &ChessMove, _state: &Board, depth: u8) {
        // Only record history for quiet (non-tactical) moves
        // Tactical moves are already prioritized by move ordering
        if !mv.is_tactical(_state) {
            record_history_cutoff(mv.from_square(), mv.to_square(), depth);
        }
    }
}

fn compare_moves(a: &ChessMove, b: &ChessMove, board: &Board) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    // 1. Use MVV-LVA (Most Valuable Victim - Least Valuable Attacker) for captures
    match (is_capture(a), is_capture(b)) {
        (true, true) => {
            // Both are captures - use MVV-LVA ordering
            let score_a = mvv_lva_score(a, board);
            let score_b = mvv_lva_score(b, board);
            // Higher score is better, so reverse comparison for ascending sort
            score_b.cmp(&score_a)
        }
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => compare_move_types(a, b, board),
    }
}

fn is_capture(chess_move: &ChessMove) -> bool {
    chess_move.captures().is_some()
}

/// MVV-LVA (Most Valuable Victim - Least Valuable Attacker) scoring.
/// Higher score = better capture (prefer capturing valuable pieces with cheap pieces).
fn mvv_lva_score(chess_move: &ChessMove, board: &Board) -> i32 {
    let victim_value = chess_move
        .captures()
        .map(|capture| MATERIAL_VALUES[capture.0 as usize] as i32)
        .unwrap_or(0);

    let attacker_value = get_piece_type(chess_move, board)
        .map(|piece| MATERIAL_VALUES[piece as usize] as i32)
        .unwrap_or(0);

    // Victim value dominates (multiply by 10), then subtract attacker value
    // Example: QxP (queen takes pawn) = 100*10 - 900 = 100
    //          PxQ (pawn takes queen) = 900*10 - 100 = 8900 (much better!)
    victim_value * 10 - attacker_value
}

fn compare_move_types(a: &ChessMove, b: &ChessMove, board: &Board) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    match (a, b) {
        (ChessMove::PawnPromotion(_), ChessMove::PawnPromotion(_)) => Ordering::Equal,
        (ChessMove::PawnPromotion(_), _) => Ordering::Less,
        (_, ChessMove::PawnPromotion(_)) => Ordering::Greater,
        _ => {
            // For quiet moves, use history heuristic
            let history_a = get_history_score(a.from_square(), a.to_square());
            let history_b = get_history_score(b.from_square(), b.to_square());
            match history_a.cmp(&history_b) {
                Ordering::Equal => {
                    compare_piece_types(get_piece_type(a, board), get_piece_type(b, board))
                }
                other => other.reverse(), // Higher history score = better, so reverse for ascending sort
            }
        }
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
    use crate::chess_move::capture::Capture;
    use crate::chess_move::castle::CastleChessMove;
    use crate::chess_move::en_passant::EnPassantChessMove;
    use crate::chess_move::pawn_promotion::PawnPromotionChessMove;
    use crate::chess_move::standard::StandardChessMove;
    use crate::move_generator::ChessMoveList;
    use crate::{castle_kingside, chess_position, en_passant_move, promotion, std_move};
    use common::bitboard::*;

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

    fn sort_moves(moves: &mut ChessMoveList, board: &Board) {
        ChessMoveOrderer.order_moves(moves.as_mut(), board);
    }

    #[test]
    fn test_sort_chess_moves() {
        let board = create_test_board();
        let mut moves = ChessMoveList::new();

        moves.push(std_move!(E4, E5));
        moves.push(std_move!(E4, D5, Capture(Piece::Pawn)));
        moves.push(std_move!(D1, D5, Capture(Piece::Pawn)));
        moves.push(std_move!(D1, E8, Capture(Piece::Queen)));
        moves.push(promotion!(E7, E8, None, Piece::Queen));
        moves.push(en_passant_move!(E5, D6));
        moves.push(castle_kingside!(Color::White));
        moves.push(std_move!(A1, A3));
        moves.push(std_move!(F3, G5));
        moves.push(std_move!(C3, E5));
        moves.push(std_move!(E2, E4));

        sort_moves(&mut moves, &board);

        // Captures first, ordered by MVV-LVA
        // QxQ (highest victim) > PxP/QxP (pawn victim, PxP preferred over QxP by attacker)
        assert!(moves[0].captures().is_some()); // QxQueen (best capture)
        assert!(moves[1].captures().is_some());
        assert!(moves[2].captures().is_some());
        assert!(matches!(moves[3], ChessMove::EnPassant(_))); // en passant is a capture
                                                              // Promotions next
        assert!(matches!(moves[4], ChessMove::PawnPromotion(_)));
        // Quiet moves by piece type
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

        moves.push(std_move!(E4, E5));
        moves.push(std_move!(F3, G5));
        moves.push(std_move!(A1, A3));
        moves.push(std_move!(C3, E5));

        sort_moves(&mut moves, &board);

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
    fn test_sort_captures_mvv_lva() {
        let board = create_test_board();
        let mut moves = ChessMoveList::new();

        // F3=Knight, E1=Queen, E4=Pawn; all capturing D5=Pawn
        moves.push(std_move!(F3, D5, Capture(Piece::Pawn)));
        moves.push(std_move!(E1, D5, Capture(Piece::Pawn)));
        moves.push(std_move!(E4, D5, Capture(Piece::Pawn)));
        moves.push(std_move!(A1, A3));

        sort_moves(&mut moves, &board);

        // All captures come first, ordered by MVV-LVA
        // Same victim (pawn), so prefer lower value attacker
        assert!(moves[0].captures().is_some());
        assert!(moves[1].captures().is_some());
        assert!(moves[2].captures().is_some());
        assert_eq!(moves[0].from_square(), E4); // Pawn takes pawn (best)
        assert_eq!(moves[1].from_square(), F3); // Knight takes pawn
        assert_eq!(moves[2].from_square(), E1); // Queen takes pawn (worst)
                                                // Quiet move last
        assert!(moves[3].captures().is_none());
    }

    #[test]
    fn test_is_tactical_captures_and_promotions_only() {
        use crate::alpha_beta_searcher::GameMove;
        let board = Board::default();

        let capture = std_move!(E4, D5, Capture(Piece::Pawn));
        assert!(capture.is_tactical(&board));

        let promotion = promotion!(E7, E8, None, Piece::Queen);
        assert!(promotion.is_tactical(&board));

        let quiet = std_move!(E2, E4);
        assert!(!quiet.is_tactical(&board));

        let castle = castle_kingside!(Color::White);
        assert!(!castle.is_tactical(&board));
    }

    #[test]
    fn test_pick_next_selects_best_first() {
        let board = create_test_board();
        let mut moves = ChessMoveList::new();

        // Add moves in worst-first order: quiet pawn, quiet knight, capture
        moves.push(std_move!(E4, E5));
        moves.push(std_move!(F3, G5));
        moves.push(std_move!(E4, D5, Capture(Piece::Pawn)));

        // Full sort for reference
        let mut sorted = moves.clone();
        ChessMoveOrderer.order_moves(sorted.as_mut(), &board);

        // pick_next at index 0 should place the same move as the first sorted move
        ChessMoveOrderer.pick_next(moves.as_mut(), 0, &board);
        assert_eq!(
            moves[0], sorted[0],
            "pick_next should select the same best move as full sort"
        );
    }

    #[test]
    fn test_pick_next_incremental_equals_full_sort() {
        let board = create_test_board();
        let mut moves = ChessMoveList::new();

        moves.push(std_move!(E4, E5));
        moves.push(std_move!(E4, D5, Capture(Piece::Pawn)));
        moves.push(std_move!(D1, D5, Capture(Piece::Pawn)));
        moves.push(std_move!(A1, A3));
        moves.push(std_move!(F3, G5));
        moves.push(std_move!(C3, E5));

        // Full sort for reference
        let mut sorted = moves.clone();
        ChessMoveOrderer.order_moves(sorted.as_mut(), &board);

        // Incremental selection should produce the same order
        let m = moves.as_mut();
        for i in 0..m.len() {
            ChessMoveOrderer.pick_next(m, i, &board);
        }

        for i in 0..sorted.len() {
            assert_eq!(
                moves[i], sorted[i],
                "Mismatch at index {i}: pick_next gave {:?}, sort gave {:?}",
                moves[i], sorted[i]
            );
        }
    }

    #[test]
    fn test_pick_next_noop_on_last_element() {
        let board = create_test_board();
        let mut moves = ChessMoveList::new();
        moves.push(std_move!(E4, E5));

        let original = moves[0].clone();
        ChessMoveOrderer.pick_next(moves.as_mut(), 0, &board);
        assert_eq!(moves[0], original, "Single element should be unchanged");
    }
}
