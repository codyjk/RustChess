use common::bitboard::Square;

use super::castle_rights::CastleRights;
use super::fullmove_number::FullmoveNumber;
use super::halfmove_clock::HalfmoveClock;
use super::state_stack::StateStack;

/// Stores information about state changes related to individual chess moves,
/// including en passant targets, castle rights, and position clocks.
#[derive(Clone)]
pub struct MoveInfo {
    en_passant_target_stack: StateStack<Option<Square>>,
    castle_rights_stack: StateStack<CastleRights>,
    halfmove_clock_stack: StateStack<HalfmoveClock>,
    fullmove_clock: FullmoveNumber,
}

impl Default for MoveInfo {
    fn default() -> Self {
        Self {
            en_passant_target_stack: StateStack::new(None),
            castle_rights_stack: StateStack::new(CastleRights::all()),
            halfmove_clock_stack: StateStack::new(HalfmoveClock::new(0)),
            fullmove_clock: FullmoveNumber::new(1),
        }
    }
}

impl MoveInfo {
    pub fn new() -> Self {
        Default::default()
    }

    // En passant state management

    pub fn push_en_passant_target(&mut self, target_square: Option<Square>) -> Option<Square> {
        self.en_passant_target_stack.push(target_square)
    }

    pub fn peek_en_passant_target(&self) -> Option<Square> {
        *self.en_passant_target_stack.peek()
    }

    pub fn pop_en_passant_target(&mut self) -> Option<Square> {
        self.en_passant_target_stack.pop()
    }

    // Castle rights state management

    /// Returns the current set of castle rights.
    pub fn peek_castle_rights(&self) -> CastleRights {
        *self.castle_rights_stack.peek()
    }

    /// Returns the bitmasks of the previous set of castle rights, as well as the
    /// new set of castle rights after losing the specified rights.
    /// The new set of rights is pushed onto the stack.
    pub fn lose_castle_rights(
        &mut self,
        lost_rights: CastleRights,
    ) -> (CastleRights, CastleRights) {
        let old_rights = self.peek_castle_rights();
        let new_rights = CastleRights::new(old_rights.bits() ^ (old_rights.intersection(lost_rights).bits()));
        self.castle_rights_stack.push(new_rights);
        (old_rights, new_rights)
    }

    /// The inverse of `lose_castle_rights`. Pops the last set of castle rights
    /// off the stack and returns the previous set of rights, as well as the new
    /// set of rights.
    pub fn pop_castle_rights(&mut self) -> (CastleRights, CastleRights) {
        let old_rights = self.castle_rights_stack.pop();
        let new_rights = self.peek_castle_rights();
        (old_rights, new_rights)
    }

    /// Preserves the current castle rights by pushing the current rights onto the stack.
    pub fn preserve_castle_rights(&mut self) -> CastleRights {
        let rights = self.peek_castle_rights();
        self.castle_rights_stack.push(rights)
    }

    // Position clock state management

    pub fn increment_fullmove_clock(&mut self) -> FullmoveNumber {
        self.fullmove_clock = self.fullmove_clock.increment();
        self.fullmove_clock
    }

    pub fn decrement_fullmove_clock(&mut self) -> FullmoveNumber {
        self.fullmove_clock = self.fullmove_clock.decrement();
        self.fullmove_clock
    }

    pub fn set_fullmove_clock(&mut self, clock: FullmoveNumber) -> FullmoveNumber {
        self.fullmove_clock = clock;
        clock
    }

    pub fn fullmove_clock(&self) -> FullmoveNumber {
        self.fullmove_clock
    }

    pub fn push_halfmove_clock(&mut self, clock: HalfmoveClock) -> HalfmoveClock {
        self.halfmove_clock_stack.push(clock)
    }

    pub fn increment_halfmove_clock(&mut self) -> HalfmoveClock {
        let new_clock = self.halfmove_clock_stack.peek().increment();
        self.halfmove_clock_stack.push(new_clock)
    }

    pub fn reset_halfmove_clock(&mut self) -> HalfmoveClock {
        self.halfmove_clock_stack.push(HalfmoveClock::new(0))
    }

    pub fn halfmove_clock(&self) -> HalfmoveClock {
        *self.halfmove_clock_stack.peek()
    }

    pub fn pop_halfmove_clock(&mut self) -> HalfmoveClock {
        self.halfmove_clock_stack.pop()
    }
}
