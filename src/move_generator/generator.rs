//! Move generation implementation.
//!
//! **Performance optimizations:**
//! - Pre-allocate move lists with capacity to avoid reallocations
//! - Pre-compute constants (promotion rank) before hot loops
//! - **MoveGenerator sharing**: Pass `&self` to recursive functions instead of creating new
//!   `MoveGenerator` instances (865 KB each). This eliminates redundant allocations and
//!   reduces memory overhead.
//! - **Conditional cloning**: Only clone boards when parallelizing (move lists >= 10). Use
//!   sequential apply/undo pattern for small move lists to avoid cloning overhead.
//! - **Multi-depth parallelization**: Parallelize at depth 2 and above (configurable threshold)
//!   with conditional parallelization based on move count (>= 10 moves). This improves CPU
//!   utilization across all threads and reduces thread synchronization overhead.

use rayon::prelude::*;
use smallvec::{smallvec, SmallVec};
#[cfg(feature = "instrumentation")]
use tracing::instrument;

use common::bitboard::{Bitboard, *};

use crate::board::{castle_rights::CastleRights, color::Color, piece::Piece, Board};
use crate::chess_move::{
    capture::Capture, castle::CastleChessMove, chess_move::ChessMove,
    chess_move_effect::ChessMoveEffect, en_passant::EnPassantChessMove,
    pawn_promotion::PawnPromotionChessMove, standard::StandardChessMove,
};
use crate::evaluate::{player_is_in_check, player_is_in_checkmate};

use super::targets::{
    generate_pawn_attack_targets, generate_pawn_move_targets, PieceTargetList, PinInfo, Targets,
};

pub const PAWN_PROMOTIONS: [Piece; 4] = [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight];

/// Minimum number of moves required to justify parallelization overhead.
///
/// When generating moves for position counting, we only parallelize if the move list
/// contains at least this many moves. For smaller move lists, the overhead of thread
/// creation, board cloning, and synchronization exceeds the benefits of parallelization.
///
/// This threshold is used in both `count_positions` and `count_positions_inner` to
/// determine when to use parallel iteration vs sequential apply/undo pattern.
const PARALLEL_MOVE_THRESHOLD: usize = 10;

/// A list of chess moves that is optimized for small sizes.
pub type ChessMoveList = SmallVec<[ChessMove; 32]>;

/// Generates all possible moves for a given board state.
#[derive(Clone)]
pub struct MoveGenerator {
    targets: Targets,
}

impl Default for MoveGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveGenerator {
    pub fn new() -> Self {
        crate::diagnostics::memory_profiler::MemoryProfiler::record_movegen_create();
        Self {
            targets: Targets::default(),
        }
    }

    #[cfg_attr(feature = "instrumentation", instrument(skip_all))]
    pub fn generate_moves_and_lazily_update_chess_move_effects(
        &self,
        board: &mut Board,
        player: Color,
    ) -> ChessMoveList {
        let mut moves = self.generate_moves(board, player);
        self.lazily_update_chess_move_effect_for_checks_and_checkmates(&mut moves, board, player);
        moves
    }

    #[cfg_attr(feature = "instrumentation", instrument(skip_all))]
    pub fn generate_moves(&self, board: &mut Board, player: Color) -> ChessMoveList {
        generate_valid_moves(board, player, &self.targets)
    }

    fn lazily_update_chess_move_effect_for_checks_and_checkmates(
        &self,
        moves: &mut ChessMoveList,
        board: &mut Board,
        player: Color,
    ) {
        let opponent = player.opposite();
        for chess_move in moves.iter_mut() {
            self.lazily_calculate_chess_move_effect(chess_move, board, opponent);
        }
    }

    fn lazily_calculate_chess_move_effect(
        &self,
        chess_move: &mut ChessMove,
        board: &mut Board,
        player: Color,
    ) -> ChessMoveEffect {
        chess_move
            .apply(board)
            .expect("move application should succeed when checking effects");
        let chess_move_effect = if player_is_in_checkmate(board, self, player) {
            ChessMoveEffect::Checkmate
        } else if player_is_in_check(board, self, player) {
            ChessMoveEffect::Check
        } else {
            ChessMoveEffect::None
        };
        chess_move
            .undo(board)
            .expect("move undo should succeed when checking effects");

        chess_move.set_effect(chess_move_effect);

        chess_move_effect
    }

    pub fn count_positions(&self, depth: u8, board: &mut Board, player: Color) -> usize {
        let candidates = self.generate_moves(board, player);
        let initial_count = candidates.len();

        if depth == 0 {
            return initial_count;
        }

        let next_player = player.opposite();

        // Use parallel iteration for larger move lists, sequential for small ones to avoid cloning overhead
        let inner_count = if candidates.len() >= PARALLEL_MOVE_THRESHOLD {
            // Parallel path: clone board for each task
            let inner_counts = candidates.par_iter().map(|chess_move| {
                let mut local_board = board.clone();

                chess_move
                    .apply(&mut local_board)
                    .expect("move application should succeed in position counting");
                let local_count = count_positions_inner(
                    depth - 1,
                    &mut local_board,
                    next_player,
                    self,
                    2, // parallel_threshold: parallelize at depth 2 and above
                );
                chess_move
                    .undo(&mut local_board)
                    .expect("move undo should succeed in position counting");
                local_count
            });
            inner_counts.sum::<usize>()
        } else {
            // Sequential path: no cloning needed, use apply/undo pattern
            let mut inner_count = 0;
            for chess_move in candidates.iter() {
                chess_move
                    .apply(board)
                    .expect("move application should succeed in position counting");
                inner_count += count_positions_inner(depth - 1, board, next_player, self, 2);
                chess_move
                    .undo(board)
                    .expect("move undo should succeed in position counting");
            }
            inner_count
        };

        initial_count + inner_count
    }

    pub fn get_attack_targets(&self, board: &Board, player: Color) -> Bitboard {
        self.targets.generate_attack_targets(board, player)
    }
}

fn count_positions_inner(
    depth: u8,
    board: &mut Board,
    color: Color,
    move_generator: &MoveGenerator,
    parallel_threshold: u8,
) -> usize {
    let candidates = move_generator.generate_moves(board, color);
    let mut count = candidates.len();

    if depth == 0 {
        return count;
    }

    let next_color = color.opposite();

    // Parallelize if depth is above threshold and we have enough moves to justify overhead
    if depth >= parallel_threshold && candidates.len() >= PARALLEL_MOVE_THRESHOLD {
        let inner_counts = candidates.par_iter().map(|chess_move| {
            let mut local_board = board.clone();
            chess_move
                .apply(&mut local_board)
                .expect("move application should succeed in position counting");
            let local_count = count_positions_inner(
                depth - 1,
                &mut local_board,
                next_color,
                move_generator,
                parallel_threshold,
            );
            chess_move
                .undo(&mut local_board)
                .expect("move undo should succeed in position counting");
            local_count
        });
        count += inner_counts.sum::<usize>()
    } else {
        // Sequential for shallow depths or small move lists
        for chess_move in candidates.iter() {
            chess_move
                .apply(board)
                .expect("move application should succeed in position counting");
            count += count_positions_inner(
                depth - 1,
                board,
                next_color,
                move_generator,
                parallel_threshold,
            );
            chess_move
                .undo(board)
                .expect("move undo should succeed in position counting");
        }
    }

    count
}

/// Generates all valid moves for the given board state and color.
#[cfg_attr(feature = "instrumentation", instrument(skip_all))]
fn generate_valid_moves(board: &mut Board, color: Color, targets: &Targets) -> ChessMoveList {
    // Calculate pin and check information once at the start
    let pin_info = targets.calculate_pins(board, color);
    let check_info = targets.calculate_checks(board, color);

    let mut moves = ChessMoveList::new();

    // Handle double check: only king moves are legal (major optimization!)
    if check_info.in_double_check() {
        generate_king_moves(&mut moves, board, color, targets);
        // Still need to filter king moves to ensure they don't move into check
        remove_invalid_king_moves(&mut moves, board, color, targets);
        return moves;
    }

    // Handle single check: only moves that capture checker or block check ray are legal
    if check_info.in_check() {
        // Legal target squares: capture the checker or block the check ray
        let legal_targets = check_info.checkers | check_info.check_ray;

        // Generate king moves (still need validation - king might move into check)
        generate_king_moves(&mut moves, board, color, targets);

        // Generate non-king moves restricted to legal targets
        generate_knight_moves(&mut moves, board, color, targets, &pin_info);
        generate_sliding_moves(&mut moves, board, color, targets, &pin_info);
        generate_pawn_moves(&mut moves, board, color, &pin_info);

        // Filter non-king moves to only those that address the check
        filter_moves_by_target(&mut moves, board, color, legal_targets);

        // Validate king moves only (non-king moves are guaranteed legal)
        remove_invalid_king_moves(&mut moves, board, color, targets);

        return moves;
    }

    // Not in check: Generate all moves respecting pins
    generate_knight_moves(&mut moves, board, color, targets, &pin_info);
    generate_sliding_moves(&mut moves, board, color, targets, &pin_info);
    generate_king_moves(&mut moves, board, color, targets);
    generate_pawn_moves(&mut moves, board, color, &pin_info);
    generate_castle_moves(&mut moves, board, color, targets);

    // Selective validation: only king moves, castle moves, and en passant need validation
    remove_invalid_moves(&mut moves, board, color, targets, false);

    moves
}

/// Generates all pawn moves, regardless of which rank the pawn is on.
#[cfg_attr(feature = "instrumentation", instrument(skip_all))]
fn generate_pawn_moves(moves: &mut ChessMoveList, board: &Board, color: Color, pin_info: &PinInfo) {
    let mut piece_targets = generate_pawn_move_targets(board, color);
    let mut attack_targets: PieceTargetList = smallvec![];
    generate_pawn_attack_targets(&mut attack_targets, board, color);
    let opponent_pieces = board.pieces(color.opposite()).occupied();

    // Optimized: Pre-compute promotion rank to avoid repeated match in partition
    let promotion_rank = match color {
        Color::White => Bitboard::RANK_8,
        Color::Black => Bitboard::RANK_1,
    };

    // Restrict pinned pawns to their pin rays
    for (square, target_bitboard) in piece_targets.iter_mut() {
        if pin_info.is_pinned(*square) {
            let pin_ray = pin_info.pin_ray(*square);
            *target_bitboard &= pin_ray;
        }
    }

    // Add capture targets from attack targets (also respecting pins)
    attack_targets.iter().for_each(|&(piece_square, target)| {
        if target.overlaps(opponent_pieces) {
            let mut capture_targets = target & opponent_pieces;
            // If pawn is pinned, restrict captures to pin ray
            if pin_info.is_pinned(piece_square) {
                let pin_ray = pin_info.pin_ray(piece_square);
                capture_targets &= pin_ray;
            }
            if !capture_targets.is_empty() {
                piece_targets.push((piece_square, capture_targets));
            }
        }
    });

    // Generate moves directly into output list, checking for promotions inline
    // This avoids creating temporary SmallVecs and eliminates the partition operation
    for (piece_sq, target_squares) in piece_targets {
        let mut targets = target_squares;
        while !targets.is_empty() {
            let target_sq = targets.pop_lsb_as_square();
            let capture = board.pieces(color.opposite()).get(target_sq).map(Capture);

            // Check if this is a promotion move
            if target_sq.overlaps(promotion_rank) {
                // Generate all four promotion variants
                for &promotion in &PAWN_PROMOTIONS {
                    let pawn_promotion =
                        PawnPromotionChessMove::new(piece_sq, target_sq, capture, promotion);
                    moves.push(ChessMove::PawnPromotion(pawn_promotion));
                }
            } else {
                // Standard pawn move
                let standard_move = StandardChessMove::new(piece_sq, target_sq, capture);
                moves.push(ChessMove::Standard(standard_move));
            }
        }
    }
    generate_en_passant_moves(moves, board, color, pin_info);
}

fn generate_en_passant_moves(
    moves: &mut ChessMoveList,
    board: &Board,
    color: Color,
    pin_info: &PinInfo,
) {
    let Some(target_sq) = board.peek_en_passant_target() else {
        return;
    };
    let target = target_sq.to_bitboard();

    let pawns = board.pieces(color).locate(Piece::Pawn);

    let attacks_west = match color {
        Color::White => (pawns << 9) & !Bitboard::A_FILE,
        Color::Black => (pawns >> 7) & !Bitboard::A_FILE,
    };

    let attacks_east = match color {
        Color::White => (pawns << 7) & !Bitboard::H_FILE,
        Color::Black => (pawns >> 9) & !Bitboard::H_FILE,
    };

    if attacks_west.overlaps(target) {
        let from = match color {
            Color::White => target >> 9,
            Color::Black => target << 7,
        };
        let from_sq = from.to_square();

        // Only generate en passant if pawn is not pinned, or if target is on pin ray
        if !pin_info.is_pinned(from_sq) || pin_info.pin_ray(from_sq).overlaps(target) {
            moves.push(ChessMove::EnPassant(EnPassantChessMove::new(
                from_sq, target_sq,
            )));
        }
    }

    if attacks_east.overlaps(target) {
        let from = match color {
            Color::White => target >> 7,
            Color::Black => target << 9,
        };
        let from_sq = from.to_square();

        // Only generate en passant if pawn is not pinned, or if target is on pin ray
        if !pin_info.is_pinned(from_sq) || pin_info.pin_ray(from_sq).overlaps(target) {
            moves.push(ChessMove::EnPassant(EnPassantChessMove::new(
                from_sq, target_sq,
            )));
        }
    }
}

#[cfg_attr(feature = "instrumentation", instrument(skip_all))]
fn generate_knight_moves(
    moves: &mut ChessMoveList,
    board: &Board,
    color: Color,
    targets: &Targets,
    pin_info: &PinInfo,
) {
    let mut piece_targets: PieceTargetList = smallvec![];
    targets.generate_targets_from_precomputed_tables(
        &mut piece_targets,
        board,
        color,
        Piece::Knight,
    );

    // Filter out pinned knights - they cannot move (knight moves can't stay on pin ray)
    piece_targets.retain(|(square, _)| !pin_info.is_pinned(*square));

    expand_piece_targets(moves, board, color, piece_targets)
}

#[cfg_attr(feature = "instrumentation", instrument(skip_all))]
fn generate_sliding_moves(
    moves: &mut ChessMoveList,
    board: &Board,
    color: Color,
    targets: &Targets,
    pin_info: &PinInfo,
) {
    let mut piece_targets: PieceTargetList = smallvec![];
    targets.generate_sliding_targets(&mut piece_targets, board, color);

    // Restrict pinned sliding pieces to only move along their pin ray
    for (square, target_bitboard) in piece_targets.iter_mut() {
        if pin_info.is_pinned(*square) {
            let pin_ray = pin_info.pin_ray(*square);
            *target_bitboard &= pin_ray; // Restrict targets to pin ray only
        }
    }

    expand_piece_targets(moves, board, color, piece_targets)
}

#[inline]
#[cfg_attr(feature = "instrumentation", instrument(skip_all))]
fn expand_piece_targets(
    moves: &mut ChessMoveList,
    board: &Board,
    color: Color,
    piece_targets: PieceTargetList,
) {
    for (piece_sq, target_squares) in piece_targets {
        let mut targets = target_squares;
        while !targets.is_empty() {
            let target_sq = targets.pop_lsb_as_square();
            let capture = board.pieces(color.opposite()).get(target_sq).map(Capture);

            let standard_move = StandardChessMove::new(piece_sq, target_sq, capture);
            moves.push(ChessMove::Standard(standard_move));
        }
    }
}

#[cfg_attr(feature = "instrumentation", instrument(skip_all))]
fn generate_king_moves(moves: &mut ChessMoveList, board: &Board, color: Color, targets: &Targets) {
    let mut piece_targets: PieceTargetList = smallvec![];
    targets.generate_targets_from_precomputed_tables(&mut piece_targets, board, color, Piece::King);
    expand_piece_targets(moves, board, color, piece_targets)
}

#[cfg_attr(feature = "instrumentation", instrument(skip_all))]
fn generate_castle_moves(
    moves: &mut ChessMoveList,
    board: &Board,
    color: Color,
    targets: &Targets,
) {
    // Verify king is on its starting square before checking anything else
    let king = board.pieces(color).locate(Piece::King);
    let expected_king_square = match color {
        Color::White => E1,
        Color::Black => E8,
    };
    if !expected_king_square.overlaps(king) {
        return;
    }

    // Early exit if player has no castle rights - avoids expensive attack target generation
    let castle_rights = board.peek_castle_rights();
    let player_rights = match color {
        Color::White => {
            castle_rights & (CastleRights::white_kingside() | CastleRights::white_queenside())
        }
        Color::Black => {
            castle_rights & (CastleRights::black_kingside() | CastleRights::black_queenside())
        }
    };
    if player_rights.is_empty() {
        return;
    }

    let attacked_squares = targets.generate_attack_targets(board, color.opposite());

    // Reuse the cached king bitboard instead of looking it up again
    if king.overlaps(attacked_squares) {
        return;
    }

    // Reuse the already-fetched castle_rights from above
    let (kingside_rights, queenside_rights) = match color {
        Color::White => (
            CastleRights::white_kingside() & castle_rights,
            CastleRights::white_queenside() & castle_rights,
        ),
        Color::Black => (
            CastleRights::black_kingside() & castle_rights,
            CastleRights::black_queenside() & castle_rights,
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

    if !kingside_rights.is_empty()
        && board.get(kingside_transit_square).is_none()
        && !kingside_transit_square.overlaps(attacked_squares)
        && !kingside_transit_square.overlaps(occupied)
        && !kingside_target_square.overlaps(occupied)
    {
        let castle_move = CastleChessMove::castle_kingside(color);
        moves.push(ChessMove::Castle(castle_move));
    }

    if !queenside_rights.is_empty()
        && board.get(queenside_transit_square).is_none()
        && !queenside_transit_square.overlaps(attacked_squares)
        && !queenside_transit_square.overlaps(occupied)
        && !queenside_rook_transit_square.overlaps(occupied)
        && !queenside_target_square.overlaps(occupied)
    {
        let castle_move = CastleChessMove::castle_queenside(color);
        moves.push(ChessMove::Castle(castle_move));
    }
}

/// Filters moves to only those that land on legal target squares.
/// Used when in check to restrict moves to those that capture the checker or block the check ray.
#[cfg_attr(feature = "instrumentation", instrument(skip_all))]
fn filter_moves_by_target(
    candidates: &mut ChessMoveList,
    board: &Board,
    color: Color,
    legal_targets: Bitboard,
) {
    let king_square = board.pieces(color).locate(Piece::King);

    // Keep only moves that either:
    // 1. Are king moves (handled separately), OR
    // 2. Land on legal target squares (capture checker or block check ray)
    candidates.retain(|chess_move| {
        let is_king_move = chess_move.from_square().overlaps(king_square);
        is_king_move || chess_move.to_square().overlaps(legal_targets)
    });
}

/// Validates only king moves to ensure they don't move into check.
/// Used when in check, where non-king moves are already guaranteed legal by check-aware generation.
#[cfg_attr(feature = "instrumentation", instrument(skip_all))]
fn remove_invalid_king_moves(
    candidates: &mut ChessMoveList,
    board: &mut Board,
    color: Color,
    targets: &Targets,
) {
    let mut valid_moves = ChessMoveList::with_capacity(candidates.len());
    let king_square = board.pieces(color).locate(Piece::King);

    for chess_move in candidates.drain(..) {
        let is_king_move = chess_move.from_square().overlaps(king_square);

        if is_king_move {
            // Validate king moves by applying and checking if king is attacked
            chess_move
                .apply(board)
                .expect("move application should succeed when validating moves");
            let king = board.pieces(color).locate(Piece::King);
            let attacked_squares = targets.generate_attack_targets(board, color.opposite());
            chess_move
                .undo(board)
                .expect("move undo should succeed when validating moves");

            if !king.overlaps(attacked_squares) {
                valid_moves.push(chess_move);
            }
        } else {
            // Non-king moves in check are guaranteed legal by check-aware generation
            valid_moves.push(chess_move);
        }
    }

    candidates.append(&mut valid_moves);
}

#[cfg_attr(feature = "instrumentation", instrument(skip_all))]
fn remove_invalid_moves(
    candidates: &mut ChessMoveList,
    board: &mut Board,
    color: Color,
    targets: &Targets,
    in_check: bool,
) {
    let mut valid_moves = ChessMoveList::with_capacity(candidates.len());

    let king_square = board.pieces(color).locate(Piece::King);

    for chess_move in candidates.drain(..) {
        // Determine if this move needs validation:
        // When IN CHECK: All moves need validation (must address the check)
        // When NOT in check: Only king moves and en passant need validation
        //   - Castle moves are fully validated during generation (square occupation, attacks)
        //   - Non-king moves are guaranteed legal by pin-aware generation
        let is_king_move = chess_move.from_square().overlaps(king_square);
        let is_en_passant = matches!(chess_move, ChessMove::EnPassant(_));
        let needs_validation = in_check || is_king_move || is_en_passant;

        if needs_validation {
            // Try to apply the move - if it fails, the move is invalid (skip it)
            if let Ok(()) = chess_move.apply(board) {
                let king = board.pieces(color).locate(Piece::King);
                let attacked_squares = targets.generate_attack_targets(board, color.opposite());
                chess_move
                    .undo(board)
                    .expect("move undo should succeed when validating moves");

                if !king.overlaps(attacked_squares) {
                    valid_moves.push(chess_move);
                }
            }
            // If apply failed, move is invalid - don't add it to valid_moves
        } else {
            // Pin-aware generation guarantees this move is legal
            valid_moves.push(chess_move);
        }
    }

    candidates.append(&mut valid_moves);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess_move::chess_move::ChessMove;
    use crate::{
        castle_kingside, castle_queenside, chess_position, en_passant_move, promotion, std_move,
    };
    use smallvec::smallvec;

    #[test]
    fn test_generate_pawn_moves() {
        let board = chess_position! {
            ..r.....
            .P.p...p
            ......P.
            p.......
            P.......
            .....r..
            p..P.P..
            ........
        };

        let mut expected_white_moves: ChessMoveList = smallvec![
            std_move!(D2, D3),
            std_move!(D2, D4),
            std_move!(G6, G7),
            std_move!(G6, H7, Capture(Piece::Pawn)),
            promotion!(B7, B8, None, Piece::Queen),
            promotion!(B7, B8, None, Piece::Rook),
            promotion!(B7, B8, None, Piece::Knight),
            promotion!(B7, B8, None, Piece::Bishop),
            promotion!(B7, C8, Some(Capture(Piece::Rook)), Piece::Queen),
            promotion!(B7, C8, Some(Capture(Piece::Rook)), Piece::Rook),
            promotion!(B7, C8, Some(Capture(Piece::Rook)), Piece::Knight),
            promotion!(B7, C8, Some(Capture(Piece::Rook)), Piece::Bishop),
        ];
        expected_white_moves.sort();

        let mut expected_black_moves: ChessMoveList = smallvec![
            std_move!(D7, D6),
            std_move!(D7, D5),
            std_move!(H7, H6),
            std_move!(H7, H5),
            std_move!(H7, G6, Capture(Piece::Pawn)),
            promotion!(A2, A1, None, Piece::Queen),
            promotion!(A2, A1, None, Piece::Rook),
            promotion!(A2, A1, None, Piece::Knight),
            promotion!(A2, A1, None, Piece::Bishop),
        ];
        expected_black_moves.sort();

        let mut white_moves = smallvec![];
        generate_pawn_moves(&mut white_moves, &board, Color::White, &PinInfo::empty());
        chess_move_list_with_effect_set_to_none(&mut white_moves);
        white_moves.sort();
        assert_eq!(expected_white_moves, white_moves);

        let mut black_moves = smallvec![];
        generate_pawn_moves(&mut black_moves, &board, Color::Black, &PinInfo::empty());
        chess_move_list_with_effect_set_to_none(&mut black_moves);
        black_moves.sort();
        assert_eq!(expected_black_moves, black_moves);
    }

    #[test]
    fn test_generate_pawn_double_moves() {
        let board = chess_position! {
            ........
            ........
            ........
            ........
            ........
            ..P.p...
            .PP.P...
            ........
        };

        let mut expected_moves: ChessMoveList =
            smallvec![std_move!(B2, B3), std_move!(B2, B4), std_move!(C3, C4)];
        expected_moves.sort();

        let mut moves = smallvec![];
        generate_pawn_moves(&mut moves, &board, Color::White, &PinInfo::empty());
        chess_move_list_with_effect_set_to_none(&mut moves);
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_knight_moves() {
        let targets = Targets::default();
        let board = chess_position! {
            ........
            ........
            .......n
            ...p....
            ....P...
            ..N.....
            ........
            ........
        };

        let mut expected_white_moves: ChessMoveList = smallvec![
            std_move!(C3, D5, Capture(Piece::Pawn)),
            std_move!(C3, E2),
            std_move!(C3, D1),
            std_move!(C3, B5),
            std_move!(C3, A4),
            std_move!(C3, A2),
            std_move!(C3, B1),
        ];
        expected_white_moves.sort();

        let mut expected_black_moves: ChessMoveList = smallvec![
            std_move!(H6, G8),
            std_move!(H6, F7),
            std_move!(H6, F5),
            std_move!(H6, G4),
        ];
        expected_black_moves.sort();

        let mut white_moves = smallvec![];
        generate_knight_moves(
            &mut white_moves,
            &board,
            Color::White,
            &targets,
            &PinInfo::empty(),
        );
        chess_move_list_with_effect_set_to_none(&mut white_moves);
        white_moves.sort();
        assert_eq!(expected_white_moves, white_moves);

        let mut black_moves = smallvec![];
        generate_knight_moves(
            &mut black_moves,
            &board,
            Color::Black,
            &targets,
            &PinInfo::empty(),
        );
        chess_move_list_with_effect_set_to_none(&mut black_moves);
        black_moves.sort();
        assert_eq!(expected_black_moves, black_moves);
    }

    #[test]
    fn test_generate_rook_moves_1() {
        let board = chess_position! {
            ........
            ..P.....
            ........
            ........
            ........
            P.R....p
            ........
            ..K.....
        };

        let mut expected_moves: ChessMoveList = smallvec![
            std_move!(C3, C2),
            std_move!(C3, C4),
            std_move!(C3, C5),
            std_move!(C3, C6),
            std_move!(C3, B3),
            std_move!(C3, D3),
            std_move!(C3, E3),
            std_move!(C3, F3),
            std_move!(C3, G3),
            std_move!(C3, H3, Capture(Piece::Pawn)),
        ];
        expected_moves.sort();

        let mut moves = smallvec![];
        generate_sliding_moves(
            &mut moves,
            &board,
            Color::White,
            &Targets::default(),
            &PinInfo::empty(),
        );
        chess_move_list_with_effect_set_to_none(&mut moves);
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_rook_moves_2() {
        let board = chess_position! {
            ........
            ........
            ........
            ........
            P.......
            ........
            RP......
            ........
        };

        let mut expected_moves: ChessMoveList = smallvec![std_move!(A2, A1), std_move!(A2, A3)];
        expected_moves.sort();

        let mut moves = smallvec![];
        generate_sliding_moves(
            &mut moves,
            &board,
            Color::White,
            &Targets::default(),
            &PinInfo::empty(),
        );
        chess_move_list_with_effect_set_to_none(&mut moves);
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_bishop_moves() {
        let board = chess_position! {
            ........
            ..P...p.
            ........
            ....B...
            ........
            ..P.....
            ........
            P.......
        };

        let mut expected_moves: ChessMoveList = smallvec![
            std_move!(E5, D4),
            std_move!(E5, D6),
            std_move!(E5, F4),
            std_move!(E5, F6),
            std_move!(E5, G3),
            std_move!(E5, G7, Capture(Piece::Pawn)),
            std_move!(E5, H2),
        ];
        expected_moves.sort();

        let mut moves = smallvec![];
        generate_sliding_moves(
            &mut moves,
            &board,
            Color::White,
            &Targets::default(),
            &PinInfo::empty(),
        );
        chess_move_list_with_effect_set_to_none(&mut moves);
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_queen_moves() {
        let board = chess_position! {
            .......p
            ....p...
            ....P...
            .P..Q...
            ........
            ......p.
            .P.....P
            ........
        };

        let mut expected_moves: ChessMoveList = smallvec![
            // North - no moves
            // NorthEast
            std_move!(E5, F6),
            std_move!(E5, G7),
            std_move!(E5, H8, Capture(Piece::Pawn)),
            // East
            std_move!(E5, F5),
            std_move!(E5, G5),
            std_move!(E5, H5),
            // SouthEast
            std_move!(E5, F4),
            std_move!(E5, G3, Capture(Piece::Pawn)),
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

        let mut moves = smallvec![];
        generate_sliding_moves(
            &mut moves,
            &board,
            Color::White,
            &Targets::default(),
            &PinInfo::empty(),
        );
        chess_move_list_with_effect_set_to_none(&mut moves);
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_king_moves_corner() {
        let board = chess_position! {
            ........
            ........
            ........
            ........
            ........
            ........
            ........
            K.......
        };

        let mut expected_moves: ChessMoveList =
            smallvec![std_move!(A1, A2), std_move!(A1, B1), std_move!(A1, B2)];
        expected_moves.sort();

        let mut moves = smallvec![];
        generate_king_moves(&mut moves, &board, Color::White, &Targets::default());
        chess_move_list_with_effect_set_to_none(&mut moves);
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_king_moves_edge_south() {
        let board = chess_position! {
            ........
            ........
            ........
            ........
            ........
            ........
            ...p....
            ....K...
        };

        let mut expected_moves: ChessMoveList = smallvec![
            std_move!(E1, D1),
            std_move!(E1, D2, Capture(Piece::Pawn)),
            std_move!(E1, E2),
            std_move!(E1, F1),
            std_move!(E1, F2),
        ];
        expected_moves.sort();

        let mut moves = smallvec![];
        generate_king_moves(&mut moves, &board, Color::White, &Targets::default());
        chess_move_list_with_effect_set_to_none(&mut moves);
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_king_moves_middle() {
        let board = chess_position! {
            ........
            ........
            ....P...
            ....K...
            ....p...
            ........
            ........
            ........
        };

        let mut expected_moves: ChessMoveList = smallvec![
            std_move!(E5, D4),
            std_move!(E5, D5),
            std_move!(E5, D6),
            std_move!(E5, E4, Capture(Piece::Pawn)),
            std_move!(E5, F4),
            std_move!(E5, F5),
            std_move!(E5, F6),
        ];
        expected_moves.sort();

        let mut moves = smallvec![];
        generate_king_moves(&mut moves, &board, Color::White, &Targets::default());
        chess_move_list_with_effect_set_to_none(&mut moves);
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_en_passant_moves() {
        let mut board = chess_position! {
            ........
            ........
            ........
            ........
            ...p....
            ........
            ..P.....
            ........
        };

        let move_that_reveals_en_passant_target = std_move!(C2, C4);
        move_that_reveals_en_passant_target
            .apply(&mut board)
            .unwrap();
        assert_eq!(Some(C3), board.peek_en_passant_target());

        let mut expected_black_moves: ChessMoveList =
            smallvec![std_move!(D4, D3), en_passant_move!(D4, C3)];
        expected_black_moves.sort();

        let mut moves = smallvec![];
        generate_pawn_moves(&mut moves, &board, Color::Black, &PinInfo::empty());
        chess_move_list_with_effect_set_to_none(&mut moves);
        moves.sort();

        assert_eq!(expected_black_moves, moves);
    }

    #[test]
    fn test_generate_castle_moves_with_all_rights() {
        let board = chess_position! {
            r...k..r
            ........
            ........
            ........
            ........
            ........
            ........
            R...K..R
        };

        let mut expected_white_moves: ChessMoveList = smallvec![
            castle_kingside!(Color::White),
            castle_queenside!(Color::White),
        ];
        expected_white_moves.sort();

        let mut expected_black_moves: ChessMoveList = smallvec![
            castle_kingside!(Color::Black),
            castle_queenside!(Color::Black),
        ];
        expected_black_moves.sort();

        let mut targets = Targets::default();

        let mut white_moves = smallvec![];
        generate_castle_moves(&mut white_moves, &board, Color::White, &mut targets);
        chess_move_list_with_effect_set_to_none(&mut white_moves);
        white_moves.sort();

        let mut black_moves = smallvec![];
        generate_castle_moves(&mut black_moves, &board, Color::Black, &mut targets);
        chess_move_list_with_effect_set_to_none(&mut black_moves);
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
        let board = chess_position! {
            r...k..r
            ...r....
            ........
            ........
            ........
            B.......
            ........
            R...K..R
        };

        let expected_white_moves: ChessMoveList = smallvec![castle_kingside!(Color::White)];
        let expected_black_moves: ChessMoveList = smallvec![castle_queenside!(Color::Black)];

        let mut targets = Targets::default();
        targets.generate_attack_targets(&board, Color::Black);

        let mut white_moves = smallvec![];
        generate_castle_moves(&mut white_moves, &board, Color::White, &mut targets);
        chess_move_list_with_effect_set_to_none(&mut white_moves);

        let mut black_moves = smallvec![];
        generate_castle_moves(&mut black_moves, &board, Color::Black, &mut targets);
        chess_move_list_with_effect_set_to_none(&mut black_moves);

        assert_eq!(expected_white_moves, white_moves);
        assert_eq!(expected_black_moves, black_moves);
    }

    #[test]
    pub fn test_generate_castle_moves_blocked() {
        let board = chess_position! {
            ........
            ........
            ........
            ........
            ........
            ........
            ........
            RB..K.nR
        };

        let expected_white_moves: ChessMoveList = smallvec![];
        let mut white_moves = smallvec![];
        generate_castle_moves(&mut white_moves, &board, Color::White, &Targets::default());
        chess_move_list_with_effect_set_to_none(&mut white_moves);

        assert_eq!(expected_white_moves, white_moves);
    }

    /// The lower level `move_generator` functions generate chess moves before their
    /// effect (check, checkmate, etc.) is calculated. This helper sets the effect
    /// to `None` for all chess moves in a list to simplify testing.
    fn chess_move_list_with_effect_set_to_none(chess_move_list: &mut ChessMoveList) {
        for chess_move in chess_move_list.iter_mut() {
            chess_move.set_effect(ChessMoveEffect::None);
        }
    }

    #[test]
    fn test_double_check_only_king_moves() {
        let mut board = chess_position! {
            ........
            ........
            ........
            ........
            r...K...
            ......n.
            ........
            ........
        };

        let move_gen = MoveGenerator::default();
        let moves = move_gen.generate_moves(&mut board, Color::White);

        // In double check, only king moves are legal
        // King can move to D3, D4, D5, E3, E5, F3, F4, F5 (if not attacked)
        // All moves should be king moves
        for chess_move in moves.iter() {
            let from = chess_move.from_square();
            assert!(
                from == E4,
                "In double check, only king moves should be generated, found move from {:?}",
                from
            );
        }

        // Should have some king moves (at least one escape square)
        assert!(
            !moves.is_empty(),
            "King should have at least one legal move in double check"
        );
    }

    #[test]
    fn test_no_castling_when_in_check() {
        let mut board = chess_position! {
            ........
            ........
            ........
            ........
            r...K...
            ........
            ........
            R......R
        };

        let move_gen = MoveGenerator::default();
        let moves = move_gen.generate_moves(&mut board, Color::White);

        // King is in check from rook at A4, so castling should not be possible
        for chess_move in moves.iter() {
            if let ChessMove::Castle(_) = chess_move {
                panic!("Castling should not be generated when in check");
            }
        }
    }

    #[test]
    fn test_pin_detection_integration() {
        let mut board = chess_position! {
            ........
            ........
            ........
            ........
            r.N.K...
            ........
            ........
            R......R
        };

        // Clear castle rights to avoid castle move generation issues
        board.lose_castle_rights(CastleRights::all());

        let move_gen = MoveGenerator::default();
        let moves = move_gen.generate_moves(&mut board, Color::White);

        // Knight at C4 is pinned by rook at A4 and should not be able to move
        for chess_move in moves.iter() {
            let from = chess_move.from_square();
            assert_ne!(from, C4, "Pinned knight at C4 should not be able to move");
        }
    }
}
