use super::{
    bitboard::EMPTY,
    castle_rights::{CastleRightsBitmask, ALL_CASTLE_RIGHTS},
};

/// Stores information about state changes related to individual chess moves.
#[derive(Clone)]
pub struct MoveInfo {
    en_passant_target_stack: Vec<u64>,
    castle_rights_stack: Vec<CastleRightsBitmask>,
    halfmove_clock_stack: Vec<u8>,
    fullmove_clock: u8,
}

impl Default for MoveInfo {
    fn default() -> Self {
        Self {
            en_passant_target_stack: vec![EMPTY],
            castle_rights_stack: vec![ALL_CASTLE_RIGHTS],
            halfmove_clock_stack: vec![0],
            fullmove_clock: 1,
        }
    }
}

impl MoveInfo {
    pub fn new() -> Self {
        Default::default()
    }

    // En passant state management

    pub fn push_en_passant_target(&mut self, target_square: u64) -> u64 {
        self.en_passant_target_stack.push(target_square);
        target_square
    }

    pub fn peek_en_passant_target(&self) -> u64 {
        *self.en_passant_target_stack.last().unwrap()
    }

    pub fn pop_en_passant_target(&mut self) -> u64 {
        self.en_passant_target_stack.pop().unwrap()
    }

    // Castle rights state management

    /// Returns the current set of castle rights.
    pub fn peek_castle_rights(&self) -> u8 {
        *self.castle_rights_stack.last().unwrap()
    }

    /// Returns the bitmasks of the previous set of castle rights, as well as the
    /// new set of castle rights after losing the specified rights.
    /// The new set of rights is pushed onto the stack.
    pub fn lose_castle_rights(
        &mut self,
        lost_rights: CastleRightsBitmask,
    ) -> (CastleRightsBitmask, CastleRightsBitmask) {
        let old_rights = self.peek_castle_rights();
        let new_rights = old_rights ^ (old_rights & lost_rights);
        self.castle_rights_stack.push(new_rights);
        (old_rights, new_rights)
    }

    /// The inverse of `lose_castle_rights`. Pops the last set of castle rights
    /// off the stack and returns the previous set of rights, as well as the new
    /// set of rights.
    pub fn pop_castle_rights(&mut self) -> (CastleRightsBitmask, CastleRightsBitmask) {
        let old_rights = self.castle_rights_stack.pop().unwrap();
        let new_rights = self.peek_castle_rights();
        (old_rights, new_rights)
    }

    /// Preserves the current castle rights by pushing the current rights onto the stack.
    pub fn preserve_castle_rights(&mut self) -> CastleRightsBitmask {
        let rights = self.peek_castle_rights();
        self.castle_rights_stack.push(rights);
        rights
    }

    // Position clock state management

    pub fn increment_fullmove_clock(&mut self) -> u8 {
        self.fullmove_clock += 1;
        self.fullmove_clock
    }

    pub fn decrement_fullmove_clock(&mut self) -> u8 {
        self.fullmove_clock -= 1;
        self.fullmove_clock
    }

    pub fn set_fullmove_clock(&mut self, clock: u8) -> u8 {
        self.fullmove_clock = clock;
        clock
    }

    pub fn fullmove_clock(&self) -> u8 {
        self.fullmove_clock
    }

    pub fn push_halfmove_clock(&mut self, clock: u8) -> u8 {
        self.halfmove_clock_stack.push(clock);
        clock
    }

    pub fn increment_halfmove_clock(&mut self) -> u8 {
        let old_clock = self.halfmove_clock_stack.last().unwrap();
        let new_clock = old_clock + 1;
        self.halfmove_clock_stack.push(new_clock);
        new_clock
    }

    pub fn reset_halfmove_clock(&mut self) -> u8 {
        self.halfmove_clock_stack.push(0);
        0
    }

    pub fn halfmove_clock(&self) -> u8 {
        *self.halfmove_clock_stack.last().unwrap()
    }

    pub fn pop_halfmove_clock(&mut self) -> u8 {
        self.halfmove_clock_stack.pop().unwrap()
    }
}
