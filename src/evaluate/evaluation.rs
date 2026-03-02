//! Position evaluation functions and game state checking.

use common::bitboard::bitboard::Bitboard;

use crate::board::piece::ALL_PIECES;
use crate::board::{color::Color, piece::Piece, Board};
use crate::move_generator::MoveGenerator;

use super::evaluation_tables::{
    ADJACENT_FILES, BISHOP_PAIR_BONUS_EG, BISHOP_PAIR_BONUS_MG, BONUS_TABLES_EG, BONUS_TABLES_MG,
    DOUBLED_PAWN_PENALTY, FILE_MASKS, ISOLATED_PAWN_PENALTY, KING_OPEN_FILE_PENALTY,
    MATERIAL_VALUES, MAX_PHASE, PASSED_PAWN_BONUS_EG, PASSED_PAWN_BONUS_MG, PAWN_SHIELD_BONUS,
    PHASE_WEIGHTS, ROOK_ON_SEVENTH_BONUS_EG, ROOK_ON_SEVENTH_BONUS_MG, ROOK_OPEN_FILE_BONUS_EG,
    ROOK_OPEN_FILE_BONUS_MG, ROOK_SEMI_OPEN_FILE_BONUS_EG, ROOK_SEMI_OPEN_FILE_BONUS_MG,
    SQUARE_TO_BLACK_BONUS_INDEX, SQUARE_TO_WHITE_BONUS_INDEX,
};

const BLACK_WINS: i16 = i16::MIN / 2;
const WHITE_WINS: i16 = i16::MAX / 2;

#[derive(Debug)]
pub enum GameEnding {
    Checkmate,
    Stalemate,
    Draw,
}

#[inline(always)]
pub fn current_player_is_in_check(board: &Board, move_generator: &MoveGenerator) -> bool {
    let current_player = board.turn();
    player_is_in_check(board, move_generator, current_player)
}

#[inline(always)]
pub fn player_is_in_check(board: &Board, move_generator: &MoveGenerator, player: Color) -> bool {
    let king = board.pieces(player).locate(Piece::King);
    let attacked_squares = move_generator.get_attack_targets(board, player.opposite());

    king.overlaps(attacked_squares)
}

#[inline(always)]
pub fn player_is_in_checkmate(
    board: &mut Board,
    move_generator: &MoveGenerator,
    player: Color,
) -> bool {
    let candidates = move_generator.generate_moves(board, player);
    let check = player_is_in_check(board, move_generator, player);
    check && candidates.is_empty()
}

/// Returns the game ending state if the game has ended, otherwise returns None.
/// `position_history` is used for threefold repetition detection when called from
/// the game loop (pass `&[]` during search where game history is unavailable).
#[inline(always)]
pub fn game_ending(
    board: &mut Board,
    move_generator: &MoveGenerator,
    current_turn: Color,
    position_history: &[u64],
) -> Option<GameEnding> {
    if board.halfmove_clock().value() >= 100 {
        return Some(GameEnding::Draw);
    }

    // Threefold repetition: if the current position has appeared 3+ times in game history
    if !position_history.is_empty() {
        let current_hash = board.current_position_hash();
        let count = position_history
            .iter()
            .filter(|&&h| h == current_hash)
            .count();
        if count >= 3 {
            return Some(GameEnding::Draw);
        }
    }

    let candidates = move_generator.generate_moves(board, current_turn);
    let check = current_player_is_in_check(board, move_generator);

    if candidates.is_empty() {
        if check {
            return Some(GameEnding::Checkmate);
        } else {
            return Some(GameEnding::Stalemate);
        }
    }

    None
}

/// Returns the score of the board from White's perspective (positive = White advantage).
#[inline(always)]
pub fn score(
    board: &mut Board,
    move_generator: &MoveGenerator,
    current_turn: Color,
    remaining_depth: u8,
) -> i16 {
    // Near the 50-move rule (100 half-moves), score as draw to avoid surprises
    if board.halfmove_clock().value() >= 90 {
        return 0;
    }

    // OPTIMIZATION: At leaf nodes (depth 0), only check for game ending if we're in check.
    // Most positions are not terminal, so generating all moves to check for checkmate/stalemate
    // is wasteful. We only need full game_ending() check when we're in check (potential checkmate).
    if remaining_depth == 0 {
        let in_check = current_player_is_in_check(board, move_generator);
        if !in_check {
            // Not in check at leaf - just return material evaluation
            return board_material_score(board);
        }
        // In check - need to verify if it's checkmate
        let has_legal_moves = !move_generator
            .generate_moves(board, current_turn)
            .is_empty();
        if !has_legal_moves {
            // Checkmate!
            return if current_turn == Color::White {
                BLACK_WINS - remaining_depth as i16
            } else {
                WHITE_WINS + remaining_depth as i16
            };
        }
        // In check but has legal moves - return material score
        return board_material_score(board);
    }

    // For non-leaf nodes, do full game ending check (no game history during search)
    match game_ending(board, move_generator, current_turn, &[]) {
        Some(GameEnding::Checkmate) => {
            if current_turn == Color::White {
                // Black wins, but sooner is better for Black
                BLACK_WINS - remaining_depth as i16
            } else {
                // White wins, but sooner is better for White
                WHITE_WINS + remaining_depth as i16
            }
        }
        Some(GameEnding::Stalemate) | Some(GameEnding::Draw) => 0,
        _ => board_material_score(board),
    }
}

/// Computes the game phase (0 = pure endgame, 24 = opening/full pieces).
/// Sums phase weights for all non-pawn, non-king pieces on both sides.
#[inline(always)]
fn game_phase(board: &Board) -> u8 {
    let mut phase: u8 = 0;
    for &color in &[Color::White, Color::Black] {
        let pieces = board.pieces(color);
        for &piece in &ALL_PIECES {
            let count = pieces.locate(piece).count_ones() as u8;
            phase += count * PHASE_WEIGHTS[piece as usize];
        }
    }
    // Clamp to MAX_PHASE in case of promotions creating extra pieces
    if phase > MAX_PHASE {
        MAX_PHASE
    } else {
        phase
    }
}

/// Linearly interpolates between midgame and endgame scores based on game phase.
#[inline(always)]
fn taper(mg: i16, eg: i16, phase: u8) -> i16 {
    ((mg as i32 * phase as i32 + eg as i32 * (MAX_PHASE - phase) as i32) / MAX_PHASE as i32) as i16
}

#[inline]
pub fn board_material_score(board: &Board) -> i16 {
    // Compute phase once
    let phase = game_phase(board);

    // Precompute shared bitboards used by positional eval
    let white_pawns = board.pieces(Color::White).locate(Piece::Pawn);
    let black_pawns = board.pieces(Color::Black).locate(Piece::Pawn);
    let all_pawns = white_pawns | black_pawns;

    // Material + PST
    let white_material = player_material_score(board, Color::White, phase);
    let black_material = player_material_score(board, Color::Black, phase);
    let material = white_material - black_material;

    // Positional terms using precomputed bitboards
    let pawn_score = pawn_structure_eval(white_pawns, black_pawns, phase);
    let activity = piece_activity_eval(board, all_pawns, white_pawns, black_pawns, phase);
    let king_safety = king_safety_eval(board, all_pawns, white_pawns, black_pawns, phase);

    material + pawn_score + activity + king_safety
}

/// Returns the material score of the board for the given player.
#[inline]
fn player_material_score(board: &Board, color: Color, phase: u8) -> i16 {
    let mut material: i16 = 0;
    let pieces = board.pieces(color);

    let index_lookup = match color {
        Color::White => &SQUARE_TO_WHITE_BONUS_INDEX,
        Color::Black => &SQUARE_TO_BLACK_BONUS_INDEX,
    };

    for &piece in &ALL_PIECES {
        let mut squares = pieces.locate(piece);
        let piece_value = MATERIAL_VALUES[piece as usize];
        let piece_idx = piece as usize;

        while !squares.is_empty() {
            let sq_index = squares.pop_lsb_as_square().index() as usize;
            let table_index = index_lookup[sq_index];

            let mg_bonus = BONUS_TABLES_MG[piece_idx][table_index];
            let eg_bonus = BONUS_TABLES_EG[piece_idx][table_index];
            let bonus = taper(mg_bonus, eg_bonus, phase);

            material += piece_value + bonus;
        }
    }

    material
}

/// Evaluates pawn structure: passed pawns, doubled pawns, isolated pawns.
/// Returns score from White's perspective.
#[inline]
fn pawn_structure_eval(white_pawns: Bitboard, black_pawns: Bitboard, phase: u8) -> i16 {
    let white_score = pawn_structure_for_color(white_pawns, black_pawns, phase, true);
    let black_score = pawn_structure_for_color(black_pawns, white_pawns, phase, false);
    white_score - black_score
}

/// Evaluates pawn structure for one side.
/// `is_white` determines rank perspective for passed pawn bonuses.
#[inline]
fn pawn_structure_for_color(
    friendly_pawns: Bitboard,
    enemy_pawns: Bitboard,
    phase: u8,
    is_white: bool,
) -> i16 {
    let mut score: i16 = 0;

    // --- Passed pawns ---
    let mut pawns = friendly_pawns;
    while !pawns.is_empty() {
        let sq = pawns.pop_lsb_as_square().index();
        let file = (sq % 8) as usize;
        let rank = (sq / 8) as usize; // 0-based, rank 0 = rank 1

        // A pawn is passed if there are no enemy pawns on the same or adjacent files
        // ahead of it. "Ahead" means higher ranks for White, lower for Black.
        let block_mask = FILE_MASKS[file] | ADJACENT_FILES[file];
        let ahead_mask = if is_white {
            // White pawns advance toward rank 8: clear all ranks <= current rank
            // Shift mask to only include squares above current rank
            block_mask & above_rank_mask(rank)
        } else {
            block_mask & below_rank_mask(rank)
        };

        if (enemy_pawns & ahead_mask).is_empty() {
            // Passed pawn -- bonus by rank from that side's perspective
            let bonus_rank = if is_white { rank } else { 7 - rank };
            let mg = PASSED_PAWN_BONUS_MG[bonus_rank];
            let eg = PASSED_PAWN_BONUS_EG[bonus_rank];
            score += taper(mg, eg, phase);
        }
    }

    // --- Doubled pawns ---
    for (file_idx, &file_mask) in FILE_MASKS.iter().enumerate() {
        let count = (friendly_pawns & file_mask).count_ones();
        if count > 1 {
            score -= (count - 1) as i16 * DOUBLED_PAWN_PENALTY;
        }

        // --- Isolated pawns ---
        if !(friendly_pawns & file_mask).is_empty()
            && (friendly_pawns & ADJACENT_FILES[file_idx]).is_empty()
        {
            score -= ISOLATED_PAWN_PENALTY;
        }
    }

    score
}

/// Returns a mask of all squares strictly above the given rank (0-indexed).
#[inline(always)]
fn above_rank_mask(rank: usize) -> Bitboard {
    // All ranks above `rank`: shift ALL left by (rank+1)*8
    if rank >= 7 {
        Bitboard::EMPTY
    } else {
        Bitboard(Bitboard::ALL.0 << ((rank + 1) * 8))
    }
}

/// Returns a mask of all squares strictly below the given rank (0-indexed).
#[inline(always)]
fn below_rank_mask(rank: usize) -> Bitboard {
    if rank == 0 {
        Bitboard::EMPTY
    } else {
        Bitboard(Bitboard::ALL.0 >> ((8 - rank) * 8))
    }
}

/// Evaluates piece activity: bishop pair, rook on open/semi-open file, rook on 7th.
/// Returns score from White's perspective.
#[inline]
fn piece_activity_eval(
    board: &Board,
    all_pawns: Bitboard,
    white_pawns: Bitboard,
    black_pawns: Bitboard,
    phase: u8,
) -> i16 {
    let white_score = piece_activity_for_color(board, Color::White, all_pawns, white_pawns, phase);
    let black_score = piece_activity_for_color(board, Color::Black, all_pawns, black_pawns, phase);
    white_score - black_score
}

#[inline]
fn piece_activity_for_color(
    board: &Board,
    color: Color,
    all_pawns: Bitboard,
    friendly_pawns: Bitboard,
    phase: u8,
) -> i16 {
    let mut bonus: i16 = 0;
    let pieces = board.pieces(color);

    // Bishop pair
    if pieces.locate(Piece::Bishop).count_ones() >= 2 {
        bonus += taper(BISHOP_PAIR_BONUS_MG, BISHOP_PAIR_BONUS_EG, phase);
    }

    // Rook bonuses
    let seventh_rank = match color {
        Color::White => Bitboard::RANK_7,
        Color::Black => Bitboard::RANK_2,
    };

    let mut rooks = pieces.locate(Piece::Rook);
    while !rooks.is_empty() {
        let sq = rooks.pop_lsb_as_square().index();
        let file = (sq % 8) as usize;
        let rook_file = FILE_MASKS[file];

        // Rook on open file (no pawns at all)
        if (all_pawns & rook_file).is_empty() {
            bonus += taper(ROOK_OPEN_FILE_BONUS_MG, ROOK_OPEN_FILE_BONUS_EG, phase);
        } else if (friendly_pawns & rook_file).is_empty() {
            // Semi-open file (no friendly pawns)
            bonus += taper(
                ROOK_SEMI_OPEN_FILE_BONUS_MG,
                ROOK_SEMI_OPEN_FILE_BONUS_EG,
                phase,
            );
        }

        // Rook on 7th rank
        let rook_bb = Bitboard(1u64 << sq);
        if rook_bb.overlaps(seventh_rank) {
            bonus += taper(ROOK_ON_SEVENTH_BONUS_MG, ROOK_ON_SEVENTH_BONUS_EG, phase);
        }
    }

    bonus
}

/// Evaluates king safety: pawn shield and open files near king.
/// Scaled by game phase (matters only in middlegame).
/// Returns score from White's perspective.
#[inline]
fn king_safety_eval(
    board: &Board,
    all_pawns: Bitboard,
    white_pawns: Bitboard,
    black_pawns: Bitboard,
    phase: u8,
) -> i16 {
    let white_score = king_safety_for_color(board, Color::White, all_pawns, white_pawns, phase);
    let black_score = king_safety_for_color(board, Color::Black, all_pawns, black_pawns, phase);
    white_score - black_score
}

#[inline]
fn king_safety_for_color(
    board: &Board,
    color: Color,
    all_pawns: Bitboard,
    friendly_pawns: Bitboard,
    phase: u8,
) -> i16 {
    let mut score: i16 = 0;
    let pieces = board.pieces(color);

    let king_sq = pieces.locate(Piece::King).to_square().index();
    let king_file = (king_sq % 8) as usize;
    let king_rank = (king_sq / 8) as usize;

    // Pawn shield: count friendly pawns in the 2 ranks immediately ahead on king-adjacent files
    let king_files = FILE_MASKS[king_file] | ADJACENT_FILES[king_file];
    let two_ranks_ahead = match color {
        Color::White => {
            let r = king_rank + 1;
            if r <= 7 {
                let mut m = 0xFFu64 << (r * 8);
                if r <= 6 {
                    m |= 0xFFu64 << ((r + 1) * 8);
                }
                Bitboard(m)
            } else {
                Bitboard::EMPTY
            }
        }
        Color::Black => {
            if king_rank >= 1 {
                let mut m = 0xFFu64 << ((king_rank - 1) * 8);
                if king_rank >= 2 {
                    m |= 0xFFu64 << ((king_rank - 2) * 8);
                }
                Bitboard(m)
            } else {
                Bitboard::EMPTY
            }
        }
    };
    let limited_shield = king_files & two_ranks_ahead;

    let shield_pawns = (friendly_pawns & limited_shield).count_ones() as i16;
    score += taper(shield_pawns * PAWN_SHIELD_BONUS, 0, phase);

    // Open files near king penalty (check only 2-3 adjacent files)
    let start_file = king_file.saturating_sub(1);
    let end_file = if king_file < 7 { king_file + 1 } else { 7 };
    for file_mask in &FILE_MASKS[start_file..=end_file] {
        if (all_pawns & *file_mask).is_empty() {
            score -= taper(KING_OPEN_FILE_PENALTY, 0, phase);
        }
    }

    score
}

/// Determines if the position is an endgame.
#[inline]
pub fn is_endgame(board: &Board) -> bool {
    let white_queen = board.pieces(Color::White).locate(Piece::Queen);
    let black_queen = board.pieces(Color::Black).locate(Piece::Queen);
    let white_king = board.pieces(Color::White).locate(Piece::King);
    let black_king = board.pieces(Color::Black).locate(Piece::King);

    let white_non_king_queen = board.pieces(Color::White).occupied() & !white_queen & !white_king;
    let black_non_king_queen = board.pieces(Color::Black).occupied() & !black_queen & !black_king;

    let both_sides_have_no_queens = white_queen.is_empty() && black_queen.is_empty();
    let white_has_no_queen_or_one_minor_piece =
        white_queen.is_empty() && white_non_king_queen.count_ones() <= 1;
    let black_has_no_queen_or_one_minor_piece =
        black_queen.is_empty() && black_non_king_queen.count_ones() <= 1;

    both_sides_have_no_queens
        || (white_has_no_queen_or_one_minor_piece && black_has_no_queen_or_one_minor_piece)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        board::{castle_rights::CastleRights, Board},
        chess_position,
    };
    use common::bitboard::*;

    #[test]
    fn test_game_phase_starting_position() {
        let board = Board::default();
        // 2 knights(1) + 2 bishops(1) + 2 rooks(2) + 1 queen(4) = 12 per side
        assert_eq!(game_phase(&board), 24);
    }

    #[test]
    fn test_game_phase_endgame() {
        let board = chess_position! {
            ........
            ........
            ...k....
            ........
            ...K....
            ........
            ........
            ........
        };
        assert_eq!(game_phase(&board), 0);
    }

    #[test]
    fn test_taper_midgame() {
        // At full phase (24), should return mg value
        assert_eq!(taper(100, 0, 24), 100);
    }

    #[test]
    fn test_taper_endgame() {
        // At phase 0, should return eg value
        assert_eq!(taper(100, 50, 0), 50);
    }

    #[test]
    fn test_taper_midpoint() {
        // At phase 12 (halfway), should be average
        assert_eq!(taper(100, 0, 12), 50);
    }

    #[test]
    fn test_starting_board_material_score_is_zero() {
        let board = Board::default();
        // Starting position should be equal
        assert_eq!(board_material_score(&board), 0);
    }

    #[test]
    fn test_game_ending_stalemate() {
        // Black king on A8, White queen on B6 -- Black has no legal moves, not in check
        let mut board = chess_position! {
            k.......
            ........
            .Q......
            ........
            ........
            ........
            ........
            ....K...
        };
        board.set_turn(Color::Black);
        board.lose_castle_rights(CastleRights::all());

        let ending = game_ending(&mut board, &MoveGenerator::default(), Color::Black, &[]);
        assert!(matches!(ending, Some(GameEnding::Stalemate)));
    }

    #[test]
    fn test_game_ending_checkmate() {
        // Black king on H8, White queen on G7 delivers check, White king on F7 supports
        let mut board = chess_position! {
            .......k
            .....KQ.
            ........
            ........
            ........
            ........
            ........
            ........
        };
        board.set_turn(Color::Black);
        board.lose_castle_rights(CastleRights::all());

        let ending = game_ending(&mut board, &MoveGenerator::default(), Color::Black, &[]);
        assert!(matches!(ending, Some(GameEnding::Checkmate)));
    }

    #[test]
    fn test_is_endgame_one_minor_piece() {
        let mut board = chess_position! {
            .......k
            .......b
            ........
            ...q....
            ........
            ........
            ........
            K.......
        };

        assert!(!is_endgame(&board));

        board.remove(D5);
        assert!(is_endgame(&board));
    }

    #[test]
    fn test_is_endgame_both_sides_no_queens() {
        let mut board = chess_position! {
            .......k
            .......q
            ........
            ...b....
            ........
            ........
            .Q......
            K.......
        };

        assert!(!is_endgame(&board));

        board.remove(H7);
        board.remove(B2);
        assert!(is_endgame(&board));
    }

    #[test]
    fn test_default_is_not_endgame() {
        let board = Board::default();
        assert!(!is_endgame(&board));
    }

    #[test]
    fn test_passed_pawn_detection() {
        // White pawn on e5 with no black pawns on d/e/f files ahead
        let board = chess_position! {
            ........
            ........
            ........
            ....P...
            ........
            ........
            ........
            ........
        };
        let phase = game_phase(&board);
        let white_pawns = board.pieces(Color::White).locate(Piece::Pawn);
        let black_pawns = board.pieces(Color::Black).locate(Piece::Pawn);
        // Should have a positive pawn structure score (passed pawn for White)
        let score = pawn_structure_eval(white_pawns, black_pawns, phase);
        assert!(
            score > 0,
            "Expected positive score for passed pawn, got {}",
            score
        );
    }

    #[test]
    fn test_doubled_pawn_penalty() {
        // White has doubled pawns on e-file
        let board = chess_position! {
            ........
            ........
            ........
            ....P...
            ....P...
            ........
            ........
            ........
        };
        let phase = game_phase(&board);
        let white_pawns = board.pieces(Color::White).locate(Piece::Pawn);
        let black_pawns = board.pieces(Color::Black).locate(Piece::Pawn);
        let doubled_score = pawn_structure_eval(white_pawns, black_pawns, phase);

        // Compare: single pawn on e5 (no doubled penalty)
        let single = chess_position! {
            ........
            ........
            ........
            ....P...
            ........
            ........
            ........
            ........
        };
        let single_wp = single.pieces(Color::White).locate(Piece::Pawn);
        let single_bp = single.pieces(Color::Black).locate(Piece::Pawn);
        let single_score = pawn_structure_eval(single_wp, single_bp, phase);

        // Doubled should score less than single due to the penalty
        assert!(
            doubled_score < single_score * 2,
            "Doubled pawn ({}) should be less than 2x single ({})",
            doubled_score,
            single_score
        );
    }

    #[test]
    fn test_isolated_pawn_penalty() {
        // White pawn on a-file isolated (no pawns on b-file)
        let isolated = chess_position! {
            ........
            ........
            ........
            P.......
            ........
            ........
            ........
            ........
        };
        let phase = game_phase(&isolated);
        let iso_wp = isolated.pieces(Color::White).locate(Piece::Pawn);
        let iso_bp = isolated.pieces(Color::Black).locate(Piece::Pawn);
        let iso_score = pawn_structure_eval(iso_wp, iso_bp, phase);

        // Connected pawn on a5 with support on b4
        let connected = chess_position! {
            ........
            ........
            ........
            P.......
            .P......
            ........
            ........
            ........
        };
        let conn_wp = connected.pieces(Color::White).locate(Piece::Pawn);
        let conn_bp = connected.pieces(Color::Black).locate(Piece::Pawn);
        let conn_score = pawn_structure_eval(conn_wp, conn_bp, phase);

        // Connected pawns should score better per pawn than isolated
        assert!(
            conn_score > iso_score,
            "Connected pawns ({}) should score better than isolated ({})",
            conn_score,
            iso_score
        );
    }

    #[test]
    fn test_bishop_pair_bonus() {
        // White has bishop pair
        let board = chess_position! {
            ........
            ........
            ........
            ........
            ........
            ........
            ........
            ..BB....
        };
        let phase = game_phase(&board);
        let all_pawns = Bitboard::EMPTY;
        let white_pawns = Bitboard::EMPTY;
        let black_pawns = Bitboard::EMPTY;
        let score = piece_activity_eval(&board, all_pawns, white_pawns, black_pawns, phase);
        assert!(
            score > 0,
            "Expected positive bishop pair bonus, got {}",
            score
        );
    }

    #[test]
    fn test_player_is_in_check() {
        let mut move_generator = MoveGenerator::default();
        let mut board = chess_position! {
            .......k
            .....ppp
            ........
            ...b....
            ........
            ........
            .Q......
            K......q
        };
        board.lose_castle_rights(CastleRights::all());
        board.set_turn(Color::White);

        assert!(player_is_in_check(
            &board,
            &mut move_generator,
            Color::White
        ));
        assert!(!player_is_in_check(
            &board,
            &mut move_generator,
            Color::Black
        ));
    }

    #[test]
    fn test_player_is_in_checkmate() {
        let mut move_generator = MoveGenerator::default();
        let mut board = chess_position! {
            .......k
            ........
            ........
            ........
            ........
            ........
            PPP.....
            .K.....r
        };
        board.lose_castle_rights(CastleRights::all());
        board.set_turn(Color::White);

        assert!(player_is_in_checkmate(
            &mut board,
            &mut move_generator,
            Color::White
        ));
        assert!(!player_is_in_checkmate(
            &mut board,
            &mut move_generator,
            Color::Black
        ));
    }

    #[test]
    fn test_above_rank_mask() {
        // Above rank 0 (rank 1) should include ranks 2-8
        let mask = above_rank_mask(0);
        assert!(!mask.is_empty());
        // Should not include rank 1 squares
        assert!(!mask.overlaps(Bitboard::RANK_1));
        // Should include rank 2
        assert!(mask.overlaps(Bitboard::RANK_2));

        // Above rank 7 (rank 8) should be empty
        assert!(above_rank_mask(7).is_empty());
    }

    #[test]
    fn test_below_rank_mask() {
        // Below rank 7 (rank 8) should include ranks 1-7
        let mask = below_rank_mask(7);
        assert!(!mask.is_empty());
        assert!(!mask.overlaps(Bitboard::RANK_8));
        assert!(mask.overlaps(Bitboard::RANK_7));

        // Below rank 0 (rank 1) should be empty
        assert!(below_rank_mask(0).is_empty());
    }

    #[test]
    fn test_black_passed_pawn_detection() {
        // Black pawn on d4 with no white pawns on c/d/e files below it
        let board = chess_position! {
            ........
            ........
            ........
            ........
            ...p....
            ........
            ........
            ........
        };
        let phase = game_phase(&board);
        let white_pawns = board.pieces(Color::White).locate(Piece::Pawn);
        let black_pawns = board.pieces(Color::Black).locate(Piece::Pawn);
        let score = pawn_structure_eval(white_pawns, black_pawns, phase);
        // Score from White's perspective, so Black's passed pawn should be negative
        assert!(
            score < 0,
            "Expected negative score for Black passed pawn, got {}",
            score
        );
    }

    #[test]
    fn test_king_safety_pawn_shield() {
        // Castled king with pawn shield should score better than exposed king
        let shielded = chess_position! {
            ........
            ........
            ........
            ........
            ........
            ........
            .....PPP
            ......K.
        };
        let exposed = chess_position! {
            ........
            ........
            ........
            ........
            ........
            ........
            ........
            ...K....
        };
        // Use full middlegame phase to maximize king safety relevance
        let phase_full = 24u8;
        let all_pawns_s = shielded.pieces(Color::White).locate(Piece::Pawn);
        let all_pawns_e = Bitboard::EMPTY;
        let white_score_shielded = king_safety_for_color(
            &shielded,
            Color::White,
            all_pawns_s,
            all_pawns_s,
            phase_full,
        );
        let white_score_exposed = king_safety_for_color(
            &exposed,
            Color::White,
            all_pawns_e,
            Bitboard::EMPTY,
            phase_full,
        );
        assert!(
            white_score_shielded > white_score_exposed,
            "Shielded king ({}) should score higher than exposed king ({})",
            white_score_shielded,
            white_score_exposed
        );
    }

    #[test]
    fn test_king_safety_open_file_penalty() {
        // King on open file should score worse than king behind closed files
        let open_file_king = chess_position! {
            ........
            ........
            ........
            ........
            ........
            ........
            PP...PPP
            ..K.....
        };
        let closed_file_king = chess_position! {
            ........
            ........
            ........
            ........
            ........
            ........
            PPP..PPP
            .K......
        };
        let phase_full = 24u8;

        let wp_open = open_file_king.pieces(Color::White).locate(Piece::Pawn);
        let score_open =
            king_safety_for_color(&open_file_king, Color::White, wp_open, wp_open, phase_full);

        let wp_closed = closed_file_king.pieces(Color::White).locate(Piece::Pawn);
        let score_closed = king_safety_for_color(
            &closed_file_king,
            Color::White,
            wp_closed,
            wp_closed,
            phase_full,
        );
        assert!(
            score_closed > score_open,
            "Closed files king ({}) should score higher than open file king ({})",
            score_closed,
            score_open
        );
    }
}
