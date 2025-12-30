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

    /// Returns true if the current player is in check. Used for null move pruning.
    /// Default implementation returns false (null move pruning disabled).
    fn is_in_check(&self) -> bool {
        false
    }

    /// Returns true if the position is an endgame. Used for null move pruning.
    /// Default implementation returns false (null move pruning enabled).
    fn is_endgame(&self) -> bool {
        false
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
    /// Tactical moves are those that change the material balance or create immediate threats
    /// (e.g., captures, checks, promotions). Default implementation returns false.
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
}

/// Orders moves to improve alpha-beta pruning efficiency.
pub trait MoveOrderer<S: GameState, M>: Clone + Send + Sync {
    /// Sorts moves in-place, placing "better" moves first.
    fn order_moves(&self, moves: &mut [M], state: &S);
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
