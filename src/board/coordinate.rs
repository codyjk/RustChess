#[derive(Clone, Copy, PartialEq)]
pub enum Coordinate {
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
    pub fn from_row_col(row: usize, col: usize) -> Coordinate {
        let maybe_coord = match (row, col) {
            (0, 0) => Some(Coordinate::A1),
            (1, 0) => Some(Coordinate::A2),
            (2, 0) => Some(Coordinate::A3),
            (3, 0) => Some(Coordinate::A4),
            (4, 0) => Some(Coordinate::A5),
            (5, 0) => Some(Coordinate::A6),
            (6, 0) => Some(Coordinate::A7),
            (7, 0) => Some(Coordinate::A8),
            (0, 1) => Some(Coordinate::B1),
            (1, 1) => Some(Coordinate::B2),
            (2, 1) => Some(Coordinate::B3),
            (3, 1) => Some(Coordinate::B4),
            (4, 1) => Some(Coordinate::B5),
            (5, 1) => Some(Coordinate::B6),
            (6, 1) => Some(Coordinate::B7),
            (7, 1) => Some(Coordinate::B8),
            (0, 2) => Some(Coordinate::C1),
            (1, 2) => Some(Coordinate::C2),
            (2, 2) => Some(Coordinate::C3),
            (3, 2) => Some(Coordinate::C4),
            (4, 2) => Some(Coordinate::C5),
            (5, 2) => Some(Coordinate::C6),
            (6, 2) => Some(Coordinate::C7),
            (7, 2) => Some(Coordinate::C8),
            (0, 3) => Some(Coordinate::D1),
            (1, 3) => Some(Coordinate::D2),
            (2, 3) => Some(Coordinate::D3),
            (3, 3) => Some(Coordinate::D4),
            (4, 3) => Some(Coordinate::D5),
            (5, 3) => Some(Coordinate::D6),
            (6, 3) => Some(Coordinate::D7),
            (7, 3) => Some(Coordinate::D8),
            (0, 4) => Some(Coordinate::E1),
            (1, 4) => Some(Coordinate::E2),
            (2, 4) => Some(Coordinate::E3),
            (3, 4) => Some(Coordinate::E4),
            (4, 4) => Some(Coordinate::E5),
            (5, 4) => Some(Coordinate::E6),
            (6, 4) => Some(Coordinate::E7),
            (7, 4) => Some(Coordinate::E8),
            (0, 5) => Some(Coordinate::F1),
            (1, 5) => Some(Coordinate::F2),
            (2, 5) => Some(Coordinate::F3),
            (3, 5) => Some(Coordinate::F4),
            (4, 5) => Some(Coordinate::F5),
            (5, 5) => Some(Coordinate::F6),
            (6, 5) => Some(Coordinate::F7),
            (7, 5) => Some(Coordinate::F8),
            (0, 6) => Some(Coordinate::G1),
            (1, 6) => Some(Coordinate::G2),
            (2, 6) => Some(Coordinate::G3),
            (3, 6) => Some(Coordinate::G4),
            (4, 6) => Some(Coordinate::G5),
            (5, 6) => Some(Coordinate::G6),
            (6, 6) => Some(Coordinate::G7),
            (7, 6) => Some(Coordinate::G8),
            (0, 7) => Some(Coordinate::H1),
            (1, 7) => Some(Coordinate::H2),
            (2, 7) => Some(Coordinate::H3),
            (3, 7) => Some(Coordinate::H4),
            (4, 7) => Some(Coordinate::H5),
            (5, 7) => Some(Coordinate::H6),
            (6, 7) => Some(Coordinate::H7),
            (7, 7) => Some(Coordinate::H8),
            (_, _) => None,
        };

        maybe_coord.unwrap()
    }

    pub fn to_row_col(&self) -> (usize, usize) {
        match self {
            Coordinate::A1 => (0, 0),
            Coordinate::A2 => (1, 0),
            Coordinate::A3 => (2, 0),
            Coordinate::A4 => (3, 0),
            Coordinate::A5 => (4, 0),
            Coordinate::A6 => (5, 0),
            Coordinate::A7 => (6, 0),
            Coordinate::A8 => (7, 0),
            Coordinate::B1 => (0, 1),
            Coordinate::B2 => (1, 1),
            Coordinate::B3 => (2, 1),
            Coordinate::B4 => (3, 1),
            Coordinate::B5 => (4, 1),
            Coordinate::B6 => (5, 1),
            Coordinate::B7 => (6, 1),
            Coordinate::B8 => (7, 1),
            Coordinate::C1 => (0, 2),
            Coordinate::C2 => (1, 2),
            Coordinate::C3 => (2, 2),
            Coordinate::C4 => (3, 2),
            Coordinate::C5 => (4, 2),
            Coordinate::C6 => (5, 2),
            Coordinate::C7 => (6, 2),
            Coordinate::C8 => (7, 2),
            Coordinate::D1 => (0, 3),
            Coordinate::D2 => (1, 3),
            Coordinate::D3 => (2, 3),
            Coordinate::D4 => (3, 3),
            Coordinate::D5 => (4, 3),
            Coordinate::D6 => (5, 3),
            Coordinate::D7 => (6, 3),
            Coordinate::D8 => (7, 3),
            Coordinate::E1 => (0, 4),
            Coordinate::E2 => (1, 4),
            Coordinate::E3 => (2, 4),
            Coordinate::E4 => (3, 4),
            Coordinate::E5 => (4, 4),
            Coordinate::E6 => (5, 4),
            Coordinate::E7 => (6, 4),
            Coordinate::E8 => (7, 4),
            Coordinate::F1 => (0, 5),
            Coordinate::F2 => (1, 5),
            Coordinate::F3 => (2, 5),
            Coordinate::F4 => (3, 5),
            Coordinate::F5 => (4, 5),
            Coordinate::F6 => (5, 5),
            Coordinate::F7 => (6, 5),
            Coordinate::F8 => (7, 5),
            Coordinate::G1 => (0, 6),
            Coordinate::G2 => (1, 6),
            Coordinate::G3 => (2, 6),
            Coordinate::G4 => (3, 6),
            Coordinate::G5 => (4, 6),
            Coordinate::G6 => (5, 6),
            Coordinate::G7 => (6, 6),
            Coordinate::G8 => (7, 6),
            Coordinate::H1 => (0, 7),
            Coordinate::H2 => (1, 7),
            Coordinate::H3 => (2, 7),
            Coordinate::H4 => (3, 7),
            Coordinate::H5 => (4, 7),
            Coordinate::H6 => (5, 7),
            Coordinate::H7 => (6, 7),
            Coordinate::H8 => (7, 7),
        }
    }

    pub fn all() -> Vec<Coordinate> {
        vec![
            Coordinate::A1,
            Coordinate::A2,
            Coordinate::A3,
            Coordinate::A4,
            Coordinate::A5,
            Coordinate::A6,
            Coordinate::A7,
            Coordinate::A8,
            Coordinate::B1,
            Coordinate::B2,
            Coordinate::B3,
            Coordinate::B4,
            Coordinate::B5,
            Coordinate::B6,
            Coordinate::B7,
            Coordinate::B8,
            Coordinate::C1,
            Coordinate::C2,
            Coordinate::C3,
            Coordinate::C4,
            Coordinate::C5,
            Coordinate::C6,
            Coordinate::C7,
            Coordinate::C8,
            Coordinate::D1,
            Coordinate::D2,
            Coordinate::D3,
            Coordinate::D4,
            Coordinate::D5,
            Coordinate::D6,
            Coordinate::D7,
            Coordinate::D8,
            Coordinate::E1,
            Coordinate::E2,
            Coordinate::E3,
            Coordinate::E4,
            Coordinate::E5,
            Coordinate::E6,
            Coordinate::E7,
            Coordinate::E8,
            Coordinate::F1,
            Coordinate::F2,
            Coordinate::F3,
            Coordinate::F4,
            Coordinate::F5,
            Coordinate::F6,
            Coordinate::F7,
            Coordinate::F8,
            Coordinate::G1,
            Coordinate::G2,
            Coordinate::G3,
            Coordinate::G4,
            Coordinate::G5,
            Coordinate::G6,
            Coordinate::G7,
            Coordinate::G8,
            Coordinate::H1,
            Coordinate::H2,
            Coordinate::H3,
            Coordinate::H4,
            Coordinate::H5,
            Coordinate::H6,
            Coordinate::H7,
            Coordinate::H8,
        ]
    }
}
