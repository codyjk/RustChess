use crate::board::piece::Piece;
use crate::board::Board;
use crate::chess_move::capture::Capture;
use crate::chess_move::castle::CastleChessMove;
use crate::chess_move::chess_move::ChessMove;
use crate::chess_move::en_passant::EnPassantChessMove;
use crate::chess_move::pawn_promotion::PawnPromotionChessMove;
use crate::chess_move::standard::StandardChessMove;
use crate::evaluate::GameEnding;
use crate::game::engine::EngineConfig;
use crate::game::stockfish_interface::Stockfish;
use crate::game::util::print_board_and_stats;
use crate::{board::color::Color, game::engine::Engine};
use common::bitboard::square::*;
use std::time::{Duration, Instant};
use termion::{clear, cursor};

const GAMES_PER_ELO: usize = 10;
const ELO_INCREMENT: u32 = 25;
const TIME_LIMIT: u64 = 1000; // 1 second per move

pub fn determine_stockfish_elo(depth: u8, starting_elo: u32) {
    let mut stockfish = match Stockfish::new() {
        Ok(sf) => sf,
        Err(_) => {
            println!("Error: Stockfish not found. Please ensure it's installed and in your PATH.");
            return;
        }
    };

    let mut current_elo = starting_elo;
    let mut wins = 0;
    let mut losses = 0;
    let mut draws = 0;
    let mut total_games = 0;
    let mut engine_total_time = Duration::new(0, 0);
    let mut stockfish_total_time = Duration::new(0, 0);

    loop {
        stockfish.set_elo(current_elo).unwrap();

        for _ in 0..GAMES_PER_ELO {
            let (result, engine_time, sf_time) = play_game(&mut stockfish, depth);
            total_games += 1;
            engine_total_time += engine_time;
            stockfish_total_time += sf_time;

            match result {
                GameResult::Win => wins += 1,
                GameResult::Loss => losses += 1,
                GameResult::Draw => draws += 1,
            }

            display_progress(
                current_elo,
                wins,
                losses,
                draws,
                total_games,
                engine_total_time,
                stockfish_total_time,
            );

            if is_elo_determined(wins, losses, total_games) {
                println!("\nFinal ELO determination: {}", current_elo);
                return;
            }
        }

        if wins > losses {
            current_elo += ELO_INCREMENT;
        } else {
            current_elo -= ELO_INCREMENT;
        }

        wins = 0;
        losses = 0;
        draws = 0;
    }
}

fn play_game(stockfish: &mut Stockfish, depth: u8) -> (GameResult, Duration, Duration) {
    let mut engine = Engine::with_config(EngineConfig {
        search_depth: depth,
    });
    let mut moves = Vec::new();
    let mut engine_time = Duration::new(0, 0);
    let mut stockfish_time = Duration::new(0, 0);

    let engine_color = Color::random();

    print!("{}{}", clear::All, cursor::Goto(1, 1));

    loop {
        if let Some(result) = engine.check_game_over() {
            return (
                match result {
                    GameEnding::Checkmate => {
                        if engine.board().turn() == engine_color {
                            GameResult::Loss
                        } else {
                            GameResult::Win
                        }
                    }
                    _ => GameResult::Draw,
                },
                engine_time,
                stockfish_time,
            );
        }

        let start_time = Instant::now();
        let candidate_moves = engine.get_valid_moves().clone();
        let current_turn = engine.board().turn();

        let chess_move = if current_turn == engine_color {
            let chess_move = engine.get_best_move().unwrap();
            engine_time += start_time.elapsed();
            chess_move
        } else {
            let (sf_move, sf_time) = stockfish
                .get_best_move(&moves.join(" "), TIME_LIMIT)
                .unwrap();
            stockfish_time += Duration::from_millis(sf_time);
            create_chess_move_from_uci(&sf_move, &engine.board())
        };

        engine.apply_chess_move(chess_move.clone()).unwrap();
        moves.push(chess_move.to_uci());

        print!("{}{}", clear::All, cursor::Goto(1, 1));
        print_board_and_stats(&mut engine, candidate_moves, current_turn);
        println!("* Engine color: {}", engine_color);
        println!("* Stockfish color: {}", engine_color.opposite());
        println!("* Current Stockfish ELO: {}", stockfish.get_elo());
        println!(
            "* Engine latency: {:.2}ms",
            engine_time.as_millis() as f32 / TIME_LIMIT as f32
        );
        println!(
            "* Stockfish latency: {:.2}ms",
            stockfish_time.as_millis() as f32 / TIME_LIMIT as f32
        );

        engine.board_mut().toggle_turn();
    }
}

fn create_chess_move_from_uci(uci: &str, board: &Board) -> ChessMove {
    let from = square_string_to_bitboard(&uci[0..2]);
    let to = square_string_to_bitboard(&uci[2..4]);
    let promotion = uci.chars().nth(4).map(|c| match c {
        'q' => Piece::Queen,
        'r' => Piece::Rook,
        'b' => Piece::Bishop,
        'n' => Piece::Knight,
        _ => panic!("Invalid promotion piece"),
    });

    let piece = board.get(from).unwrap().0;
    let capture = board.get(to).map(|(p, _)| Capture(p));

    match (piece, promotion) {
        (Piece::Pawn, Some(promote_to)) => {
            ChessMove::PawnPromotion(PawnPromotionChessMove::new(from, to, capture, promote_to))
        }
        (Piece::Pawn, None) if to == board.peek_en_passant_target() => {
            ChessMove::EnPassant(EnPassantChessMove::new(from, to))
        }
        (Piece::King, None) if (from, to) == (E1, G1) || (from, to) == (E8, G8) => {
            ChessMove::Castle(CastleChessMove::castle_kingside(board.turn()))
        }
        (Piece::King, None) if (from, to) == (E1, C1) || (from, to) == (E8, C8) => {
            ChessMove::Castle(CastleChessMove::castle_queenside(board.turn()))
        }
        _ => ChessMove::Standard(StandardChessMove::new(from, to, capture)),
    }
}

fn is_elo_determined(wins: usize, _losses: usize, total_games: usize) -> bool {
    total_games >= GAMES_PER_ELO && (wins as f32 / total_games as f32 - 0.5).abs() < 0.1
}

fn display_progress(
    elo: u32,
    wins: usize,
    losses: usize,
    draws: usize,
    total_games: usize,
    engine_time: Duration,
    stockfish_time: Duration,
) {
    print!("{}{}", clear::All, cursor::Goto(1, 1));
    println!("Determining Stockfish ELO");
    println!("-------------------------");
    println!("Current ELO: {}", elo);
    println!("Wins: {}", wins);
    println!("Losses: {}", losses);
    println!("Draws: {}", draws);
    println!("Total games: {}", total_games);
    println!(
        "Engine avg move time: {:.2}ms",
        engine_time.as_millis() as f32 / total_games as f32
    );
    println!(
        "Stockfish avg move time: {:.2}ms",
        stockfish_time.as_millis() as f32 / total_games as f32
    );
}

enum GameResult {
    Win,
    Loss,
    Draw,
}
