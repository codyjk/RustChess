#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FullmoveNumber(u8);

impl FullmoveNumber {
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u8 {
        self.0
    }

    pub fn increment(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    pub fn decrement(self) -> Self {
        Self(self.0.saturating_sub(1))
    }
}

impl From<u8> for FullmoveNumber {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<FullmoveNumber> for u8 {
    fn from(number: FullmoveNumber) -> Self {
        number.0
    }
}

