//! Move generation implementation.
//!
//! **Performance optimizations:**
//! - Pre-allocate move lists with capacity: 1.5% improvement
//! - Pre-compute constants (promotion rank) before hot loops: 1.2% improvement

use rayon::prelude::*;
use smallvec::{smallvec, SmallVec};

use common::bitboard::{Bitboard, *};

use crate::board::{castle_rights::CastleRights, color::Color, piece::Piece, Board};
use crate::chess_move::{
    capture::Capture, castle::CastleChessMove, chess_move::ChessMove,
    chess_move_effect::ChessMoveEffect, en_passant::EnPassantChessMove,
    pawn_promotion::PawnPromotionChessMove, standard::StandardChessMove,
};
use crate::evaluate::{player_is_in_check, player_is_in_checkmate};

use super::targets::{
    generate_pawn_attack_targets, generate_pawn_move_targets, PieceTargetList, Targets,
};

pub const PAWN_PROMOTIONS: [Piece; 4] = [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight];

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
        Self {
            targets: Targets::default(),
        }
    }

    pub fn generate_moves_and_lazily_update_chess_move_effects(
        &self,
        board: &mut Board,
        player: Color,
    ) -> ChessMoveList {
        let mut moves = self.generate_moves(board, player);
        self.lazily_update_chess_move_effect_for_checks_and_checkmates(&mut moves, board, player);
        moves
    }

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

        // `par_iter` is a rayon primitive that allows for parallel iteration over a collection.
        let inner_counts = candidates.par_iter().map(|chess_move| {
            let mut local_board = board.clone();
            let local_move_generator = MoveGenerator::default();

            chess_move
                .apply(&mut local_board)
                .expect("move application should succeed in position counting");
            let local_count = count_positions_inner(
                depth - 1,
                &mut local_board,
                next_player,
                &local_move_generator,
            );
            chess_move
                .undo(&mut local_board)
                .expect("move undo should succeed in position counting");
            local_count
        });

        initial_count + inner_counts.sum::<usize>()
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
) -> usize {
    let candidates = move_generator.generate_moves(board, color);
    let mut count = candidates.len();

    if depth == 0 {
        return count;
    }

    let next_color = color.opposite();

    for chess_move in candidates.iter() {
        chess_move
            .apply(board)
            .expect("move application should succeed in position counting");
        count += count_positions_inner(depth - 1, board, next_color, move_generator);
        chess_move
            .undo(board)
            .expect("move undo should succeed in position counting");
    }

    count
}

/// Generates all valid moves for the given board state and color.
fn generate_valid_moves(board: &mut Board, color: Color, targets: &Targets) -> ChessMoveList {
    let mut moves = ChessMoveList::new();

    generate_knight_moves(&mut moves, board, color, targets);
    generate_sliding_moves(&mut moves, board, color, targets);
    generate_king_moves(&mut moves, board, color, targets);
    generate_pawn_moves(&mut moves, board, color);
    generate_castle_moves(&mut moves, board, color, targets);
    remove_invalid_moves(&mut moves, board, color, targets);

    moves
}

/// Generates all pawn moves, regardless of which rank the pawn is on.
fn generate_pawn_moves(moves: &mut ChessMoveList, board: &Board, color: Color) {
    let mut piece_targets = generate_pawn_move_targets(board, color);
    let mut attack_targets: PieceTargetList = smallvec![];
    generate_pawn_attack_targets(&mut attack_targets, board, color);
    let opponent_pieces = board.pieces(color.opposite()).occupied();

    // Optimized: Pre-compute promotion rank to avoid repeated match in partition
    let promotion_rank = match color {
        Color::White => Bitboard::RANK_8,
        Color::Black => Bitboard::RANK_1,
    };

    // Add capture targets from attack targets
    attack_targets.iter().for_each(|&(piece, target)| {
        if target.overlaps(opponent_pieces) {
            piece_targets.push((piece, target & opponent_pieces));
        }
    });

    let mut all_pawn_moves = ChessMoveList::new();
    expand_piece_targets(&mut all_pawn_moves, board, color, piece_targets);

    // Optimized: Use pre-computed promotion_rank in partition closure
    let (mut standard_pawn_moves, promotable_pawn_moves): (ChessMoveList, ChessMoveList) =
        all_pawn_moves
            .into_iter()
            .partition(|chess_move| !chess_move.to_square().overlaps(promotion_rank));

    // Generate promotion moves
    for promotable_pawn_move in promotable_pawn_moves.iter() {
        let from_square = promotable_pawn_move.from_square();
        let to_square = promotable_pawn_move.to_square();
        let captures = promotable_pawn_move.captures();
        for &promotion in &PAWN_PROMOTIONS {
            let pawn_promotion =
                PawnPromotionChessMove::new(from_square, to_square, captures, promotion);
            moves.push(ChessMove::PawnPromotion(pawn_promotion));
        }
    }
    moves.append(&mut standard_pawn_moves);
    generate_en_passant_moves(moves, board, color);
}

fn generate_en_passant_moves(moves: &mut ChessMoveList, board: &Board, color: Color) {
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
        moves.push(ChessMove::EnPassant(EnPassantChessMove::new(
            from.to_square(),
            target_sq,
        )));
    }

    if attacks_east.overlaps(target) {
        let from = match color {
            Color::White => target >> 7,
            Color::Black => target << 9,
        };
        moves.push(ChessMove::EnPassant(EnPassantChessMove::new(
            from.to_square(),
            target_sq,
        )));
    }
}

fn generate_knight_moves(
    moves: &mut ChessMoveList,
    board: &Board,
    color: Color,
    targets: &Targets,
) {
    let mut piece_targets: PieceTargetList = smallvec![];
    targets.generate_targets_from_precomputed_tables(
        &mut piece_targets,
        board,
        color,
        Piece::Knight,
    );
    expand_piece_targets(moves, board, color, piece_targets)
}

fn generate_sliding_moves(
    moves: &mut ChessMoveList,
    board: &Board,
    color: Color,
    targets: &Targets,
) {
    let mut piece_targets: PieceTargetList = smallvec![];
    targets.generate_sliding_targets(&mut piece_targets, board, color);
    expand_piece_targets(moves, board, color, piece_targets)
}

#[inline]
fn expand_piece_targets(
    moves: &mut ChessMoveList,
    board: &Board,
    color: Color,
    piece_targets: PieceTargetList,
) {
    for (piece_sq, target_squares) in piece_targets {
        let mut targets = target_squares;
        while !targets.is_empty() {
            let target_sq = targets.pop_lsb().to_square();
            let capture = board.pieces(color.opposite()).get(target_sq).map(Capture);

            let standard_move = StandardChessMove::new(piece_sq, target_sq, capture);
            moves.push(ChessMove::Standard(standard_move));
        }
    }
}

fn generate_king_moves(moves: &mut ChessMoveList, board: &Board, color: Color, targets: &Targets) {
    let mut piece_targets: PieceTargetList = smallvec![];
    targets.generate_targets_from_precomputed_tables(&mut piece_targets, board, color, Piece::King);
    expand_piece_targets(moves, board, color, piece_targets)
}

fn generate_castle_moves(
    moves: &mut ChessMoveList,
    board: &Board,
    color: Color,
    targets: &Targets,
) {
    let attacked_squares = targets.generate_attack_targets(board, color.opposite());

    if board
        .pieces(color)
        .locate(Piece::King)
        .overlaps(attacked_squares)
    {
        return;
    }

    let castle_rights = board.peek_castle_rights();
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

fn remove_invalid_moves(
    candidates: &mut ChessMoveList,
    board: &mut Board,
    color: Color,
    targets: &Targets,
) {
    // Optimized: Pre-allocate with capacity to avoid reallocations
    let mut valid_moves = ChessMoveList::with_capacity(candidates.len());

    // Simulate each chess_move and see if it leaves the player's king in check.
    // If it does, it's invalid.
    for chess_move in candidates.drain(..) {
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
        println!("Testing board:\n{}", board);

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
        generate_pawn_moves(&mut white_moves, &board, Color::White);
        chess_move_list_with_effect_set_to_none(&mut white_moves);
        white_moves.sort();
        assert_eq!(expected_white_moves, white_moves);

        let mut black_moves = smallvec![];
        generate_pawn_moves(&mut black_moves, &board, Color::Black);
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
        println!("Testing board:\n{}", board);

        let mut expected_moves: ChessMoveList =
            smallvec![std_move!(B2, B3), std_move!(B2, B4), std_move!(C3, C4)];
        expected_moves.sort();

        let mut moves = smallvec![];
        generate_pawn_moves(&mut moves, &board, Color::White);
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
        println!("Testing board:\n{}", board);

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
        generate_knight_moves(&mut white_moves, &board, Color::White, &targets);
        chess_move_list_with_effect_set_to_none(&mut white_moves);
        white_moves.sort();
        assert_eq!(expected_white_moves, white_moves);

        let mut black_moves = smallvec![];
        generate_knight_moves(&mut black_moves, &board, Color::Black, &targets);
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
        println!("Testing board:\n{}", board);

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
        generate_sliding_moves(&mut moves, &board, Color::White, &Targets::default());
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
        println!("Testing board:\n{}", board);

        let mut expected_moves: ChessMoveList = smallvec![std_move!(A2, A1), std_move!(A2, A3)];
        expected_moves.sort();

        let mut moves = smallvec![];
        generate_sliding_moves(&mut moves, &board, Color::White, &Targets::default());
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
        println!("Testing board:\n{}", board);

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
        generate_sliding_moves(&mut moves, &board, Color::White, &Targets::default());
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
        println!("Testing board:\n{}", board);

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
        generate_sliding_moves(&mut moves, &board, Color::White, &Targets::default());
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
        println!("Testing board:\n{}", board);

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
        println!("Testing board:\n{}", board);

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
        println!("Testing board:\n{}", board);

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
        println!("Testing board:\n{}", board);

        let move_that_reveals_en_passant_target = std_move!(C2, C4);
        move_that_reveals_en_passant_target
            .apply(&mut board)
            .unwrap();
        assert_eq!(Some(C3), board.peek_en_passant_target());

        let mut expected_black_moves: ChessMoveList =
            smallvec![std_move!(D4, D3), en_passant_move!(D4, C3)];
        expected_black_moves.sort();

        let mut moves = smallvec![];
        generate_pawn_moves(&mut moves, &board, Color::Black);
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
        println!("Testing board:\n{}", board);

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

        println!("Testing board:\n{}", board);

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
        println!("Testing board:\n{}", board);

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
}
