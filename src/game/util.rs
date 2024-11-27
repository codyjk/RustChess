use crate::{board::color::Color, chess_move::chess_move::ChessMove};

use super::{engine::Engine, ui::GameUI};

pub fn print_board_and_stats(
    engine: &Engine,
    moves: Vec<(ChessMove, String)>,
    current_turn: Color,
) {
    let mut ui = GameUI::new();

    let last_move = engine.last_move().and_then(|mv| {
        moves
            .iter()
            .find(|(m, _)| m == &mv)
            .map(|(m, n)| (m, n.as_str()))
    });

    let stats = format!(
        "* Score: {}\n* Positions searched: {}\n* Search depth: {}",
        engine
            .get_search_stats()
            .last_score
            .map_or("-".to_string(), |s| s.to_string()),
        engine.get_search_stats().positions_searched,
        engine.get_search_stats().depth
    );

    ui.render_game_state(engine.board(), current_turn, last_move, Some(&stats));
}
