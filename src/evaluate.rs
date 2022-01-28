use crate::board::color::Color;
use crate::board::piece::{Piece, ALL_PIECES};
use crate::board::pieces::Pieces;
use crate::board::Board;
use crate::moves;
use crate::moves::targets::{self, Targets};

mod bonus_tables;

#[derive(Debug)]
pub enum GameEnding {
    Checkmate,
    Stalemate,
    Draw,
}

fn current_player_is_in_check(board: &Board, targets: &mut Targets) -> bool {
    let current_player = board.turn();
    let king = board.pieces(current_player).locate(Piece::King);

    let attacked_squares =
        targets::generate_attack_targets(board, current_player.opposite(), targets);

    king & attacked_squares > 0
}

pub fn game_ending(
    board: &mut Board,
    targets: &mut Targets,
    current_turn: Color,
) -> Option<GameEnding> {
    if board.max_seen_position_count() == 3 {
        return Some(GameEnding::Draw);
    }

    let candidates = moves::generate(board, current_turn, targets);
    let check = current_player_is_in_check(board, targets);

    if candidates.len() == 0 {
        if check {
            return Some(GameEnding::Checkmate);
        } else {
            return Some(GameEnding::Stalemate);
        }
    }

    return None;
}

pub fn score(board: &mut Board, targets: &mut Targets, current_turn: Color) -> f32 {
    match (game_ending(board, targets, current_turn), current_turn) {
        (Some(GameEnding::Checkmate), Color::White) => return f32::NEG_INFINITY,
        (Some(GameEnding::Checkmate), Color::Black) => return f32::INFINITY,
        (Some(GameEnding::Stalemate), Color::White) => return f32::NEG_INFINITY,
        (Some(GameEnding::Stalemate), Color::Black) => return f32::INFINITY,
        (Some(GameEnding::Draw), Color::White) => return f32::NEG_INFINITY,
        (Some(GameEnding::Draw), Color::Black) => return f32::INFINITY,
        _ => (),
    };

    let white = board.pieces(Color::White);
    let black = board.pieces(Color::Black);
    material_score(white) - material_score(black)
}

fn material_score(pieces: Pieces) -> f32 {
    let mut material = 0.;

    for piece in &ALL_PIECES {
        let bonuses = bonus_tables::get(*piece);
        let squares = pieces.locate(*piece);
        let piece_value = f32::from(piece.material_value());

        for i in 0..64 {
            let sq = 1 << i;

            if sq & squares == 0 {
                continue;
            }

            material += piece_value * bonuses[i];
        }
    }

    material
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;

    #[test]
    fn test_starting_material_score() {
        let board = Board::starting_position();
        let white = board.pieces(Color::White);
        let starting_material = material_score(white) - f32::from(Piece::King.material_value());
        // (piece value) * (piece quantity) * (starting tile bonus)
        // 8 * 1 * 1.0 = 8 pawns
        // 1 * 9 * 1.0 = 9 queens
        // 2 * 5 * 1.0 = 10 rooks
        // 2 * 3 * .75 = 4.5 knights
        // 2 * 3 * 1.0 = 6 bishops
        // total = 37.5
        assert_eq!(37.5, starting_material);
    }
}
