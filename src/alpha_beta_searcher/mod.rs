use crate::board::Board;
use crate::chess_move::chess_move::ChessMove;
use crate::evaluate;
use crate::move_generator::MoveGenerator;
use log::debug;
use thiserror::Error;

use rayon::prelude::*;
use std::cmp::{max, min};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use self::prioritize_chess_moves::sort_chess_moves;
use self::transposition_table::{BoundType, TranspositionTable};

mod prioritize_chess_moves;
mod transposition_table;

pub struct SearchContext {
    search_depth: u8,
    searched_position_count: AtomicUsize,
    last_score: Option<i16>,
    last_search_duration: Option<Duration>,
    transposition_table: TranspositionTable,
}

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("no available moves")]
    NoAvailableMoves,
    #[error("depth must be at least 1")]
    DepthTooLow,
}

impl SearchContext {
    pub fn new(depth: u8) -> Self {
        Self {
            search_depth: depth,
            searched_position_count: AtomicUsize::new(0),
            last_score: None,
            last_search_duration: None,
            transposition_table: TranspositionTable::default(),
        }
    }

    pub fn reset_stats(&mut self) {
        self.last_score = None;
        self.last_search_duration = None;
        self.searched_position_count.store(0, Ordering::SeqCst);
        self.transposition_table.clear();
    }

    pub fn searched_position_count(&self) -> usize {
        self.searched_position_count.load(Ordering::SeqCst)
    }

    pub fn search_depth(&self) -> u8 {
        self.search_depth
    }

    pub fn last_score(&self) -> Option<i16> {
        self.last_score
    }

    pub fn last_search_duration(&self) -> Option<Duration> {
        self.last_search_duration
    }

    fn increment_position_count(&self) {
        self.searched_position_count.fetch_add(1, Ordering::SeqCst);
    }
}

pub fn alpha_beta_search(
    context: &mut SearchContext,
    board: &mut Board,
    move_generator: &mut MoveGenerator,
) -> Result<ChessMove, SearchError> {
    debug!("alpha-beta search depth: {}", context.search_depth());
    let depth = context.search_depth();

    if depth < 1 {
        return Err(SearchError::DepthTooLow);
    }

    let start = Instant::now();
    let current_player = board.turn();
    let current_player_is_maximizing = current_player.maximize_score();
    let mut candidates =
        move_generator.generate_moves_and_lazily_update_chess_move_effects(board, current_player);

    sort_chess_moves(&mut candidates, board);

    // First try the transposition table
    let hash = board.current_position_hash();
    if let Some((score, best_move)) =
        context
            .transposition_table
            .probe(hash, depth, i16::MIN, i16::MAX)
    {
        if let Some(mv) = best_move {
            if candidates.iter().any(|c| *c == mv) {
                debug!("Using transposition table hit");
                context.last_score = Some(score);
                context.last_search_duration = Some(start.elapsed());
                return Ok(mv);
            }
        }
    }

    // Score moves in parallel
    let scored_moves = candidates.par_iter().map(|chess_move| {
        let mut local_board = board.clone();
        let mut local_move_generator = MoveGenerator::default();

        chess_move.apply(&mut local_board).unwrap();
        local_board.toggle_turn();

        let score = alpha_beta_minimax(
            context,
            &mut local_board,
            &mut local_move_generator,
            depth - 1,
            i16::MIN,
            i16::MAX,
            !current_player_is_maximizing,
        )
        .unwrap();

        (score, chess_move.clone())
    });

    // Sort the best move to the end so we can pop it off
    let mut scored_moves = scored_moves.collect::<Vec<_>>();
    scored_moves.sort_by(|(a, _), (b, _)| b.cmp(a));
    if current_player_is_maximizing {
        scored_moves.reverse();
    }

    let (score, best_move) = scored_moves.pop().ok_or(SearchError::NoAvailableMoves)?;

    // Store result in transposition table
    context.transposition_table.store(
        hash,
        score,
        depth,
        BoundType::Exact,
        Some(best_move.clone()),
    );

    context.last_score = Some(score);
    context.last_search_duration = Some(start.elapsed());

    Ok(best_move)
}

fn alpha_beta_minimax(
    context: &SearchContext,
    board: &mut Board,
    move_generator: &mut MoveGenerator,
    depth: u8,
    mut alpha: i16,
    mut beta: i16,
    maximizing_player: bool,
) -> Result<i16, SearchError> {
    context.increment_position_count();

    let hash = board.current_position_hash();

    // Check transposition table
    if let Some((score, _)) = context.transposition_table.probe(hash, depth, alpha, beta) {
        return Ok(score);
    }

    if depth == 0 {
        let score = evaluate::score(board, move_generator, board.turn(), depth);
        return Ok(score);
    }

    let mut candidates =
        move_generator.generate_moves_and_lazily_update_chess_move_effects(board, board.turn());

    if candidates.is_empty() {
        let score = evaluate::score(board, move_generator, board.turn(), depth);
        return Ok(score);
    }

    sort_chess_moves(&mut candidates, board);

    let mut best_move = None;
    let mut best_score = if maximizing_player {
        i16::MIN
    } else {
        i16::MAX
    };
    let original_alpha = alpha;

    for chess_move in candidates {
        chess_move.apply(board).unwrap();
        board.toggle_turn();

        let score = alpha_beta_minimax(
            context,
            board,
            move_generator,
            depth - 1,
            alpha,
            beta,
            !maximizing_player,
        )?;

        chess_move.undo(board).unwrap();
        board.toggle_turn();

        if maximizing_player {
            if score > best_score {
                best_score = score;
                best_move = Some(chess_move);
            }
            alpha = max(alpha, score);
        } else {
            if score < best_score {
                best_score = score;
                best_move = Some(chess_move);
            }
            beta = min(beta, score);
        }

        if beta <= alpha {
            break;
        }
    }

    // Store position in transposition table
    let bound_type = if best_score <= original_alpha {
        BoundType::Upper
    } else if best_score >= beta {
        BoundType::Lower
    } else {
        BoundType::Exact
    };

    context
        .transposition_table
        .store(hash, best_score, depth, bound_type, best_move);

    Ok(best_score)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::castle_rights_bitmask::ALL_CASTLE_RIGHTS;
    use crate::board::color::Color;
    use crate::board::piece::Piece;
    use crate::chess_move::capture::Capture;
    use crate::chess_move::chess_move_effect::ChessMoveEffect;
    use crate::chess_move::standard::StandardChessMove;
    use crate::{check_move, checkmate_move, chess_position, std_move};
    use common::bitboard::bitboard::Bitboard;
    use common::bitboard::square::*;

    #[test]
    fn test_find_mate_in_1_white() {
        let mut search_context = SearchContext::new(4);
        let mut move_generator = MoveGenerator::default();

        let mut board = chess_position! {
            .Q......
            ........
            ........
            ........
            ........
            ........
            k.K.....
            ........
        };
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);
        println!("Testing board:\n{}", board);

        let chess_move =
            alpha_beta_search(&mut search_context, &mut board, &mut move_generator).unwrap();
        let valid_checkmates = [
            checkmate_move!(std_move!(B8, B2)),
            checkmate_move!(std_move!(B8, A8)),
            checkmate_move!(std_move!(B8, A7)),
        ];
        assert!(
            valid_checkmates.contains(&chess_move),
            "{} does not lead to checkmate",
            chess_move
        );
    }

    #[test]
    fn test_find_mate_in_1_black() {
        let mut search_context = SearchContext::new(4);
        let mut move_generator = MoveGenerator::default();
        let mut board = chess_position! {
            .q......
            ........
            ........
            ........
            ........
            ........
            K.k.....
            ........
        };
        board.set_turn(Color::Black);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        println!("Testing board:\n{}", board);

        let chess_move =
            alpha_beta_search(&mut search_context, &mut board, &mut move_generator).unwrap();

        let valid_checkmates = [
            checkmate_move!(std_move!(B8, B2)),
            checkmate_move!(std_move!(B8, A8)),
            checkmate_move!(std_move!(B8, A7)),
        ];
        assert!(
            valid_checkmates.contains(&chess_move),
            "{} does not lead to checkmate",
            chess_move
        );
    }

    #[test]
    fn test_find_back_rank_mate_in_2_white() {
        let mut search_context = SearchContext::new(4);
        let mut move_generator = MoveGenerator::default();

        let mut board = chess_position! {
            .k.....r
            ppp.....
            ........
            ........
            ........
            ........
            ...Q....
            K..R....
        };
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        println!("Testing board:\n{}", board);

        let expected_moves = [
            check_move!(std_move!(D2, D8)),
            std_move!(H8, D8, Capture(Piece::Queen)),
            checkmate_move!(std_move!(D1, D8, Capture(Piece::Rook))),
        ];
        let mut expected_move_iter = expected_moves.iter();

        let move1 =
            alpha_beta_search(&mut search_context, &mut board, &mut move_generator).unwrap();
        move1.apply(&mut board).unwrap();
        board.toggle_turn();
        assert_eq!(expected_move_iter.next().unwrap(), &move1);
        println!("Testing board:\n{}", board);

        let move2 =
            alpha_beta_search(&mut search_context, &mut board, &mut move_generator).unwrap();
        move2.apply(&mut board).unwrap();
        board.toggle_turn();
        assert_eq!(expected_move_iter.next().unwrap(), &move2);
        println!("Testing board:\n{}", board);

        let move3 =
            alpha_beta_search(&mut search_context, &mut board, &mut move_generator).unwrap();
        move3.apply(&mut board).unwrap();
        board.toggle_turn();
        assert_eq!(expected_move_iter.next().unwrap(), &move3);
        println!("Testing board:\n{}", board);
    }

    #[test]
    fn test_find_back_rank_mate_in_2_black() {
        let mut search_context = SearchContext::new(4);
        let mut move_generator = MoveGenerator::default();

        let mut board = chess_position! {
            ....r..k
            ....q...
            ........
            ........
            ........
            ........
            .....PPP
            R.....K.
        };
        board.set_turn(Color::Black);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        println!("Testing board:\n{}", board);

        let expected_moves = [
            check_move!(std_move!(E7, E1)),
            std_move!(A1, E1, Capture(Piece::Queen)),
            checkmate_move!(std_move!(E8, E1, Capture(Piece::Rook))),
        ];
        let mut expected_move_iter = expected_moves.iter();

        let move1 =
            alpha_beta_search(&mut search_context, &mut board, &mut move_generator).unwrap();
        move1.apply(&mut board).unwrap();
        board.toggle_turn();
        assert_eq!(
            expected_move_iter.next().unwrap(),
            &move1,
            "failed to find first move of mate in 2"
        );
        println!("Testing board:\n{}", board);

        let move2 =
            alpha_beta_search(&mut search_context, &mut board, &mut move_generator).unwrap();
        move2.apply(&mut board).unwrap();
        board.toggle_turn();
        assert_eq!(
            expected_move_iter.next().unwrap(),
            &move2,
            "failed to find second move of mate in 2"
        );
        println!("Testing board:\n{}", board);

        let move3 =
            alpha_beta_search(&mut search_context, &mut board, &mut move_generator).unwrap();
        move3.apply(&mut board).unwrap();
        board.toggle_turn();
        assert_eq!(
            expected_move_iter.next().unwrap(),
            &move3,
            "failed to find third move of mate in 2"
        );
        println!("Testing board:\n{}", board);
    }
}
