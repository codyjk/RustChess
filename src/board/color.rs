use rand::seq::SliceRandom;
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Debug, Eq, PartialOrd, Ord)]
pub enum Color {
    Black = 0,
    White = 1,
}

impl Color {
    const ALL: [Color; 2] = [Color::Black, Color::White];

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

    pub fn random() -> Self {
        *Self::ALL.choose(&mut rand::thread_rng()).unwrap()
    }
}

impl From<u8> for Color {
    fn from(value: u8) -> Self {
        match value {
            0 => Color::Black,
            1 => Color::White,
            _ => panic!("Invalid color value: {} (must be 0 or 1)", value),
        }
    }
}

impl From<Color> for u8 {
    fn from(color: Color) -> Self {
        color as u8
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

// used for parsing cli args
type ParseError = &'static str;
impl FromStr for Color {
    type Err = ParseError;
    fn from_str(color: &str) -> Result<Self, Self::Err> {
        match color {
            "black" => Ok(Color::Black),
            "white" => Ok(Color::White),
            "random" => Ok(Color::random()),
            _ => Err("invalid color; options are: black, white, random"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random() {
        assert!(Color::ALL.contains(&Color::random()));
    }

    #[test]
    fn test_parse_white() {
        assert_eq!(Color::White, Color::from_str("white").unwrap());
    }

    #[test]
    fn test_parse_black() {
        assert_eq!(Color::Black, Color::from_str("black").unwrap());
    }

    #[test]
    fn test_parse_random() {
        let rand_color = Color::from_str("random").unwrap();
        assert!(Color::ALL.contains(&rand_color));
    }

    #[test]
    fn test_color_from_u8() {
        assert_eq!(Color::from(0u8), Color::Black);
        assert_eq!(Color::from(1u8), Color::White);
    }

    #[test]
    fn test_color_into_u8() {
        assert_eq!(u8::from(Color::Black), 0);
        assert_eq!(u8::from(Color::White), 1);
    }
}
