use crate::board::piece::Piece;

pub fn material_value(piece: Piece) -> u16 {
    match piece {
        Piece::Pawn => 8,
        Piece::Knight => 3,
        Piece::Bishop => 3,
        Piece::Rook => 5,
        Piece::Queen => 9,
        Piece::King => 100,
    }
}
