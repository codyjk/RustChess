use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::Board;
use crate::moves;
use crate::moves::targets::{self, Targets};

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

    board.material_value()
}
