//! Core traits for generic alpha-beta search.

use std::fmt::Debug;

/// Represents the state of a two-player zero-sum game.
pub trait GameState: Clone + Send + Sync {
    /// Returns a hash of the current position for transposition table lookups.
    fn position_hash(&self) -> u64;

    /// Returns true if the current player is the maximizing player.
    fn is_maximizing_player(&self) -> bool;

    /// Switches to the next player's turn.
    fn toggle_turn(&mut self);

    /// Applies a null move (pass turn without making a move).
    /// Used for null move pruning. Default flips the turn.
    fn apply_null_move(&mut self) {
        self.toggle_turn();
    }

    /// Undoes a null move. Default flips the turn back.
    fn undo_null_move(&mut self) {
        self.toggle_turn();
    }
}

/// Represents an action that can be applied to and undone from a game state.
pub trait GameMove: Clone + Send + Sync + PartialEq + Debug {
    type State: GameState;
    type Error: Debug;

    /// Applies this move to the given state.
    fn apply(&self, state: &mut Self::State) -> Result<(), Self::Error>;

    /// Undoes this move on the given state.
    fn undo(&self, state: &mut Self::State) -> Result<(), Self::Error>;

    /// Returns true if this move is "tactical" and should be searched in quiescence.
    /// Tactical moves are those that significantly change the position evaluation and
    /// should not be evaluated at a horizon (e.g., material exchanges, forced sequences).
    /// Default implementation returns false.
    fn is_tactical(&self, _state: &Self::State) -> bool {
        false
    }
}

/// Generates all legal moves from a given game state.
pub trait MoveGenerator<S: GameState>: Clone + Send + Sync {
    type Move: GameMove<State = S>;
    type MoveList: MoveCollection<Self::Move>;

    /// Generates all legal moves for the current player.
    fn generate_moves(&self, state: &mut S) -> Self::MoveList;
}

/// Evaluates a game position and returns a score.
pub trait Evaluator<S: GameState>: Clone + Send + Sync {
    /// Evaluates the given state. Higher scores favor the maximizing player.
    fn evaluate(&self, state: &mut S, remaining_depth: u8) -> i16;

    /// Returns the maximum possible gain from a tactical move in the current position.
    /// Used for delta pruning in quiescence search. Returns a conservative upper bound
    /// on the evaluation improvement possible from any single tactical move.
    /// Default implementation returns a very large value (no pruning).
    fn max_tactical_gain(&self, _state: &S) -> i16 {
        i16::MAX
    }

    /// Returns true if null move pruning should be skipped for this position.
    /// Default returns true (NMP disabled) for safety — games must opt in explicitly.
    fn should_skip_null_move(&self, _state: &mut S) -> bool {
        true
    }

    /// Returns the reverse futility pruning margin for the given depth, or None to
    /// disable RFP. At shallow depths, if the static eval exceeds the opponent's
    /// bound by this margin, the entire subtree is pruned. Larger margins at deeper
    /// depths compensate for the greater risk of missing a tactic.
    /// Default returns None (RFP disabled).
    fn rfp_margin(&self, _depth: u8) -> Option<i16> {
        None
    }

    /// Returns true if the current player is in check.
    /// Used for check extensions (extending search depth by 1 when in check)
    /// and to gate speculative pruning techniques.
    /// Default returns false (no check detection).
    fn is_in_check(&self, _state: &mut S) -> bool {
        false
    }
}

/// Orders moves to improve alpha-beta pruning efficiency.
pub trait MoveOrderer<S: GameState, M>: Clone + Send + Sync {
    /// Sorts moves in-place, placing "better" moves first.
    fn order_moves(&self, moves: &mut [M], state: &S);

    /// Called when a move causes a beta cutoff. Can be used to update
    /// move ordering heuristics (e.g., history table). Default does nothing.
    fn record_cutoff(&self, _mv: &M, _state: &S, _depth: u8) {}
}

/// A no-op move orderer for games without move ordering heuristics.
#[derive(Clone, Default, Debug)]
pub struct NoOpMoveOrderer;

impl<S: GameState, M> MoveOrderer<S, M> for NoOpMoveOrderer {
    #[inline(always)]
    fn order_moves(&self, _moves: &mut [M], _state: &S) {}
}

/// Abstraction over move collections (Vec, SmallVec, etc.)
pub trait MoveCollection<M>: AsRef<[M]> + AsMut<[M]> + Send {
    #[inline]
    fn is_empty(&self) -> bool {
        self.as_ref().is_empty()
    }

    #[inline]
    fn len(&self) -> usize {
        self.as_ref().len()
    }
}

impl<M: Send> MoveCollection<M> for Vec<M> {}
