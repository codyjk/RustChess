use regex::Regex;

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

const ALGEBRAIC: [&'static str; 64] = [
    "A1", "B1", "C1", "D1", "E1", "F1", "G1", "H1", "A2", "B2", "C2", "D2", "E2", "F2", "G2", "H2",
    "A3", "B3", "C3", "D3", "E3", "F3", "G3", "H3", "A4", "B4", "C4", "D4", "E4", "F4", "G4", "H4",
    "A5", "B5", "C5", "D5", "E5", "F5", "G5", "H5", "A6", "B6", "C6", "D6", "E6", "F6", "G6", "H6",
    "A7", "B7", "C7", "D7", "E7", "F7", "G7", "H7", "A8", "B8", "C8", "D8", "E8", "F8", "G8", "H8",
];

pub fn is_square(maybe_square: u64) -> bool {
    // it's a square if only 1 bit is set
    (maybe_square & (maybe_square - 1)) == 0
}

pub fn assert(maybe_square: u64) -> u64 {
    assert!(is_square(maybe_square));
    maybe_square
}

pub fn from_rank_file(rank: u8, file: u8) -> u64 {
    1 << (file + rank * 8)
}

pub fn from_algebraic(algebraic_coord: &str) -> u64 {
    let re = Regex::new("^([a-hA-H]{1})([1-8]{1})$").unwrap();
    let caps = re.captures(&algebraic_coord).unwrap();
    let rank_raw = &caps[2];
    let file_raw = &caps[1];

    let rank = (rank_raw.chars().nth(0).unwrap().to_digit(10).unwrap() - 1) as u8;
    let file_char = file_raw
        .chars()
        .nth(0)
        .unwrap()
        .to_lowercase()
        .nth(0)
        .unwrap();
    let file = match file_char {
        'a' => Some(0),
        'b' => Some(1),
        'c' => Some(2),
        'd' => Some(3),
        'e' => Some(4),
        'f' => Some(5),
        'g' => Some(6),
        'h' => Some(7),
        _ => None,
    }
    .unwrap();

    from_rank_file(rank, file)
}

pub fn to_algebraic(square: u64) -> &'static str {
    let mut b = assert(square);
    let mut i = 0;
    while b > 0 {
        b = b >> 1;
        i += 1;
    }
    ALGEBRAIC[i - 1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_rank_file() {
        assert_eq!(A1, from_rank_file(0, 0));
        assert_eq!(B2, from_rank_file(1, 1));
        assert_eq!(E4, from_rank_file(3, 4));
    }

    #[test]
    fn test_from_algebraic() {
        assert_eq!(A1, from_algebraic("A1"));
        assert_eq!(A1, from_algebraic("a1"));
        assert_eq!(E5, from_algebraic("E5"));
    }

    #[test]
    fn test_to_algebraic() {
        assert_eq!("A1", to_algebraic(A1));
        assert_eq!("A8", to_algebraic(A8));
        assert_eq!("B8", to_algebraic(B8));
        assert_eq!("H8", to_algebraic(H8));
    }
}
