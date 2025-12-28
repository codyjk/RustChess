#[derive(Debug, PartialEq, Clone, Copy, Eq, PartialOrd, Ord)]
pub enum ChessMoveEffect {
    None,
    Check,
    Checkmate,
}
