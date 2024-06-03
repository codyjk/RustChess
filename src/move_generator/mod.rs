mod magic_table;
mod targets;

use std::num::NonZeroUsize;

use crate::board::castle_rights_bitmask::{
    BLACK_KINGSIDE_RIGHTS, BLACK_QUEENSIDE_RIGHTS, WHITE_KINGSIDE_RIGHTS, WHITE_QUEENSIDE_RIGHTS,
};
use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::Board;
use crate::chess_move::castle::CastleChessMove;
use crate::chess_move::en_passant::EnPassantChessMove;
use crate::chess_move::pawn_promotion::PawnPromotionChessMove;
use crate::chess_move::standard::StandardChessMove;
use crate::chess_move::ChessMove;
use common::bitboard::bitboard::Bitboard;
use common::bitboard::square::*;
use lru::LruCache;
use targets::{PieceTarget, Targets};

use self::targets::{generate_pawn_attack_targets, generate_pawn_move_targets};

pub const PAWN_PROMOTIONS: [Piece; 4] = [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight];

/// Implements a move generation algorithm that generates all possible moves for
/// a given board state. The algorithm is optimized to cache the results of
/// previous move generation calls to avoid redundant work.
pub struct MoveGenerator {
    targets: Targets,
    // (board, color) -> moves
    cache: LruCache<(u64, u8), Vec<ChessMove>>,
    hit_count: usize,
}

impl Default for MoveGenerator {
    fn default() -> Self {
        Self {
            targets: Targets::default(),
            // Potentially a lot of memory, but helpful for high depths
            cache: LruCache::new(NonZeroUsize::new(100_000).unwrap()),
            hit_count: 0,
        }
    }
}

impl MoveGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cache_hit_count(&self) -> usize {
        self.hit_count
    }

    pub fn cache_entry_count(&self) -> usize {
        self.cache.len()
    }

    pub fn reset_cache_hit_count(&mut self) {
        self.hit_count = 0;
    }

    pub fn generate_moves(&mut self, board: &mut Board, color: Color) -> Vec<ChessMove> {
        let key = (board.current_position_hash(), color as u8);
        if let Some(moves) = self.cache.get(&key) {
            self.hit_count += 1;
            return moves.clone();
        }

        let moves = generate_valid_moves(board, color, &mut self.targets);
        self.cache.put(key, moves.clone());
        moves
    }

    pub fn count_positions(&mut self, depth: u8, board: &mut Board, color: Color) -> usize {
        let candidates = self.generate_moves(board, color);
        let mut count = candidates.len();

        if depth == 0 {
            return count;
        }

        let next_color = color.opposite();

        for chess_move in candidates.iter() {
            chess_move.apply(board).unwrap();
            count += self.count_positions(depth - 1, board, next_color);
            chess_move.undo(board).unwrap();
        }

        count
    }

    pub fn get_attack_targets(&mut self, board: &Board, color: Color) -> Bitboard {
        let board_hash = board.current_position_hash();

        if let Some(cached_targets) = self.targets.get_cached_attack(color, board_hash) {
            return cached_targets;
        }

        let attack_targets = self.targets.generate_attack_targets(board, color);

        self.targets.cache_attack(color, board_hash, attack_targets);

        attack_targets
    }
}

/// Generates all valid moves for the given board state and color. The code is
/// implemented in such a way that copying of lists of moves is minimized.
fn generate_valid_moves(board: &mut Board, color: Color, targets: &mut Targets) -> Vec<ChessMove> {
    let mut moves = Vec::new();

    generate_knight_moves(&mut moves, board, color, targets);
    generate_sliding_moves(&mut moves, board, color, targets);
    generate_king_moves(&mut moves, board, color, targets);
    generate_pawn_moves(&mut moves, board, color);
    generate_castle_moves(&mut moves, board, color, targets);
    remove_invalid_moves(&mut moves, board, color, targets);

    moves
}

/// Generates all pawn moves, regardless of which rank the pawn is on.
/// To get promotions, the code later applies some special logic to find the
/// targets that are at the end of the board, and then expand those targets
/// into the candidate promotion pieces.
fn generate_pawn_moves(moves: &mut Vec<ChessMove>, board: &Board, color: Color) {
    let mut piece_targets = generate_pawn_move_targets(board, color);
    let attack_targets = generate_pawn_attack_targets(board, color);
    let opponent_pieces = board.pieces(color.opposite()).occupied();
    attack_targets.iter().for_each(|&(piece, target)| {
        if target.overlaps(opponent_pieces) {
            piece_targets.push((piece, target & opponent_pieces));
        }
    });

    let mut all_pawn_moves = Vec::new();
    expand_piece_targets(&mut all_pawn_moves, board, color, piece_targets);

    let (mut standard_pawn_moves, promotable_pawn_moves): (Vec<_>, Vec<_>) =
        all_pawn_moves.into_iter().partition(|chess_move| {
            let to_square = chess_move.to_square();
            let promotion_rank = match color {
                Color::White => Bitboard::RANK_8,
                Color::Black => Bitboard::RANK_1,
            };
            !to_square.overlaps(promotion_rank)
        });

    for promotable_pawn_move in promotable_pawn_moves.iter() {
        let from_square = promotable_pawn_move.from_square();
        let to_square = promotable_pawn_move.to_square();
        let capture = promotable_pawn_move.capture();
        for &promotion in &PAWN_PROMOTIONS {
            let pawn_promotion =
                PawnPromotionChessMove::new(from_square, to_square, capture, promotion);
            moves.push(ChessMove::PawnPromotion(pawn_promotion));
        }
    }
    moves.append(&mut standard_pawn_moves);
    generate_en_passant_moves(moves, board, color);
}

fn generate_en_passant_moves(moves: &mut Vec<ChessMove>, board: &Board, color: Color) {
    let en_passant_target = board.peek_en_passant_target();

    if en_passant_target.is_empty() {
        return;
    }

    let pawns = board.pieces(color).locate(Piece::Pawn);

    let attacks_west = match color {
        Color::White => (pawns << 9) & !Bitboard::A_FILE,
        Color::Black => (pawns >> 7) & !Bitboard::A_FILE,
    };

    let attacks_east = match color {
        Color::White => (pawns << 7) & !Bitboard::H_FILE,
        Color::Black => (pawns >> 9) & !Bitboard::H_FILE,
    };

    if attacks_west.overlaps(en_passant_target) {
        let from_square = match color {
            Color::White => en_passant_target >> 9,
            Color::Black => en_passant_target << 7,
        };
        let en_passant_move = EnPassantChessMove::new(from_square, en_passant_target);
        moves.push(ChessMove::EnPassant(en_passant_move));
    }

    if attacks_east.overlaps(en_passant_target) {
        let from_square = match color {
            Color::White => en_passant_target >> 7,
            Color::Black => en_passant_target << 9,
        };
        let en_passant_move = EnPassantChessMove::new(from_square, en_passant_target);
        moves.push(ChessMove::EnPassant(en_passant_move));
    }
}

fn generate_knight_moves(
    moves: &mut Vec<ChessMove>,
    board: &Board,
    color: Color,
    targets: &Targets,
) {
    expand_piece_targets(
        moves,
        board,
        color,
        targets.generate_targets_from_precomputed_tables(board, color, Piece::Knight),
    )
}

fn generate_sliding_moves(
    moves: &mut Vec<ChessMove>,
    board: &Board,
    color: Color,
    targets: &Targets,
) {
    let piece_targets = targets.generate_sliding_targets(board, color);
    expand_piece_targets(moves, board, color, piece_targets)
}

fn expand_piece_targets(
    moves: &mut Vec<ChessMove>,
    board: &Board,
    color: Color,
    piece_targets: Vec<PieceTarget>,
) {
    // TODO(codyjk): Do we need to loop over every square?
    for (piece, target_squares) in piece_targets {
        let piece_sq = assert_square(piece);
        for &target in &ORDERED_SQUARES {
            if !target_squares.overlaps(target) {
                continue;
            }

            let capture = board
                .pieces(color.opposite())
                .get(target)
                .map(|piece| (piece, color.opposite()));

            let standard_move = StandardChessMove::new(piece_sq, target, capture);
            moves.push(ChessMove::Standard(standard_move));
        }
    }
}

fn generate_king_moves(moves: &mut Vec<ChessMove>, board: &Board, color: Color, targets: &Targets) {
    expand_piece_targets(
        moves,
        board,
        color,
        targets.generate_targets_from_precomputed_tables(board, color, Piece::King),
    )
}

fn generate_castle_moves(
    moves: &mut Vec<ChessMove>,
    board: &Board,
    color: Color,
    targets: &mut Targets,
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
        && !kingside_transit_square.overlaps(attacked_squares)
        && !kingside_transit_square.overlaps(occupied)
        && !kingside_target_square.overlaps(occupied)
    {
        let castle_move = CastleChessMove::castle_kingside(color);
        moves.push(ChessMove::Castle(castle_move));
    }

    if queenside_rights > 0
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
    candidates: &mut Vec<ChessMove>,
    board: &mut Board,
    color: Color,
    targets: &mut Targets,
) {
    let mut valid_moves = Vec::new();

    // simulate each chess_move and see if it leaves the player's king in check.
    // if it does, it's invalid.
    for chess_move in candidates.drain(..) {
        chess_move.apply(board).unwrap();
        let king = board.pieces(color).locate(Piece::King);
        let attacked_squares = targets.generate_attack_targets(board, color.opposite());
        chess_move.undo(board).unwrap();

        if !king.overlaps(attacked_squares) {
            valid_moves.push(chess_move);
        }
    }

    candidates.append(&mut valid_moves);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess_move::ChessMove;
    use crate::{
        castle_kingside, castle_queenside, chess_position, en_passant_move, promotion, std_move,
    };

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

        let mut expected_white_moves = vec![
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

        let mut expected_black_moves = vec![
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

        let mut white_moves = vec![];
        generate_pawn_moves(&mut white_moves, &board, Color::White);
        white_moves.sort();
        assert_eq!(expected_white_moves, white_moves);

        let mut black_moves = vec![];
        generate_pawn_moves(&mut black_moves, &board, Color::Black);
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

        let mut expected_moves = vec![std_move!(B2, B3), std_move!(B2, B4), std_move!(C3, C4)];
        expected_moves.sort();

        let mut moves = vec![];
        generate_pawn_moves(&mut moves, &board, Color::White);
        moves.sort();

        assert_eq!(expected_moves, moves);
    }

    #[test]
    fn test_generate_knight_moves() {
        let targets = Targets::new();
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

        let mut expected_white_moves = vec![
            std_move!(C3, D5, (Piece::Pawn, Color::Black)),
            std_move!(C3, E2),
            std_move!(C3, D1),
            std_move!(C3, B5),
            std_move!(C3, A4),
            std_move!(C3, A2),
            std_move!(C3, B1),
        ];
        expected_white_moves.sort();

        let mut expected_black_moves = vec![
            std_move!(H6, G8),
            std_move!(H6, F7),
            std_move!(H6, F5),
            std_move!(H6, G4),
        ];
        expected_black_moves.sort();

        let mut white_moves = vec![];
        generate_knight_moves(&mut white_moves, &board, Color::White, &targets);
        white_moves.sort();
        assert_eq!(expected_white_moves, white_moves);

        let mut black_moves = vec![];
        generate_knight_moves(&mut black_moves, &board, Color::Black, &targets);
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

        let mut expected_moves = vec![
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

        let mut moves = vec![];
        generate_sliding_moves(&mut moves, &board, Color::White, &Targets::new());
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

        let mut expected_moves = vec![std_move!(A2, A1), std_move!(A2, A3)];
        expected_moves.sort();

        let mut moves = vec![];
        generate_sliding_moves(&mut moves, &board, Color::White, &Targets::new());
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

        let mut expected_moves = vec![
            std_move!(E5, D4),
            std_move!(E5, D6),
            std_move!(E5, F4),
            std_move!(E5, F6),
            std_move!(E5, G3),
            std_move!(E5, G7, (Piece::Pawn, Color::Black)),
            std_move!(E5, H2),
        ];
        expected_moves.sort();

        let mut moves = vec![];
        generate_sliding_moves(&mut moves, &board, Color::White, &Targets::new());
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

        let mut expected_moves = vec![
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

        let mut moves = vec![];
        generate_sliding_moves(&mut moves, &board, Color::White, &Targets::new());
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

        let mut expected_moves = vec![std_move!(A1, A2), std_move!(A1, B1), std_move!(A1, B2)];
        expected_moves.sort();

        let mut moves = vec![];
        generate_king_moves(&mut moves, &board, Color::White, &Targets::new());
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

        let mut expected_moves = vec![
            std_move!(E1, D1),
            std_move!(E1, D2, (Piece::Pawn, Color::Black)),
            std_move!(E1, E2),
            std_move!(E1, F1),
            std_move!(E1, F2),
        ];
        expected_moves.sort();

        let mut moves = vec![];
        generate_king_moves(&mut moves, &board, Color::White, &Targets::new());
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

        let mut expected_moves = vec![
            std_move!(E5, D4),
            std_move!(E5, D5),
            std_move!(E5, D6),
            std_move!(E5, E4, (Piece::Pawn, Color::Black)),
            std_move!(E5, F4),
            std_move!(E5, F5),
            std_move!(E5, F6),
        ];
        expected_moves.sort();

        let mut moves = vec![];
        generate_king_moves(&mut moves, &board, Color::White, &Targets::new());
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
        assert_eq!(C3, board.peek_en_passant_target());

        let mut expected_black_moves = vec![std_move!(D4, D3), en_passant_move!(D4, C3)];
        expected_black_moves.sort();

        let mut moves = vec![];
        generate_pawn_moves(&mut moves, &board, Color::Black);
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

        let mut expected_white_moves = vec![
            castle_kingside!(Color::White),
            castle_queenside!(Color::White),
        ];
        expected_white_moves.sort();

        let mut expected_black_moves = vec![
            castle_kingside!(Color::Black),
            castle_queenside!(Color::Black),
        ];
        expected_black_moves.sort();

        let mut targets = Targets::new();

        let mut white_moves = vec![];
        generate_castle_moves(&mut white_moves, &board, Color::White, &mut targets);
        white_moves.sort();

        let mut black_moves = vec![];
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

        let expected_white_moves = vec![castle_kingside!(Color::White)];
        let expected_black_moves = vec![castle_queenside!(Color::Black)];

        let mut targets = Targets::new();
        targets.generate_attack_targets(&board, Color::Black);

        let mut white_moves = vec![];
        generate_castle_moves(&mut white_moves, &board, Color::White, &mut targets);

        let mut black_moves = vec![];
        generate_castle_moves(&mut black_moves, &board, Color::Black, &mut targets);

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

        let expected_white_moves: Vec<ChessMove> = vec![];
        let mut white_moves = vec![];
        generate_castle_moves(&mut white_moves, &board, Color::White, &mut Targets::new());

        assert_eq!(expected_white_moves, white_moves);
    }
}
