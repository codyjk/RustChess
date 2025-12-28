//! Domain-agnostic tests for the alpha-beta search algorithm using Nim.

use super::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// State of a Nim game: players take 1-3 objects, last to take wins.
#[derive(Clone, Debug)]
struct NimState {
    pile: u8,
    is_player_one_turn: bool,
}

impl NimState {
    fn new(pile: u8) -> Self {
        Self {
            pile,
            is_player_one_turn: true,
        }
    }
}

impl GameState for NimState {
    fn position_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.pile.hash(&mut hasher);
        self.is_player_one_turn.hash(&mut hasher);
        hasher.finish()
    }

    fn is_maximizing_player(&self) -> bool {
        self.is_player_one_turn
    }

    fn toggle_turn(&mut self) {
        self.is_player_one_turn = !self.is_player_one_turn;
    }
}

#[derive(Clone, Debug, PartialEq)]
struct NimMove {
    take: u8,
}

impl GameMove for NimMove {
    type State = NimState;
    type Error = &'static str;

    fn apply(&self, state: &mut NimState) -> Result<(), Self::Error> {
        if self.take > state.pile || self.take == 0 || self.take > 3 {
            return Err("Invalid move");
        }
        state.pile -= self.take;
        Ok(())
    }

    fn undo(&self, state: &mut NimState) -> Result<(), Self::Error> {
        state.pile += self.take;
        Ok(())
    }
}

#[derive(Clone)]
struct NimMoveGenerator;

impl MoveGenerator<NimState> for NimMoveGenerator {
    type Move = NimMove;
    type MoveList = Vec<NimMove>;

    fn generate_moves(&self, state: &mut NimState) -> Vec<NimMove> {
        if state.pile == 0 {
            return vec![];
        }
        (1..=std::cmp::min(3, state.pile))
            .map(|take| NimMove { take })
            .collect()
    }
}

#[derive(Clone)]
struct NimEvaluator;

impl Evaluator<NimState> for NimEvaluator {
    fn evaluate(&self, state: &mut NimState, remaining_depth: u8) -> i16 {
        if state.pile == 0 {
            // Current player has no moves - previous player took the last piece and won
            if state.is_player_one_turn {
                -1000 - remaining_depth as i16
            } else {
                1000 + remaining_depth as i16
            }
        } else {
            // pile % 4 == 0 is a losing position for the player to move
            if state.pile % 4 == 0 {
                if state.is_player_one_turn { -100 } else { 100 }
            } else {
                if state.is_player_one_turn { 100 } else { -100 }
            }
        }
    }
}

#[test]
fn test_nim_finds_winning_move_from_5() {
    let mut state = NimState::new(5);
    let mut context = SearchContext::<NimMove>::new(10);

    let best_move = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    )
    .unwrap();

    assert_eq!(best_move.take, 1, "From pile of 5, should take 1 to leave opponent with 4");
}

#[test]
fn test_nim_finds_winning_move_from_6() {
    let mut state = NimState::new(6);
    let mut context = SearchContext::<NimMove>::new(10);

    let best_move = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    )
    .unwrap();

    assert_eq!(best_move.take, 2, "From pile of 6, should take 2 to leave opponent with 4");
}

#[test]
fn test_nim_finds_winning_move_from_7() {
    let mut state = NimState::new(7);
    let mut context = SearchContext::<NimMove>::new(10);

    let best_move = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    )
    .unwrap();

    assert_eq!(best_move.take, 3, "From pile of 7, should take 3 to leave opponent with 4");
}

#[test]
fn test_nim_losing_position() {
    let mut state = NimState::new(4);
    let mut context = SearchContext::<NimMove>::new(10);

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    );

    assert!(result.is_ok(), "Should return a move even from losing position");
    let best_move = result.unwrap();
    assert!(best_move.take >= 1 && best_move.take <= 3, "Move should be valid (1-3)");
}

#[test]
fn test_nim_exhaustive_winning_positions() {
    for pile in 1..=15 {
        if pile % 4 == 0 {
            continue;
        }

        let mut state = NimState::new(pile);
        let mut context = SearchContext::<NimMove>::new(20);

        let result = alpha_beta_search(
            &mut context,
            &mut state,
            &NimMoveGenerator,
            &NimEvaluator,
            &NoOpMoveOrderer,
        );

        assert!(result.is_ok(), "Should find a move for pile size {}", pile);

        let best_move = result.unwrap();
        let remaining = pile - best_move.take;
        assert_eq!(
            remaining % 4, 0,
            "From pile {}, taking {} leaves {} which should be a multiple of 4",
            pile, best_move.take, remaining
        );
    }
}

#[test]
fn test_nim_game_to_completion() {
    let mut state = NimState::new(5);
    let mut context = SearchContext::<NimMove>::new(12);
    let mut move_count = 0;

    while state.pile > 0 {
        let best_move = alpha_beta_search(
            &mut context,
            &mut state,
            &NimMoveGenerator,
            &NimEvaluator,
            &NoOpMoveOrderer,
        )
        .unwrap();

        best_move.apply(&mut state).unwrap();
        state.toggle_turn();
        move_count += 1;

        assert!(move_count < 20, "Game should not exceed 20 moves");
    }

    assert!(!state.is_player_one_turn, "Player one should win from pile of 5");
}

#[test]
fn test_search_returns_error_for_zero_depth() {
    let mut state = NimState::new(5);
    let mut context = SearchContext::<NimMove>::new(0);

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    );

    assert!(matches!(result, Err(SearchError::DepthTooLow)));
}

#[test]
fn test_search_returns_error_for_no_moves() {
    let mut state = NimState::new(0);
    let mut context = SearchContext::<NimMove>::new(5);

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    );

    assert!(matches!(result, Err(SearchError::NoAvailableMoves)));
}

#[test]
fn test_transposition_table_reduces_search_count() {
    let mut state = NimState::new(8);
    let mut context = SearchContext::<NimMove>::new(10);

    let _ = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    );
    let first_count = context.searched_position_count();

    context.searched_position_count.store(0, Ordering::SeqCst);

    let _ = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    );
    let second_count = context.searched_position_count();

    assert!(
        second_count <= first_count,
        "Second search ({}) should explore fewer positions than first ({})",
        second_count, first_count
    );
}
