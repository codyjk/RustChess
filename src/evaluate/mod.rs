use common::bitboard::bitboard::Bitboard;

use crate::board::color::Color;
use crate::board::piece::{Piece, ALL_PIECES};
use crate::board::Board;
use crate::evaluate::evaluation_tables::BONUS_TABLES;
use crate::move_generator::MoveGenerator;

use self::evaluation_tables::{
    MATERIAL_VALUES, SQUARE_TO_BLACK_BONUS_INDEX, SQUARE_TO_WHITE_BONUS_INDEX,
};

mod evaluation_tables;

// These scores are significantly larger than any possible material value,
// and therefore will incentivize the engine to select for (or against) their own
// (or the opponent's) win or draw condition.
const BLACK_WINS: i16 = i16::MIN / 2;
const WHITE_WINS: i16 = i16::MAX / 2;

#[derive(Debug)]
pub enum GameEnding {
    Checkmate,
    Stalemate,
    Draw,
}

#[inline(always)]
pub fn current_player_is_in_check(board: &Board, move_generator: &mut MoveGenerator) -> bool {
    let current_player = board.turn();
    let king = board.pieces(current_player).locate(Piece::King);
    let attacked_squares = move_generator.get_attack_targets(board, current_player.opposite());

    king.overlaps(attacked_squares)
}

/// Returns the game ending state if the game has ended, otherwise returns None.
#[inline(always)]
pub fn game_ending(
    board: &mut Board,
    move_generator: &mut MoveGenerator,
    current_turn: Color,
) -> Option<GameEnding> {
    if board.max_seen_position_count() == 3 {
        return Some(GameEnding::Draw);
    }

    if board.halfmove_clock() >= 50 {
        return Some(GameEnding::Draw);
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

/// Returns the score of the board from the perspective of the current player.
#[inline(always)]
pub fn score(board: &mut Board, move_generator: &mut MoveGenerator, current_turn: Color) -> i16 {
    // Check for position repetition
    if board.max_seen_position_count() == 3 {
        match current_turn {
            Color::White => return BLACK_WINS,
            Color::Black => return WHITE_WINS,
        }
    }

    match (
        game_ending(board, move_generator, current_turn),
        current_turn,
    ) {
        (Some(GameEnding::Checkmate), Color::White) => WHITE_WINS,
        (Some(GameEnding::Checkmate), Color::Black) => BLACK_WINS,
        (Some(GameEnding::Stalemate), Color::White) => BLACK_WINS,
        (Some(GameEnding::Stalemate), Color::Black) => WHITE_WINS,
        (Some(GameEnding::Draw), Color::White) => BLACK_WINS,
        (Some(GameEnding::Draw), Color::Black) => WHITE_WINS,
        _ => material_score(board, Color::White) - material_score(board, Color::Black),
    }
}

/// Returns the material score of the board for the given player. The bonus tables
/// incentivize the placement of pieces on specific parts of the board (e.g.
/// knights towards the center, bishops on long diagonals, etc.).
#[inline(always)]
fn material_score(board: &Board, color: Color) -> i16 {
    let mut material = 0;
    let pieces = board.pieces(color);

    // The code shares the bonuses between white and black. To achieve this, the
    // lookup against the bonus table is transposed depending on which player
    // the bonus is being evaluated for.
    let index_lookup = match color {
        Color::White => SQUARE_TO_WHITE_BONUS_INDEX,
        Color::Black => SQUARE_TO_BLACK_BONUS_INDEX,
    };
    let is_endgame = is_endgame(board) as usize;

    for &piece in &ALL_PIECES {
        let squares = pieces.locate(piece);
        let piece_value = MATERIAL_VALUES[piece as usize];

        for i in 0..64 {
            let sq = Bitboard(1 << i);
            if !sq.overlaps(squares) {
                continue;
            }

            let bonus_table = BONUS_TABLES[piece as usize][is_endgame];
            let bonus = bonus_table[index_lookup[i]];

            material += piece_value + bonus;
        }
    }

    material
}

/// Endgame conditions:
/// 1. Both sides have no queens or
/// 2. Every side which has a queen has additionally no other pieces or one minorpiece maximum.
#[inline(always)]
fn is_endgame(board: &Board) -> bool {
    let white_queen = board.pieces(Color::White).locate(Piece::Queen);
    let black_queen = board.pieces(Color::Black).locate(Piece::Queen);
    let white_king = board.pieces(Color::White).locate(Piece::King);
    let black_king = board.pieces(Color::Black).locate(Piece::King);

    let white_non_queen_pieces = board.pieces(Color::White).occupied() & !white_queen & !white_king;
    let black_non_queen_pieces = board.pieces(Color::Black).occupied() & !black_queen & !black_king;
    let white_minor_pieces = white_non_queen_pieces & !white_king;
    let black_minor_pieces = black_non_queen_pieces & !black_king;

    let both_sides_have_no_queens = white_queen.is_empty() && black_queen.is_empty();
    let white_has_no_queen_or_one_minor_piece =
        white_queen.is_empty() && white_minor_pieces.count_ones() <= 1;
    let black_has_no_queen_or_one_minor_piece =
        black_queen.is_empty() && black_minor_pieces.count_ones() <= 1;

    both_sides_have_no_queens
        || (white_has_no_queen_or_one_minor_piece && black_has_no_queen_or_one_minor_piece)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        board::{castle_rights_bitmask::ALL_CASTLE_RIGHTS, Board},
        chess_position,
    };
    use common::bitboard::square::*;

    #[test]
    fn test_starting_material_score() {
        let board = Board::starting_position();
        println!("Testing board:\n{}", board);

        let white_score = material_score(&board, Color::White);
        assert_eq!(white_score, 23905);

        let black_score = material_score(&board, Color::Black);
        assert_eq!(black_score, 23905);
    }

    #[test]
    fn test_game_ending_stalemate() {
        let mut board = chess_position! {
            .......k
            ........
            ........
            ........
            ........
            ........
            ........
            K.......
        };
        board.set_turn(Color::Black);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);
        println!("Testing board:\n{}", board);

        let ending = game_ending(&mut board, &mut MoveGenerator::new(), Color::Black);
        matches!(ending, Some(GameEnding::Stalemate));
    }

    #[test]
    fn test_game_ending_checkmate() {
        let mut board = chess_position! {
            .......k
            .......q
            ........
            ........
            ........
            ........
            ........
            K.......
        };
        board.set_turn(Color::Black);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);
        println!("Testing board:\n{}", board);

        let ending = game_ending(&mut board, &mut MoveGenerator::new(), Color::Black);
        matches!(ending, Some(GameEnding::Checkmate));
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
        println!("Testing board:\n{}", board);

        assert!(!is_endgame(&board));

        board.remove(D5);
        println!("Testing board:\n{}", board);
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
        println!("Testing board:\n{}", board);

        assert!(!is_endgame(&board));

        board.remove(H7);
        board.remove(B2);
        println!("Testing board:\n{}", board);
        assert!(is_endgame(&board));
    }

    #[test]
    fn test_starting_position_is_not_endgame() {
        let board = Board::starting_position();
        assert!(!is_endgame(&board));
    }

    #[test]
    fn test_pawn_material_bonus_on_final_rank() {
        let board = chess_position! {
            ........
            .......P
            ........
            ........
            ........
            ........
            .......p
            ........
        };
        println!("Testing board:\n{}", board);

        let white_score = material_score(&board, Color::White);
        assert_eq!(white_score, 150);

        let black_score = material_score(&board, Color::Black);
        assert_eq!(black_score, 150);
    }
}
