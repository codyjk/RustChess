#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HalfmoveClock(u8);

impl HalfmoveClock {
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u8 {
        self.0
    }

    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn increment(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    pub fn reset(self) -> Self {
        Self(0)
    }
}

impl From<u8> for HalfmoveClock {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<HalfmoveClock> for u8 {
    fn from(clock: HalfmoveClock) -> Self {
        clock.0
    }
}

