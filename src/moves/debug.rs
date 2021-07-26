use std::fmt;

use super::ChessMove;

impl fmt::Debug for ChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}",
            self.from_square.to_algebraic().to_lowercase(),
            self.to_square.to_algebraic().to_lowercase()
        )
    }
}
