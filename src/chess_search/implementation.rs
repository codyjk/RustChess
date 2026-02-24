//! Chess-specific trait implementations for the alpha-beta search.

use crate::alpha_beta_searcher::{
    alpha_beta_search, Evaluator, GameMove, GameState, MoveCollection, MoveGenerator,
    SearchContext, SearchError,
};
use crate::board::{error::BoardError, Board};
use crate::chess_move::chess_move::ChessMove;
use crate::move_generator::{ChessMoveList, MoveGenerator as ChessMoveGen};
use crate::{evaluate, move_generator};
#[cfg(feature = "instrumentation")]
use tracing::instrument;

use super::move_orderer::{clear_history, ChessMoveOrderer};

impl GameState for Board {
    #[inline]
    fn position_hash(&self) -> u64 {
        self.current_position_hash()
    }

    #[inline]
    fn is_maximizing_player(&self) -> bool {
        self.turn().maximize_score()
    }

    #[inline]
    fn toggle_turn(&mut self) {
        Board::toggle_turn(self);
    }

    #[inline]
    fn apply_null_move(&mut self) {
        self.push_en_passant_target(None);
        Board::toggle_turn(self);
    }

    #[inline]
    fn undo_null_move(&mut self) {
        Board::toggle_turn(self);
        self.pop_en_passant_target();
    }
}

impl GameMove for ChessMove {
    type State = Board;
    type Error = BoardError;

    #[inline]
    fn apply(&self, state: &mut Board) -> Result<(), BoardError> {
        ChessMove::apply(self, state)
    }

    #[inline]
    fn undo(&self, state: &mut Board) -> Result<(), BoardError> {
        ChessMove::undo(self, state)
    }

    #[inline]
    fn is_tactical(&self, _state: &Board) -> bool {
        self.captures().is_some() || matches!(self, ChessMove::PawnPromotion(_))
    }
}

impl MoveCollection<ChessMove> for ChessMoveList {
    fn is_empty(&self) -> bool {
        ChessMoveList::is_empty(self)
    }

    fn len(&self) -> usize {
        ChessMoveList::len(self)
    }
}

#[derive(Clone, Default)]
pub struct ChessMoveGenerator {
    inner: ChessMoveGen,
}

impl ChessMoveGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inner(&self) -> &ChessMoveGen {
        &self.inner
    }
}

impl MoveGenerator<Board> for ChessMoveGenerator {
    type Move = ChessMove;
    type MoveList = ChessMoveList;

    #[inline]
    fn generate_moves(&self, state: &mut Board) -> ChessMoveList {
        self.inner.generate_moves(state, state.turn())
    }
}

#[derive(Clone, Default)]
pub struct ChessEvaluator {
    move_generator: move_generator::MoveGenerator,
}

impl ChessEvaluator {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Evaluator<Board> for ChessEvaluator {
    #[inline]
    #[cfg_attr(feature = "instrumentation", instrument(skip_all))]
    fn evaluate(&self, state: &mut Board, remaining_depth: u8) -> i16 {
        evaluate::score(state, &self.move_generator, state.turn(), remaining_depth)
    }

    #[inline]
    fn max_tactical_gain(&self, _state: &Board) -> i16 {
        // Maximum gain is capturing a queen (900) plus a margin for positional gains
        // This is a conservative upper bound for delta pruning in quiescence
        900 + 200
    }

    #[inline]
    fn should_skip_null_move(&self, state: &mut Board) -> bool {
        evaluate::current_player_is_in_check(state, &self.move_generator)
            || evaluate::is_endgame(state)
    }

    #[inline]
    fn rfp_margin(&self, depth: u8) -> Option<i16> {
        match depth {
            1 => Some(200),
            2 => Some(500),
            3 => Some(900),
            _ => None,
        }
    }
}

/// Searches for the best chess move from the given position.
#[must_use = "search returns the best move found"]
pub fn search_best_move(
    context: &mut SearchContext<ChessMove>,
    board: &mut Board,
) -> Result<ChessMove, SearchError> {
    // Clear history at start of each search to prevent unbounded growth
    clear_history();

    let move_generator = ChessMoveGenerator::default();
    let evaluator = ChessEvaluator::default();
    let move_orderer = ChessMoveOrderer;

    alpha_beta_search(context, board, &move_generator, &evaluator, &move_orderer)
}
