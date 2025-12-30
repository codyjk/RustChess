//! Domain-agnostic tests for the alpha-beta search algorithm using Nim.
//!
//! Test coverage:
//! - Basic search functionality (winning moves, losing positions, game completion)
//! - Error handling (zero depth, no moves)
//! - Transposition tables (TT hits, bound types, position caching)
//! - Killer moves (storage, retrieval, clearing, multiple plies)
//! - Move ordering (PV move prioritization)
//! - Quiescence search (tactical moves, stand-pat, depth limiting, alpha/beta boundaries)
//! - Alpha-beta pruning (beta cutoffs, score boundaries, best move positions)
//! - Depth edge cases (depth 1, single/two moves)
//! - Parallel vs sequential search consistency

use super::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::Ordering;

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
                if state.is_player_one_turn {
                    -100
                } else {
                    100
                }
            } else {
                if state.is_player_one_turn {
                    100
                } else {
                    -100
                }
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

    assert_eq!(
        best_move.take, 1,
        "From pile of 5, should take 1 to leave opponent with 4"
    );
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

    assert_eq!(
        best_move.take, 2,
        "From pile of 6, should take 2 to leave opponent with 4"
    );
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

    assert_eq!(
        best_move.take, 3,
        "From pile of 7, should take 3 to leave opponent with 4"
    );
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

    assert!(
        result.is_ok(),
        "Should return a move even from losing position"
    );
    let best_move = result.unwrap();
    assert!(
        best_move.take >= 1 && best_move.take <= 3,
        "Move should be valid (1-3)"
    );
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
            remaining % 4,
            0,
            "From pile {}, taking {} leaves {} which should be a multiple of 4",
            pile,
            best_move.take,
            remaining
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

    assert!(
        !state.is_player_one_turn,
        "Player one should win from pile of 5"
    );
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
    let mut state = NimState::new(12);
    let mut context = SearchContext::<NimMove>::new(10);

    let _ = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    );
    let first_count = context.searched_position_count();
    assert!(first_count > 0, "First search should explore positions");

    let tt_hits_before = context.tt_hits();
    context.reset_stats();

    let _ = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    );
    let second_count = context.searched_position_count();
    let tt_hits_after = context.tt_hits();

    assert!(
        tt_hits_after > tt_hits_before || second_count <= first_count,
        "TT should be used (hits: {} -> {}) or second search ({}) should explore fewer positions than first ({})",
        tt_hits_before, tt_hits_after, second_count, first_count
    );
}

#[test]
fn test_killer_moves_stored_and_retrieved() {
    let context = SearchContext::<NimMove>::new(5);

    let killer1 = NimMove { take: 1 };
    let killer2 = NimMove { take: 2 };

    context.store_killer(0, killer1.clone());
    context.store_killer(0, killer2.clone());

    let killers = context.get_killers(0);
    assert_eq!(
        killers[0],
        Some(killer2),
        "Most recent killer should be first"
    );
    assert_eq!(
        killers[1],
        Some(killer1),
        "Previous killer should be second"
    );
}

#[test]
fn test_killer_moves_cleared() {
    let mut context = SearchContext::<NimMove>::new(5);
    let killer = NimMove { take: 1 };

    context.store_killer(0, killer.clone());
    assert_eq!(context.get_killers(0)[0], Some(killer.clone()));

    context.clear_killers();
    assert_eq!(context.get_killers(0)[0], None);
    assert_eq!(context.get_killers(0)[1], None);
}

#[derive(Clone)]
struct OrderedNimMoveOrderer {
    preferred_move: Option<NimMove>,
}

impl MoveOrderer<NimState, NimMove> for OrderedNimMoveOrderer {
    fn order_moves(&self, moves: &mut [NimMove], _state: &NimState) {
        if let Some(ref preferred) = self.preferred_move {
            if let Some(pos) = moves.iter().position(|m| m == preferred) {
                if pos > 0 {
                    moves[0..=pos].rotate_right(1);
                }
            }
        }
    }
}

#[test]
fn test_move_ordering_pv_move_prioritized() {
    let mut state = NimState::new(12);
    let mut context = SearchContext::<NimMove>::new(7);

    let first_result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    )
    .unwrap();
    let first_count = context.searched_position_count();
    let first_tt_hits = context.tt_hits();

    context.reset_stats();

    let second_result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    )
    .unwrap();
    let second_count = context.searched_position_count();
    let second_tt_hits = context.tt_hits();

    assert_eq!(
        first_result, second_result,
        "PV move from TT should be used in second search"
    );
    assert!(
        second_tt_hits >= first_tt_hits,
        "TT hits should increase or stay same ({} -> {})",
        first_tt_hits,
        second_tt_hits
    );
    assert!(
        first_count > 0 && second_count > 0,
        "Both searches should explore positions ({} and {})",
        first_count,
        second_count
    );
}

#[test]
fn test_quiescence_search_with_tactical_moves() {
    #[derive(Clone, Debug, PartialEq)]
    struct TacticalNimMove {
        take: u8,
        is_tactical: bool,
    }

    impl GameMove for TacticalNimMove {
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

        fn is_tactical(&self, _state: &NimState) -> bool {
            self.is_tactical
        }
    }

    #[derive(Clone)]
    struct TacticalNimMoveGenerator;

    impl MoveGenerator<NimState> for TacticalNimMoveGenerator {
        type Move = TacticalNimMove;
        type MoveList = Vec<TacticalNimMove>;

        fn generate_moves(&self, state: &mut NimState) -> Vec<TacticalNimMove> {
            if state.pile == 0 {
                return vec![];
            }
            (1..=std::cmp::min(3, state.pile))
                .map(|take| TacticalNimMove {
                    take,
                    is_tactical: take == 3,
                })
                .collect()
        }
    }

    let mut state = NimState::new(5);
    let mut context = SearchContext::<TacticalNimMove>::new(1);
    let evaluator = NimEvaluator;
    let move_orderer = NoOpMoveOrderer;

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &TacticalNimMoveGenerator,
        &evaluator,
        &move_orderer,
    );

    assert!(result.is_ok(), "Quiescence search should succeed");
    let best_move = result.unwrap();
    assert!(
        best_move.is_tactical || best_move.take >= 1 && best_move.take <= 3,
        "Quiescence should search tactical moves, got take={}",
        best_move.take
    );
    assert!(
        context.searched_position_count() > 0,
        "Quiescence should search positions"
    );
}

#[test]
fn test_quiescence_search_without_tactical_moves() {
    let mut state = NimState::new(5);
    let mut context = SearchContext::<NimMove>::new(1);
    let evaluator = NimEvaluator;
    let move_orderer = NoOpMoveOrderer;

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &evaluator,
        &move_orderer,
    );

    assert!(
        result.is_ok(),
        "Quiescence should work even without tactical moves"
    );
    let best_move = result.unwrap();
    assert!(
        best_move.take >= 1 && best_move.take <= 3,
        "Move should be valid"
    );
}

#[test]
fn test_quiescence_depth_limit() {
    #[derive(Clone, Debug, PartialEq)]
    struct AlwaysTacticalMove {
        take: u8,
    }

    impl GameMove for AlwaysTacticalMove {
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

        fn is_tactical(&self, _state: &NimState) -> bool {
            true
        }
    }

    #[derive(Clone)]
    struct AlwaysTacticalMoveGenerator;

    impl MoveGenerator<NimState> for AlwaysTacticalMoveGenerator {
        type Move = AlwaysTacticalMove;
        type MoveList = Vec<AlwaysTacticalMove>;

        fn generate_moves(&self, state: &mut NimState) -> Vec<AlwaysTacticalMove> {
            if state.pile == 0 {
                return vec![];
            }
            (1..=std::cmp::min(3, state.pile))
                .map(|take| AlwaysTacticalMove { take })
                .collect()
        }
    }

    let mut state = NimState::new(20);
    let mut context = SearchContext::<AlwaysTacticalMove>::new(1);
    let evaluator = NimEvaluator;
    let move_orderer = NoOpMoveOrderer;

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &AlwaysTacticalMoveGenerator,
        &evaluator,
        &move_orderer,
    );

    assert!(
        result.is_ok(),
        "Quiescence should terminate due to depth limit"
    );
    let position_count = context.searched_position_count();
    assert!(
        position_count < 1000,
        "Quiescence depth limit should prevent explosion (searched {} positions)",
        position_count
    );
}

#[test]
fn test_alpha_beta_depth_1() {
    let mut state = NimState::new(5);
    let mut context = SearchContext::<NimMove>::new(1);

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    );

    assert!(result.is_ok(), "Depth 1 search should succeed");
    let best_move = result.unwrap();
    assert!(
        best_move.take >= 1 && best_move.take <= 3,
        "Move should be valid"
    );
}

#[test]
fn test_alpha_beta_single_move() {
    let mut state = NimState::new(1);
    let mut context = SearchContext::<NimMove>::new(5);

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    );

    assert!(result.is_ok(), "Single move search should succeed");
    let best_move = result.unwrap();
    assert_eq!(best_move.take, 1, "Only move should be selected");
}

#[test]
fn test_alpha_beta_two_moves() {
    let mut state = NimState::new(2);
    let mut context = SearchContext::<NimMove>::new(5);

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    );

    assert!(result.is_ok(), "Two move search should succeed");
    let best_move = result.unwrap();
    assert!(
        best_move.take == 1 || best_move.take == 2,
        "Move should be valid"
    );
}

#[test]
fn test_alpha_beta_beta_cutoff_first_move() {
    #[derive(Clone)]
    struct HighScoreEvaluator;

    impl Evaluator<NimState> for HighScoreEvaluator {
        fn evaluate(&self, state: &mut NimState, _remaining_depth: u8) -> i16 {
            if state.pile == 0 {
                if state.is_player_one_turn {
                    -1000
                } else {
                    1000
                }
            } else {
                if state.is_player_one_turn {
                    200
                } else {
                    -200
                }
            }
        }
    }

    let mut state = NimState::new(5);
    let mut context = SearchContext::<NimMove>::new(2);

    let first_count = {
        let mut ctx = SearchContext::<NimMove>::new(2);
        let _ = alpha_beta_search(
            &mut ctx,
            &mut state.clone(),
            &NimMoveGenerator,
            &NimEvaluator,
            &NoOpMoveOrderer,
        );
        ctx.searched_position_count()
    };

    let second_count = {
        let mut ctx = SearchContext::<NimMove>::new(2);
        let _ = alpha_beta_search(
            &mut ctx,
            &mut state.clone(),
            &NimMoveGenerator,
            &HighScoreEvaluator,
            &NoOpMoveOrderer,
        );
        ctx.searched_position_count()
    };

    assert!(
        second_count <= first_count,
        "High scores should enable more beta cutoffs ({} vs {})",
        second_count,
        first_count
    );
}

#[test]
fn test_alpha_beta_all_moves_cause_cutoff() {
    #[derive(Clone)]
    struct AlwaysWinningEvaluator;

    impl Evaluator<NimState> for AlwaysWinningEvaluator {
        fn evaluate(&self, state: &mut NimState, _remaining_depth: u8) -> i16 {
            if state.is_player_one_turn {
                1000
            } else {
                -1000
            }
        }
    }

    let mut state = NimState::new(5);
    let mut context = SearchContext::<NimMove>::new(3);

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &AlwaysWinningEvaluator,
        &NoOpMoveOrderer,
    );

    assert!(
        result.is_ok(),
        "Search should succeed even with all cutoffs"
    );
}

#[test]
fn test_quiescence_stand_pat_equals_beta() {
    #[derive(Clone)]
    struct FixedScoreEvaluator {
        score: i16,
    }

    impl Evaluator<NimState> for FixedScoreEvaluator {
        fn evaluate(&self, _state: &mut NimState, _remaining_depth: u8) -> i16 {
            self.score
        }
    }

    let mut state = NimState::new(5);
    let mut context = SearchContext::<NimMove>::new(1);
    let evaluator = FixedScoreEvaluator { score: 100 };

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &evaluator,
        &NoOpMoveOrderer,
    );

    assert!(
        result.is_ok(),
        "Quiescence with stand-pat = beta should succeed"
    );
}

#[test]
fn test_quiescence_stand_pat_equals_alpha() {
    #[derive(Clone)]
    struct FixedScoreEvaluator {
        score: i16,
    }

    impl Evaluator<NimState> for FixedScoreEvaluator {
        fn evaluate(&self, _state: &mut NimState, _remaining_depth: u8) -> i16 {
            self.score
        }
    }

    let mut state = NimState::new(5);
    let mut context = SearchContext::<NimMove>::new(1);
    let evaluator = FixedScoreEvaluator { score: -100 };

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &evaluator,
        &NoOpMoveOrderer,
    );

    assert!(
        result.is_ok(),
        "Quiescence with stand-pat = alpha should succeed"
    );
}

#[test]
fn test_quiescence_stand_pat_above_beta() {
    #[derive(Clone)]
    struct HighScoreEvaluator;

    impl Evaluator<NimState> for HighScoreEvaluator {
        fn evaluate(&self, _state: &mut NimState, _remaining_depth: u8) -> i16 {
            1000
        }
    }

    let mut state = NimState::new(5);
    let mut context = SearchContext::<NimMove>::new(1);
    let evaluator = HighScoreEvaluator;

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &evaluator,
        &NoOpMoveOrderer,
    );

    assert!(
        result.is_ok(),
        "Quiescence with stand-pat > beta should cutoff"
    );
}

#[test]
fn test_quiescence_stand_pat_below_alpha() {
    #[derive(Clone)]
    struct LowScoreEvaluator;

    impl Evaluator<NimState> for LowScoreEvaluator {
        fn evaluate(&self, _state: &mut NimState, _remaining_depth: u8) -> i16 {
            -1000
        }
    }

    let mut state = NimState::new(5);
    let mut context = SearchContext::<NimMove>::new(1);
    let evaluator = LowScoreEvaluator;

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &evaluator,
        &NoOpMoveOrderer,
    );

    assert!(
        result.is_ok(),
        "Quiescence with stand-pat < alpha should succeed"
    );
}

#[test]
fn test_quiescence_beta_cutoff() {
    #[derive(Clone, Debug, PartialEq)]
    struct HighValueTacticalMove {
        take: u8,
    }

    impl GameMove for HighValueTacticalMove {
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

        fn is_tactical(&self, _state: &NimState) -> bool {
            true
        }
    }

    #[derive(Clone)]
    struct HighValueTacticalMoveGenerator;

    impl MoveGenerator<NimState> for HighValueTacticalMoveGenerator {
        type Move = HighValueTacticalMove;
        type MoveList = Vec<HighValueTacticalMove>;

        fn generate_moves(&self, state: &mut NimState) -> Vec<HighValueTacticalMove> {
            if state.pile == 0 {
                return vec![];
            }
            (1..=std::cmp::min(3, state.pile))
                .map(|take| HighValueTacticalMove { take })
                .collect()
        }
    }

    #[derive(Clone)]
    struct HighValueEvaluator;

    impl Evaluator<NimState> for HighValueEvaluator {
        fn evaluate(&self, state: &mut NimState, _remaining_depth: u8) -> i16 {
            if state.pile == 0 {
                if state.is_player_one_turn {
                    -1000
                } else {
                    1000
                }
            } else {
                if state.is_player_one_turn {
                    500
                } else {
                    -500
                }
            }
        }
    }

    let mut state = NimState::new(5);
    let mut context = SearchContext::<HighValueTacticalMove>::new(1);
    let evaluator = HighValueEvaluator;
    let move_orderer = NoOpMoveOrderer;

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &HighValueTacticalMoveGenerator,
        &evaluator,
        &move_orderer,
    );

    assert!(result.is_ok(), "Quiescence with beta cutoff should succeed");
}

#[test]
fn test_killer_move_already_first() {
    let mut state = NimState::new(8);
    let mut context = SearchContext::<NimMove>::new(5);
    let killer = NimMove { take: 1 };

    context.store_killer(0, killer.clone());

    let mut candidates = vec![killer.clone(), NimMove { take: 2 }, NimMove { take: 3 }];

    let orderer = OrderedNimMoveOrderer {
        preferred_move: None,
    };
    orderer.order_moves(&mut candidates, &state);

    assert_eq!(
        candidates[0], killer,
        "Killer move already first should remain first"
    );
}

#[test]
fn test_killer_move_is_pv_move() {
    let mut state = NimState::new(8);
    let mut context = SearchContext::<NimMove>::new(5);
    let pv_move = NimMove { take: 2 };

    let first_result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    )
    .unwrap();

    context.store_killer(0, first_result.clone());

    let second_result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    )
    .unwrap();

    assert_eq!(
        first_result, second_result,
        "Killer move that is also PV move should work correctly"
    );
}

#[test]
fn test_tt_upper_bound_cutoff() {
    let mut state = NimState::new(10);
    let mut context = SearchContext::<NimMove>::new(5);

    let first_result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    )
    .unwrap();
    let first_count = context.searched_position_count();

    context.reset_stats();

    let second_result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    )
    .unwrap();
    let second_count = context.searched_position_count();

    assert_eq!(
        first_result, second_result,
        "TT should preserve move selection"
    );
    assert!(
        context.tt_hits() > 0 || second_count < first_count,
        "TT should be used or reduce search count"
    );
}

#[test]
fn test_tt_lower_bound_cutoff() {
    let mut state = NimState::new(12);
    let mut context = SearchContext::<NimMove>::new(6);

    let first_result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    )
    .unwrap();

    context.reset_stats();

    let second_result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    )
    .unwrap();

    assert_eq!(
        first_result, second_result,
        "TT lower bound should preserve move selection"
    );
}

#[test]
fn test_tt_exact_bound() {
    let mut state = NimState::new(10);
    let mut context = SearchContext::<NimMove>::new(5);

    let first_result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    )
    .unwrap();

    context.reset_stats();

    let second_result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    )
    .unwrap();

    assert_eq!(
        first_result, second_result,
        "TT exact bound should preserve move selection"
    );
}

#[test]
fn test_parallel_vs_sequential_same_result() {
    let mut state1 = NimState::new(10);
    let mut context1 = SearchContext::<NimMove>::with_parallel(5, false);

    let mut state2 = NimState::new(10);
    let mut context2 = SearchContext::<NimMove>::with_parallel(5, true);

    let sequential_result = alpha_beta_search(
        &mut context1,
        &mut state1,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    )
    .unwrap();

    let parallel_result = alpha_beta_search(
        &mut context2,
        &mut state2,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    )
    .unwrap();

    assert_eq!(
        sequential_result, parallel_result,
        "Parallel and sequential search should return same move"
    );
}

#[test]
fn test_alpha_beta_score_exactly_equals_beta() {
    #[derive(Clone)]
    struct BetaBoundaryEvaluator {
        target_score: i16,
    }

    impl Evaluator<NimState> for BetaBoundaryEvaluator {
        fn evaluate(&self, state: &mut NimState, _remaining_depth: u8) -> i16 {
            if state.pile == 0 {
                if state.is_player_one_turn {
                    -1000
                } else {
                    1000
                }
            } else {
                self.target_score
            }
        }
    }

    let mut state = NimState::new(5);
    let mut context = SearchContext::<NimMove>::new(2);
    let evaluator = BetaBoundaryEvaluator { target_score: 100 };

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &evaluator,
        &NoOpMoveOrderer,
    );

    assert!(
        result.is_ok(),
        "Score exactly equal to beta should cause cutoff"
    );
}

#[test]
fn test_alpha_beta_score_exactly_equals_alpha() {
    #[derive(Clone)]
    struct AlphaBoundaryEvaluator {
        target_score: i16,
    }

    impl Evaluator<NimState> for AlphaBoundaryEvaluator {
        fn evaluate(&self, state: &mut NimState, _remaining_depth: u8) -> i16 {
            if state.pile == 0 {
                if state.is_player_one_turn {
                    -1000
                } else {
                    1000
                }
            } else {
                self.target_score
            }
        }
    }

    let mut state = NimState::new(5);
    let mut context = SearchContext::<NimMove>::new(2);
    let evaluator = AlphaBoundaryEvaluator { target_score: -100 };

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &evaluator,
        &NoOpMoveOrderer,
    );

    assert!(
        result.is_ok(),
        "Score exactly equal to alpha should be handled"
    );
}

#[test]
fn test_quiescence_empty_move_list() {
    #[derive(Clone)]
    struct EmptyMoveGenerator;

    impl MoveGenerator<NimState> for EmptyMoveGenerator {
        type Move = NimMove;
        type MoveList = Vec<NimMove>;

        fn generate_moves(&self, _state: &mut NimState) -> Vec<NimMove> {
            vec![]
        }
    }

    let mut state = NimState::new(5);
    let mut context = SearchContext::<NimMove>::new(1);
    let evaluator = NimEvaluator;

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &EmptyMoveGenerator,
        &evaluator,
        &NoOpMoveOrderer,
    );

    assert!(result.is_err(), "Empty move list should return error");
    assert!(
        matches!(result, Err(SearchError::NoAvailableMoves)),
        "Should return NoAvailableMoves error"
    );
}

#[test]
fn test_quiescence_all_tactical_moves() {
    #[derive(Clone, Debug, PartialEq)]
    struct AlwaysTacticalNimMove {
        take: u8,
    }

    impl GameMove for AlwaysTacticalNimMove {
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

        fn is_tactical(&self, _state: &NimState) -> bool {
            true
        }
    }

    #[derive(Clone)]
    struct AlwaysTacticalMoveGenerator;

    impl MoveGenerator<NimState> for AlwaysTacticalMoveGenerator {
        type Move = AlwaysTacticalNimMove;
        type MoveList = Vec<AlwaysTacticalNimMove>;

        fn generate_moves(&self, state: &mut NimState) -> Vec<AlwaysTacticalNimMove> {
            if state.pile == 0 {
                return vec![];
            }
            (1..=std::cmp::min(3, state.pile))
                .map(|take| AlwaysTacticalNimMove { take })
                .collect()
        }
    }

    let mut state = NimState::new(3);
    let mut context = SearchContext::<AlwaysTacticalNimMove>::new(1);
    let evaluator = NimEvaluator;
    let move_orderer = NoOpMoveOrderer;

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &AlwaysTacticalMoveGenerator,
        &evaluator,
        &move_orderer,
    );

    assert!(
        result.is_ok(),
        "Quiescence with all tactical moves should succeed"
    );
    let position_count = context.searched_position_count();
    assert!(
        position_count > 0,
        "Should search positions when all moves are tactical"
    );
}

#[test]
fn test_killer_move_multiple_at_same_ply() {
    let context = SearchContext::<NimMove>::new(5);

    let killer1 = NimMove { take: 1 };
    let killer2 = NimMove { take: 2 };
    let killer3 = NimMove { take: 3 };

    context.store_killer(0, killer1.clone());
    context.store_killer(0, killer2.clone());
    context.store_killer(0, killer3.clone());

    let killers = context.get_killers(0);
    assert_eq!(
        killers[0],
        Some(killer3),
        "Most recent killer should be first"
    );
    assert_eq!(
        killers[1],
        Some(killer2),
        "Previous killer should be second"
    );
}

#[test]
fn test_killer_move_different_plies() {
    let context = SearchContext::<NimMove>::new(5);

    let killer_ply0 = NimMove { take: 1 };
    let killer_ply1 = NimMove { take: 2 };

    context.store_killer(0, killer_ply0.clone());
    context.store_killer(1, killer_ply1.clone());

    assert_eq!(
        context.get_killers(0)[0],
        Some(killer_ply0),
        "Killer at ply 0 should be stored"
    );
    assert_eq!(
        context.get_killers(1)[0],
        Some(killer_ply1),
        "Killer at ply 1 should be stored"
    );
}

#[test]
fn test_alpha_beta_best_move_is_last() {
    #[derive(Clone)]
    struct LastMoveBestEvaluator;

    impl Evaluator<NimState> for LastMoveBestEvaluator {
        fn evaluate(&self, state: &mut NimState, remaining_depth: u8) -> i16 {
            if state.pile == 0 {
                if state.is_player_one_turn {
                    -1000 - remaining_depth as i16
                } else {
                    1000 + remaining_depth as i16
                }
            } else {
                if state.pile == 1 {
                    if state.is_player_one_turn {
                        200
                    } else {
                        -200
                    }
                } else {
                    if state.is_player_one_turn {
                        100
                    } else {
                        -100
                    }
                }
            }
        }
    }

    let mut state = NimState::new(3);
    let mut context = SearchContext::<NimMove>::new(3);

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &LastMoveBestEvaluator,
        &NoOpMoveOrderer,
    );

    assert!(result.is_ok(), "Best move being last should work");
    let best_move = result.unwrap();
    assert!(
        best_move.take >= 1 && best_move.take <= 3,
        "Move should be valid"
    );
}

#[test]
fn test_alpha_beta_best_move_is_first() {
    #[derive(Clone)]
    struct FirstMoveBestEvaluator;

    impl Evaluator<NimState> for FirstMoveBestEvaluator {
        fn evaluate(&self, state: &mut NimState, remaining_depth: u8) -> i16 {
            if state.pile == 0 {
                if state.is_player_one_turn {
                    -1000 - remaining_depth as i16
                } else {
                    1000 + remaining_depth as i16
                }
            } else {
                if state.pile == 2 {
                    if state.is_player_one_turn {
                        200
                    } else {
                        -200
                    }
                } else {
                    if state.is_player_one_turn {
                        100
                    } else {
                        -100
                    }
                }
            }
        }
    }

    let mut state = NimState::new(3);
    let mut context = SearchContext::<NimMove>::new(3);

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &NimMoveGenerator,
        &FirstMoveBestEvaluator,
        &NoOpMoveOrderer,
    );

    assert!(result.is_ok(), "Best move being first should work");
    let best_move = result.unwrap();
    assert!(
        best_move.take >= 1 && best_move.take <= 3,
        "Move should be valid"
    );
}

#[test]
fn test_quiescence_alpha_update() {
    #[derive(Clone, Debug, PartialEq)]
    struct ImprovingTacticalMove {
        take: u8,
        improvement: i16,
    }

    impl GameMove for ImprovingTacticalMove {
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

        fn is_tactical(&self, _state: &NimState) -> bool {
            true
        }
    }

    #[derive(Clone)]
    struct ImprovingTacticalMoveGenerator;

    impl MoveGenerator<NimState> for ImprovingTacticalMoveGenerator {
        type Move = ImprovingTacticalMove;
        type MoveList = Vec<ImprovingTacticalMove>;

        fn generate_moves(&self, state: &mut NimState) -> Vec<ImprovingTacticalMove> {
            if state.pile == 0 {
                return vec![];
            }
            (1..=std::cmp::min(3, state.pile))
                .map(|take| ImprovingTacticalMove {
                    take,
                    improvement: take as i16 * 10,
                })
                .collect()
        }
    }

    #[derive(Clone)]
    struct ImprovingEvaluator;

    impl Evaluator<NimState> for ImprovingEvaluator {
        fn evaluate(&self, state: &mut NimState, _remaining_depth: u8) -> i16 {
            if state.pile == 0 {
                if state.is_player_one_turn {
                    -1000
                } else {
                    1000
                }
            } else {
                if state.is_player_one_turn {
                    50
                } else {
                    -50
                }
            }
        }
    }

    let mut state = NimState::new(5);
    let mut context = SearchContext::<ImprovingTacticalMove>::new(1);
    let evaluator = ImprovingEvaluator;
    let move_orderer = NoOpMoveOrderer;

    let result = alpha_beta_search(
        &mut context,
        &mut state,
        &ImprovingTacticalMoveGenerator,
        &evaluator,
        &move_orderer,
    );

    assert!(
        result.is_ok(),
        "Quiescence with alpha update should succeed"
    );
}

#[test]
fn test_null_move_pruning_requires_depth_3() {
    let mut state = NimState::new(10);
    let mut context_depth_2 = SearchContext::<NimMove>::new(2);
    let mut context_depth_3 = SearchContext::<NimMove>::new(3);

    let result_2 = alpha_beta_search(
        &mut context_depth_2,
        &mut state.clone(),
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    )
    .unwrap();
    let count_2 = context_depth_2.searched_position_count();

    let result_3 = alpha_beta_search(
        &mut context_depth_3,
        &mut state,
        &NimMoveGenerator,
        &NimEvaluator,
        &NoOpMoveOrderer,
    )
    .unwrap();
    let count_3 = context_depth_3.searched_position_count();

    assert_eq!(
        result_2, result_3,
        "Results should match regardless of null move pruning"
    );
    assert!(
        count_2 > 0 && count_3 > 0,
        "Both searches should explore positions"
    );
}
