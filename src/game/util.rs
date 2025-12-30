use crate::{board::color::Color, chess_move::chess_move::ChessMove};

use super::{display::GameDisplay, engine::Engine};

pub fn print_board_and_stats(
    engine: &Engine,
    moves: Vec<(ChessMove, String)>,
    current_turn: Color,
) {
    let mut ui = GameDisplay::new();

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

    let opening_name = engine.get_book_line_name();
    ui.render_game_state(
        engine.board(),
        current_turn,
        last_move,
        Some(&stats),
        opening_name.as_deref(),
    );
}
