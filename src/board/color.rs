use std::fmt;

#[derive(Clone, Copy, PartialEq, Debug, Eq, PartialOrd, Ord)]
pub enum Color {
    Black = 0b01,
    White = 0b10,
}

impl Color {
    pub fn color(&self) -> Self {
        match self {
            Color::Black => Color::Black,
            Color::White => Color::White,
        }
    }

    pub fn opposite(&self) -> Self {
        match self {
            Color::Black => Color::White,
            Color::White => Color::Black,
        }
    }

    pub fn maximize_score(&self) -> bool {
        match self {
            Color::White => true,
            Color::Black => false,
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let color_str = match self {
            Color::Black => "black",
            Color::White => "white",
        };
        write!(f, "{}", color_str)
    }
}
