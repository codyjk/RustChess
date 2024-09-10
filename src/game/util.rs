use crate::board::Board;
use crate::chess_move::chess_move::ChessMove;
use crate::game::game::Game;
use common::bitboard::square::from_rank_file;

pub fn print_board_and_stats(
    game: &mut Game,
    enumerated_candidate_moves: Vec<(ChessMove, String)>,
) {
    let board = game.board();
    let last_move_algebraic = match game.last_move() {
        Some(chess_move) => enumerated_candidate_moves
            .iter()
            .find(|(move_, _)| move_ == &chess_move)
            .map(|(_, notation)| notation.clone())
            .unwrap_or_else(|| "-".to_string()),
        None => "-".to_string(),
    };
    let searched_position_count = game.searched_position_count();
    let searched_position_message = match searched_position_count {
        0 => "0 (book move)".to_string(),
        _ => format!(
            "{} (depth {})",
            searched_position_count,
            game.search_depth()
        ),
    };
    println!(
        "{} chose move: {}\n",
        game.board().turn().opposite(),
        last_move_algebraic
    );
    print_board(game.board());
    println!("* Turn: {}", board.turn());
    println!("* Halfmove clock: {}", board.halfmove_clock());
    println!("* Score: {}", game.score(board.turn()));
    println!("* Positions searched: {}", searched_position_message);
}

pub fn print_enter_move_prompt() {
    println!("Enter your move:");
}

pub fn print_board(board: &Board) {
    let mut board_str = String::new();
    board_str.push_str("    a   b   c   d   e   f   g   h\n");
    board_str.push_str("  ┌───┬───┬───┬───┬───┬───┬───┬───┐\n");
    for rank in 0..8 {
        let transposed_rank = 7 - rank;
        board_str.push_str(&format!("{} │", transposed_rank + 1));
        for file in 0..8 {
            let square = from_rank_file(transposed_rank, file);
            let piece = board.get(square);
            let piece_str = match piece {
                Some((piece, color)) => piece.to_unicode_piece_char(color).to_string(),
                None => if (rank + file) % 2 == 0 { " " } else { "·" }.to_string(),
            };
            board_str.push_str(&format!(" {} │", piece_str));
        }
        board_str.push_str(&format!(" {}\n", transposed_rank + 1));
        if rank < 7 {
            board_str.push_str("  ├───┼───┼───┼───┼───┼───┼───┼───┤\n");
        } else {
            board_str.push_str("  └───┴───┴───┴───┴───┴───┴───┴───┘\n");
        }
    }
    board_str.push_str("    a   b   c   d   e   f   g   h\n");
    println!("{}", board_str);
}
