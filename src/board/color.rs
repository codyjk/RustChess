#[derive(Clone, Copy, PartialEq, Debug, Eq, PartialOrd, Ord)]
pub enum Color {
    Black,
    White,
}

impl Color {
    pub fn opposite(&self) -> Self {
        match self {
            Color::Black => Color::White,
            Color::White => Color::Black,
        }
    }
}
