pub const A1: u64 = 1 << 0;
pub const B1: u64 = 1 << 1;
pub const C1: u64 = 1 << 2;
pub const D1: u64 = 1 << 3;
pub const E1: u64 = 1 << 4;
pub const F1: u64 = 1 << 5;
pub const G1: u64 = 1 << 6;
pub const H1: u64 = 1 << 7;
pub const A2: u64 = 1 << 8;
pub const B2: u64 = 1 << 9;
pub const C2: u64 = 1 << 10;
pub const D2: u64 = 1 << 11;
pub const E2: u64 = 1 << 12;
pub const F2: u64 = 1 << 13;
pub const G2: u64 = 1 << 14;
pub const H2: u64 = 1 << 15;
pub const A3: u64 = 1 << 16;
pub const B3: u64 = 1 << 17;
pub const C3: u64 = 1 << 18;
pub const D3: u64 = 1 << 19;
pub const E3: u64 = 1 << 20;
pub const F3: u64 = 1 << 21;
pub const G3: u64 = 1 << 22;
pub const H3: u64 = 1 << 23;
pub const A4: u64 = 1 << 24;
pub const B4: u64 = 1 << 25;
pub const C4: u64 = 1 << 26;
pub const D4: u64 = 1 << 27;
pub const E4: u64 = 1 << 28;
pub const F4: u64 = 1 << 29;
pub const G4: u64 = 1 << 30;
pub const H4: u64 = 1 << 31;
pub const A5: u64 = 1 << 32;
pub const B5: u64 = 1 << 33;
pub const C5: u64 = 1 << 34;
pub const D5: u64 = 1 << 35;
pub const E5: u64 = 1 << 36;
pub const F5: u64 = 1 << 37;
pub const G5: u64 = 1 << 38;
pub const H5: u64 = 1 << 39;
pub const A6: u64 = 1 << 40;
pub const B6: u64 = 1 << 41;
pub const C6: u64 = 1 << 42;
pub const D6: u64 = 1 << 43;
pub const E6: u64 = 1 << 44;
pub const F6: u64 = 1 << 45;
pub const G6: u64 = 1 << 46;
pub const H6: u64 = 1 << 47;
pub const A7: u64 = 1 << 48;
pub const B7: u64 = 1 << 49;
pub const C7: u64 = 1 << 50;
pub const D7: u64 = 1 << 51;
pub const E7: u64 = 1 << 52;
pub const F7: u64 = 1 << 53;
pub const G7: u64 = 1 << 54;
pub const H7: u64 = 1 << 55;
pub const A8: u64 = 1 << 56;
pub const B8: u64 = 1 << 57;
pub const C8: u64 = 1 << 58;
pub const D8: u64 = 1 << 59;
pub const E8: u64 = 1 << 60;
pub const F8: u64 = 1 << 61;
pub const G8: u64 = 1 << 62;
pub const H8: u64 = 1 << 63;

pub const ORDERED: [u64; 64] = [
    A1, A2, A3, A4, A5, A6, A7, A8, B1, B2, B3, B4, B5, B6, B7, B8, C1, C2, C3, C4, C5, C6, C7, C8,
    D1, D2, D3, D4, D5, D6, D7, D8, E1, E2, E3, E4, E5, E6, E7, E8, F1, F2, F3, F4, F5, F6, F7, F8,
    G1, G2, G3, G4, G5, G6, G7, G8, H1, H2, H3, H4, H5, H6, H7, H8,
];

pub fn is_square(maybe_square: u64) -> bool {
    // it's a square if only 1 bit is set
    (maybe_square & (maybe_square - 1)) == 0
}

pub fn assert(maybe_square: u64) -> u64 {
    assert!(is_square(maybe_square));
    maybe_square
}

pub fn from_row_col(row: usize, col: usize) -> u64 {
    let maybe_square = match (row, col) {
        (0, 0) => Some(A1),
        (1, 0) => Some(A2),
        (2, 0) => Some(A3),
        (3, 0) => Some(A4),
        (4, 0) => Some(A5),
        (5, 0) => Some(A6),
        (6, 0) => Some(A7),
        (7, 0) => Some(A8),
        (0, 1) => Some(B1),
        (1, 1) => Some(B2),
        (2, 1) => Some(B3),
        (3, 1) => Some(B4),
        (4, 1) => Some(B5),
        (5, 1) => Some(B6),
        (6, 1) => Some(B7),
        (7, 1) => Some(B8),
        (0, 2) => Some(C1),
        (1, 2) => Some(C2),
        (2, 2) => Some(C3),
        (3, 2) => Some(C4),
        (4, 2) => Some(C5),
        (5, 2) => Some(C6),
        (6, 2) => Some(C7),
        (7, 2) => Some(C8),
        (0, 3) => Some(D1),
        (1, 3) => Some(D2),
        (2, 3) => Some(D3),
        (3, 3) => Some(D4),
        (4, 3) => Some(D5),
        (5, 3) => Some(D6),
        (6, 3) => Some(D7),
        (7, 3) => Some(D8),
        (0, 4) => Some(E1),
        (1, 4) => Some(E2),
        (2, 4) => Some(E3),
        (3, 4) => Some(E4),
        (4, 4) => Some(E5),
        (5, 4) => Some(E6),
        (6, 4) => Some(E7),
        (7, 4) => Some(E8),
        (0, 5) => Some(F1),
        (1, 5) => Some(F2),
        (2, 5) => Some(F3),
        (3, 5) => Some(F4),
        (4, 5) => Some(F5),
        (5, 5) => Some(F6),
        (6, 5) => Some(F7),
        (7, 5) => Some(F8),
        (0, 6) => Some(G1),
        (1, 6) => Some(G2),
        (2, 6) => Some(G3),
        (3, 6) => Some(G4),
        (4, 6) => Some(G5),
        (5, 6) => Some(G6),
        (6, 6) => Some(G7),
        (7, 6) => Some(G8),
        (0, 7) => Some(H1),
        (1, 7) => Some(H2),
        (2, 7) => Some(H3),
        (3, 7) => Some(H4),
        (4, 7) => Some(H5),
        (5, 7) => Some(H6),
        (6, 7) => Some(H7),
        (7, 7) => Some(H8),
        (_, _) => None,
    };

    maybe_square.unwrap()
}

pub fn from_algebraic(algebraic_coord: &str) -> u64 {
    let maybe_square = match algebraic_coord.to_uppercase().as_str() {
        "A1" => Some(A1),
        "A2" => Some(A2),
        "A3" => Some(A3),
        "A4" => Some(A4),
        "A5" => Some(A5),
        "A6" => Some(A6),
        "A7" => Some(A7),
        "A8" => Some(A8),
        "B1" => Some(B1),
        "B2" => Some(B2),
        "B3" => Some(B3),
        "B4" => Some(B4),
        "B5" => Some(B5),
        "B6" => Some(B6),
        "B7" => Some(B7),
        "B8" => Some(B8),
        "C1" => Some(C1),
        "C2" => Some(C2),
        "C3" => Some(C3),
        "C4" => Some(C4),
        "C5" => Some(C5),
        "C6" => Some(C6),
        "C7" => Some(C7),
        "C8" => Some(C8),
        "D1" => Some(D1),
        "D2" => Some(D2),
        "D3" => Some(D3),
        "D4" => Some(D4),
        "D5" => Some(D5),
        "D6" => Some(D6),
        "D7" => Some(D7),
        "D8" => Some(D8),
        "E1" => Some(E1),
        "E2" => Some(E2),
        "E3" => Some(E3),
        "E4" => Some(E4),
        "E5" => Some(E5),
        "E6" => Some(E6),
        "E7" => Some(E7),
        "E8" => Some(E8),
        "F1" => Some(F1),
        "F2" => Some(F2),
        "F3" => Some(F3),
        "F4" => Some(F4),
        "F5" => Some(F5),
        "F6" => Some(F6),
        "F7" => Some(F7),
        "F8" => Some(F8),
        "G1" => Some(G1),
        "G2" => Some(G2),
        "G3" => Some(G3),
        "G4" => Some(G4),
        "G5" => Some(G5),
        "G6" => Some(G6),
        "G7" => Some(G7),
        "G8" => Some(G8),
        "H1" => Some(H1),
        "H2" => Some(H2),
        "H3" => Some(H3),
        "H4" => Some(H4),
        "H5" => Some(H5),
        "H6" => Some(H6),
        "H7" => Some(H7),
        "H8" => Some(H8),
        _ => None,
    };

    maybe_square.unwrap()
}

pub fn to_algebraic(square: u64) -> &'static str {
    match square {
        A1 => Some("A1"),
        A2 => Some("A2"),
        A3 => Some("A3"),
        A4 => Some("A4"),
        A5 => Some("A5"),
        A6 => Some("A6"),
        A7 => Some("A7"),
        A8 => Some("A8"),
        B1 => Some("B1"),
        B2 => Some("B2"),
        B3 => Some("B3"),
        B4 => Some("B4"),
        B5 => Some("B5"),
        B6 => Some("B6"),
        B7 => Some("B7"),
        B8 => Some("B8"),
        C1 => Some("C1"),
        C2 => Some("C2"),
        C3 => Some("C3"),
        C4 => Some("C4"),
        C5 => Some("C5"),
        C6 => Some("C6"),
        C7 => Some("C7"),
        C8 => Some("C8"),
        D1 => Some("D1"),
        D2 => Some("D2"),
        D3 => Some("D3"),
        D4 => Some("D4"),
        D5 => Some("D5"),
        D6 => Some("D6"),
        D7 => Some("D7"),
        D8 => Some("D8"),
        E1 => Some("E1"),
        E2 => Some("E2"),
        E3 => Some("E3"),
        E4 => Some("E4"),
        E5 => Some("E5"),
        E6 => Some("E6"),
        E7 => Some("E7"),
        E8 => Some("E8"),
        F1 => Some("F1"),
        F2 => Some("F2"),
        F3 => Some("F3"),
        F4 => Some("F4"),
        F5 => Some("F5"),
        F6 => Some("F6"),
        F7 => Some("F7"),
        F8 => Some("F8"),
        G1 => Some("G1"),
        G2 => Some("G2"),
        G3 => Some("G3"),
        G4 => Some("G4"),
        G5 => Some("G5"),
        G6 => Some("G6"),
        G7 => Some("G7"),
        G8 => Some("G8"),
        H1 => Some("H1"),
        H2 => Some("H2"),
        H3 => Some("H3"),
        H4 => Some("H4"),
        H5 => Some("H5"),
        H6 => Some("H6"),
        H7 => Some("H7"),
        H8 => Some("H8"),
        _ => None,
    }
    .unwrap()
}
