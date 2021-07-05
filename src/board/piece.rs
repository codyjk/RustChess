#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Color {
    Black,
    White,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Piece {
    Bishop(Color),
    King(Color),
    Knight(Color),
    Pawn(Color),
    Queen(Color),
    Rook(Color),
}

impl Piece {
    pub fn to_fen(&self) -> char {
        match self {
            Piece::Bishop(Color::Black) => 'b',
            Piece::Bishop(Color::White) => 'B',
            Piece::King(Color::Black) => 'k',
            Piece::King(Color::White) => 'K',
            Piece::Knight(Color::Black) => 'n',
            Piece::Knight(Color::White) => 'N',
            Piece::Pawn(Color::Black) => 'p',
            Piece::Pawn(Color::White) => 'P',
            Piece::Queen(Color::Black) => 'q',
            Piece::Queen(Color::White) => 'Q',
            Piece::Rook(Color::Black) => 'r',
            Piece::Rook(Color::White) => 'R',
        }
    }

    pub fn from_fen(c: char) -> Option<Piece> {
        match c {
            'b' => Some(Piece::Bishop(Color::Black)),
            'B' => Some(Piece::Bishop(Color::White)),
            'k' => Some(Piece::King(Color::Black)),
            'K' => Some(Piece::King(Color::White)),
            'n' => Some(Piece::Knight(Color::Black)),
            'N' => Some(Piece::Knight(Color::White)),
            'p' => Some(Piece::Pawn(Color::Black)),
            'P' => Some(Piece::Pawn(Color::White)),
            'q' => Some(Piece::Queen(Color::Black)),
            'Q' => Some(Piece::Queen(Color::White)),
            'r' => Some(Piece::Rook(Color::Black)),
            'R' => Some(Piece::Rook(Color::White)),
            _ => None,
        }
    }
}
