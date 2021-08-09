use std::fmt;

use super::ChessMove;

impl fmt::Debug for ChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let capture_msg = match self.capture {
            Some((piece, color)) => format!(" (captures {})", piece.to_fen(color)),
            None => "".to_string(),
        };

        write!(
            f,
            "{}{}{}",
            self.from_square.to_algebraic().to_lowercase(),
            self.to_square.to_algebraic().to_lowercase(),
            capture_msg
        )
    }
}
