fn main() {
    let mut board = Board::new();

    board.put(Coordinate::A2, Piece::Pawn(Color::White));

    let piece = board.get(Coordinate::A2);

    println!("Piece at A2: {}", piece.unwrap().to_fen());
}

struct Square {
    coord: Coordinate,
    piece: Option<Piece>,
}

#[derive(Clone, Copy)]
enum Color {
    Black,
    White,
}

#[derive(Clone, Copy)]
enum Piece {
    Bishop(Color),
    King(Color),
    Knight(Color),
    Pawn(Color),
    Queen(Color),
    Rook(Color),
}

impl Piece {
    fn to_fen(self) -> char {
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
}

struct Board {
    squares: Vec<Vec<Square>>,
}

impl Board {
    pub fn new() -> Board {
        let squares = (0..8)
            .map(|row| {
                (0..8)
                    .map(|col| Square {
                        coord: Coordinate::from_row_col(row, col),
                        piece: None,
                    })
                    .collect()
            })
            .collect();

        Board { squares }
    }

    pub fn get(&self, coord: Coordinate) -> Option<Piece> {
        let (row, col) = coord.to_row_col();
        self.squares[row][col].piece
    }

    pub fn put(&mut self, coord: Coordinate, piece: Piece) -> Option<Piece> {
        let (row, col) = coord.to_row_col();
        let prev = self.squares[row][col].piece;
        self.squares[row][col].piece = Some(piece);
        prev
    }
}

#[derive(Clone, Copy)]
enum Coordinate {
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
    A7,
    A8,
    B1,
    B2,
    B3,
    B4,
    B5,
    B6,
    B7,
    B8,
    C1,
    C2,
    C3,
    C4,
    C5,
    C6,
    C7,
    C8,
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
    D8,
    E1,
    E2,
    E3,
    E4,
    E5,
    E6,
    E7,
    E8,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    G1,
    G2,
    G3,
    G4,
    G5,
    G6,
    G7,
    G8,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    H7,
    H8,
}

impl Coordinate {
    fn from_row_col(row: usize, col: usize) -> Coordinate {
        let maybe_coord = match (row, col) {
            (0, 0) => Some(Coordinate::A1),
            (0, 1) => Some(Coordinate::A2),
            (0, 2) => Some(Coordinate::A3),
            (0, 3) => Some(Coordinate::A4),
            (0, 4) => Some(Coordinate::A5),
            (0, 5) => Some(Coordinate::A6),
            (0, 6) => Some(Coordinate::A7),
            (0, 7) => Some(Coordinate::A8),
            (1, 0) => Some(Coordinate::B1),
            (1, 1) => Some(Coordinate::B2),
            (1, 2) => Some(Coordinate::B3),
            (1, 3) => Some(Coordinate::B4),
            (1, 4) => Some(Coordinate::B5),
            (1, 5) => Some(Coordinate::B6),
            (1, 6) => Some(Coordinate::B7),
            (1, 7) => Some(Coordinate::B8),
            (2, 0) => Some(Coordinate::C1),
            (2, 1) => Some(Coordinate::C2),
            (2, 2) => Some(Coordinate::C3),
            (2, 3) => Some(Coordinate::C4),
            (2, 4) => Some(Coordinate::C5),
            (2, 5) => Some(Coordinate::C6),
            (2, 6) => Some(Coordinate::C7),
            (2, 7) => Some(Coordinate::C8),
            (3, 0) => Some(Coordinate::D1),
            (3, 1) => Some(Coordinate::D2),
            (3, 2) => Some(Coordinate::D3),
            (3, 3) => Some(Coordinate::D4),
            (3, 4) => Some(Coordinate::D5),
            (3, 5) => Some(Coordinate::D6),
            (3, 6) => Some(Coordinate::D7),
            (3, 7) => Some(Coordinate::D8),
            (4, 0) => Some(Coordinate::E1),
            (4, 1) => Some(Coordinate::E2),
            (4, 2) => Some(Coordinate::E3),
            (4, 3) => Some(Coordinate::E4),
            (4, 4) => Some(Coordinate::E5),
            (4, 5) => Some(Coordinate::E6),
            (4, 6) => Some(Coordinate::E7),
            (4, 7) => Some(Coordinate::E8),
            (5, 0) => Some(Coordinate::F1),
            (5, 1) => Some(Coordinate::F2),
            (5, 2) => Some(Coordinate::F3),
            (5, 3) => Some(Coordinate::F4),
            (5, 4) => Some(Coordinate::F5),
            (5, 5) => Some(Coordinate::F6),
            (5, 6) => Some(Coordinate::F7),
            (5, 7) => Some(Coordinate::F8),
            (6, 0) => Some(Coordinate::G1),
            (6, 1) => Some(Coordinate::G2),
            (6, 2) => Some(Coordinate::G3),
            (6, 3) => Some(Coordinate::G4),
            (6, 4) => Some(Coordinate::G5),
            (6, 5) => Some(Coordinate::G6),
            (6, 6) => Some(Coordinate::G7),
            (6, 7) => Some(Coordinate::G8),
            (7, 0) => Some(Coordinate::H1),
            (7, 1) => Some(Coordinate::H2),
            (7, 2) => Some(Coordinate::H3),
            (7, 3) => Some(Coordinate::H4),
            (7, 4) => Some(Coordinate::H5),
            (7, 5) => Some(Coordinate::H6),
            (7, 6) => Some(Coordinate::H7),
            (7, 7) => Some(Coordinate::H8),
            (_, _) => None,
        };

        maybe_coord.unwrap()
    }

    fn to_row_col(self) -> (usize, usize) {
        match self {
            Coordinate::A1 => (0, 0),
            Coordinate::A2 => (0, 1),
            Coordinate::A3 => (0, 2),
            Coordinate::A4 => (0, 3),
            Coordinate::A5 => (0, 4),
            Coordinate::A6 => (0, 5),
            Coordinate::A7 => (0, 6),
            Coordinate::A8 => (0, 7),
            Coordinate::B1 => (1, 0),
            Coordinate::B2 => (1, 1),
            Coordinate::B3 => (1, 2),
            Coordinate::B4 => (1, 3),
            Coordinate::B5 => (1, 4),
            Coordinate::B6 => (1, 5),
            Coordinate::B7 => (1, 6),
            Coordinate::B8 => (1, 7),
            Coordinate::C1 => (2, 0),
            Coordinate::C2 => (2, 1),
            Coordinate::C3 => (2, 2),
            Coordinate::C4 => (2, 3),
            Coordinate::C5 => (2, 4),
            Coordinate::C6 => (2, 5),
            Coordinate::C7 => (2, 6),
            Coordinate::C8 => (2, 7),
            Coordinate::D1 => (3, 0),
            Coordinate::D2 => (3, 1),
            Coordinate::D3 => (3, 2),
            Coordinate::D4 => (3, 3),
            Coordinate::D5 => (3, 4),
            Coordinate::D6 => (3, 5),
            Coordinate::D7 => (3, 6),
            Coordinate::D8 => (3, 7),
            Coordinate::E1 => (4, 0),
            Coordinate::E2 => (4, 1),
            Coordinate::E3 => (4, 2),
            Coordinate::E4 => (4, 3),
            Coordinate::E5 => (4, 4),
            Coordinate::E6 => (4, 5),
            Coordinate::E7 => (4, 6),
            Coordinate::E8 => (4, 7),
            Coordinate::F1 => (5, 0),
            Coordinate::F2 => (5, 1),
            Coordinate::F3 => (5, 2),
            Coordinate::F4 => (5, 3),
            Coordinate::F5 => (5, 4),
            Coordinate::F6 => (5, 5),
            Coordinate::F7 => (5, 6),
            Coordinate::F8 => (5, 7),
            Coordinate::G1 => (6, 0),
            Coordinate::G2 => (6, 1),
            Coordinate::G3 => (6, 2),
            Coordinate::G4 => (6, 3),
            Coordinate::G5 => (6, 4),
            Coordinate::G6 => (6, 5),
            Coordinate::G7 => (6, 6),
            Coordinate::G8 => (6, 7),
            Coordinate::H1 => (7, 0),
            Coordinate::H2 => (7, 1),
            Coordinate::H3 => (7, 2),
            Coordinate::H4 => (7, 3),
            Coordinate::H5 => (7, 4),
            Coordinate::H6 => (7, 5),
            Coordinate::H7 => (7, 6),
            Coordinate::H8 => (7, 7),
        }
    }
}
