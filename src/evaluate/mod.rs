use log::debug;

use crate::board::color::Color;
use crate::board::piece::{Piece, ALL_PIECES};
use crate::board::square::to_algebraic;
use crate::board::Board;
use crate::move_generator::MoveGenerator;

use self::piece_values::material_value;

mod bonus_tables;
mod piece_values;

#[derive(Debug)]
pub enum GameEnding {
    Checkmate,
    Stalemate,
    Draw,
}

fn current_player_is_in_check(board: &Board, move_generator: &mut MoveGenerator) -> bool {
    let current_player = board.turn();
    let king = board.pieces(current_player).locate(Piece::King);
    let attacked_squares = move_generator.get_attack_targets(board, current_player.opposite());

    king & attacked_squares > 0
}

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

pub fn score(board: &mut Board, move_generator: &mut MoveGenerator, current_turn: Color) -> f32 {
    // Check for position repetition
    if board.max_seen_position_count() == 3 {
        match current_turn {
            Color::White => return f32::NEG_INFINITY,
            Color::Black => return f32::INFINITY,
        }
    }

    match (
        game_ending(board, move_generator, current_turn),
        current_turn,
    ) {
        (Some(GameEnding::Checkmate), Color::White) => f32::INFINITY,
        (Some(GameEnding::Checkmate), Color::Black) => f32::NEG_INFINITY,
        (Some(GameEnding::Stalemate), Color::White) => f32::NEG_INFINITY,
        (Some(GameEnding::Stalemate), Color::Black) => f32::INFINITY,
        (Some(GameEnding::Draw), Color::White) => f32::NEG_INFINITY,
        (Some(GameEnding::Draw), Color::Black) => f32::INFINITY,
        _ => material_score(board, Color::White) - material_score(board, Color::Black),
    }
}

fn material_score(board: &Board, color: Color) -> f32 {
    let mut material = 0.;
    let pieces = board.pieces(color);

    for &piece in &ALL_PIECES {
        let bonuses = bonus_tables::get(piece);
        let squares = pieces.locate(piece);
        let piece_value = f32::from(material_value(piece));

        for i in 0..64 {
            let sq = 1 << i;

            if sq & squares == 0 {
                continue;
            }

            // need to flip around the bonuses if calculating for black
            let bonus_i = match color {
                Color::White => i,
                Color::Black => {
                    let rank = i / 8;
                    let file = i % 8;
                    (7 - file) + ((7 - rank) * 8)
                }
            };

            material += piece_value + bonuses[bonus_i];

            let square_name = to_algebraic(sq);
            debug!(
                "{} at {} has value {} + bonus {} (total {})",
                piece, square_name, piece_value, bonuses[bonus_i], material
            );
        }
    }

    material
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{
        castle_rights::ALL_CASTLE_RIGHTS,
        square::{A1, H7, H8},
        Board,
    };

    #[test]
    fn test_starting_material_score() {
        let board = Board::starting_position();
        println!("Testing board:\n{}", board);

        let white_score = material_score(&board, Color::White);
        assert_eq!(white_score, 100.0);

        let black_score = material_score(&board, Color::Black);
        assert_eq!(black_score, 100.0);
    }

    #[test]
    fn test_game_ending_stalemate() {
        let mut board = Board::new();
        let mut move_generator = MoveGenerator::new();

        board.put(A1, Piece::King, Color::White).unwrap();
        board.put(H8, Piece::King, Color::Black).unwrap();
        board.set_turn(Color::Black);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);
        println!("Testing board:\n{}", board);

        let ending = game_ending(&mut board, &mut move_generator, Color::Black);
        matches!(ending, Some(GameEnding::Stalemate));
    }

    #[test]
    fn test_game_ending_checkmate() {
        let mut board = Board::new();
        let mut move_generator = MoveGenerator::new();

        board.put(A1, Piece::King, Color::White).unwrap();
        board.put(H8, Piece::King, Color::Black).unwrap();
        board.put(H7, Piece::Queen, Color::Black).unwrap();
        board.set_turn(Color::Black);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);
        println!("Testing board:\n{}", board);

        let ending = game_ending(&mut board, &mut move_generator, Color::Black);
        matches!(ending, Some(GameEnding::Checkmate));
    }
}
