use crate::board::bitboard::{A_FILE, H_FILE, RANK_1, RANK_8};
use crate::board::square::*;
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, PartialEq, Debug, Eq, Hash)]
pub enum Direction {
    East,
    North,
    NorthEast,
    NorthWest,
    South,
    SouthEast,
    SouthWest,
    West,
}

pub const ROOK_DIRS: [Direction; 4] = [
    Direction::East,
    Direction::North,
    Direction::South,
    Direction::West,
];

pub const BISHOP_DIRS: [Direction; 4] = [
    Direction::NorthEast,
    Direction::NorthWest,
    Direction::SouthEast,
    Direction::SouthWest,
];

#[derive(Default)]
pub struct RayTable {
    north: FxHashMap<u64, u64>,
    east: FxHashMap<u64, u64>,
    south: FxHashMap<u64, u64>,
    west: FxHashMap<u64, u64>,
    northeast: FxHashMap<u64, u64>,
    northwest: FxHashMap<u64, u64>,
    southeast: FxHashMap<u64, u64>,
    southwest: FxHashMap<u64, u64>,
}

impl RayTable {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn populate(&mut self) -> &Self {
        for x in 0..64 {
            let square_bit = 1 << x;
            let square = assert(square_bit);

            self.north
                .insert(square, generate_rook_ray(square, Direction::North));
            self.east
                .insert(square, generate_rook_ray(square, Direction::East));
            self.south
                .insert(square, generate_rook_ray(square, Direction::South));
            self.west
                .insert(square, generate_rook_ray(square, Direction::West));
            self.northeast
                .insert(square, generate_bishop_ray(square, Direction::NorthEast));
            self.northwest
                .insert(square, generate_bishop_ray(square, Direction::NorthWest));
            self.southeast
                .insert(square, generate_bishop_ray(square, Direction::SouthEast));
            self.southwest
                .insert(square, generate_bishop_ray(square, Direction::SouthWest));
        }

        self
    }

    pub fn get(&self, square: u64, dir: Direction) -> u64 {
        let inner_table = match dir {
            Direction::North => &self.north,
            Direction::East => &self.east,
            Direction::South => &self.south,
            Direction::West => &self.west,
            Direction::NorthEast => &self.northeast,
            Direction::NorthWest => &self.northwest,
            Direction::SouthEast => &self.southeast,
            Direction::SouthWest => &self.southwest,
        };
        *inner_table.get(&square).unwrap()
    }
}

fn generate_rook_ray(square_bit: u64, dir: Direction) -> u64 {
    let mut ray = square_bit;

    let boundary = match dir {
        Direction::North => RANK_8,
        Direction::South => RANK_1,
        Direction::East => H_FILE,
        Direction::West => A_FILE,
        _ => 0,
    };

    while ray & boundary == 0 {
        let next_ray = match dir {
            Direction::North => ray << 8,
            Direction::South => ray >> 8,
            Direction::East => ray << 1,
            Direction::West => ray >> 1,
            _ => 0,
        };
        ray |= next_ray;
    }

    ray ^= square_bit;

    ray
}

fn generate_bishop_ray(square_bit: u64, dir: Direction) -> u64 {
    let mut ray = square_bit;

    let (boundary_rank, boundary_file) = match dir {
        Direction::NorthWest => (RANK_8, A_FILE),
        Direction::NorthEast => (RANK_8, H_FILE),
        Direction::SouthWest => (RANK_1, A_FILE),
        Direction::SouthEast => (RANK_1, H_FILE),
        _ => (0, 0),
    };

    while ray & boundary_rank == 0 && ray & boundary_file == 0 {
        let next_ray = match dir {
            Direction::NorthWest => ray << 7,
            Direction::NorthEast => ray << 9,
            Direction::SouthWest => ray >> 9,
            Direction::SouthEast => ray >> 7,
            _ => 0,
        };
        ray |= next_ray;
    }

    ray ^= square_bit;

    ray
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::bitboard::{C_FILE, EMPTY, RANK_2, RANK_3};

    #[test]
    fn test_generate_rook_ray_on_corner() {
        let sq = A1;
        let expected_ray_n = 0x0101010101010100; // A_FILE without A1
        let expected_ray_e = 0xFE; // RANK_1 without A1
        assert_eq!(expected_ray_n, generate_rook_ray(sq, Direction::North));
        assert_eq!(expected_ray_e, generate_rook_ray(sq, Direction::East));
        assert_eq!(EMPTY, generate_rook_ray(sq, Direction::South));
        assert_eq!(EMPTY, generate_rook_ray(sq, Direction::West));
    }

    #[test]
    fn test_generate_rook_ray_on_boundary() {
        let sq = A2;
        let expected_ray_n = A_FILE ^ A1 ^ A2;
        let expected_ray_s = A1;
        let expected_ray_e = RANK_2 ^ A2;
        assert_eq!(expected_ray_n, generate_rook_ray(sq, Direction::North));
        assert_eq!(expected_ray_e, generate_rook_ray(sq, Direction::East));
        assert_eq!(expected_ray_s, generate_rook_ray(sq, Direction::South));
        assert_eq!(EMPTY, generate_rook_ray(sq, Direction::West));
    }

    #[test]
    fn test_generate_rook_ray_in_middle() {
        let sq = C3;
        // crude way of building rays...
        let expected_ray_n = C_FILE ^ C3 ^ C2 ^ C1;
        let expected_ray_s = C_FILE ^ C3 ^ C4 ^ C5 ^ C6 ^ C7 ^ C8;
        let expected_ray_e = RANK_3 ^ C3 ^ B3 ^ A3;
        let expected_ray_w = RANK_3 ^ C3 ^ D3 ^ E3 ^ F3 ^ G3 ^ H3;
        assert_eq!(expected_ray_n, generate_rook_ray(sq, Direction::North));
        assert_eq!(expected_ray_s, generate_rook_ray(sq, Direction::South));
        assert_eq!(expected_ray_e, generate_rook_ray(sq, Direction::East));
        assert_eq!(expected_ray_w, generate_rook_ray(sq, Direction::West));
    }

    #[test]
    fn test_generate_bishop_ray_on_corner() {
        let sq = A1;
        let expected_ray_ne = B2 | C3 | D4 | E5 | F6 | G7 | H8;
        assert_eq!(
            expected_ray_ne,
            generate_bishop_ray(sq, Direction::NorthEast)
        );
        assert_eq!(EMPTY, generate_bishop_ray(sq, Direction::NorthWest));
        assert_eq!(EMPTY, generate_bishop_ray(sq, Direction::SouthWest));
        assert_eq!(EMPTY, generate_bishop_ray(sq, Direction::SouthEast));
    }

    #[test]
    fn test_generate_bishop_ray_on_boundary() {
        let sq = A3;
        let expected_ray_ne = B4 | C5 | D6 | E7 | F8;
        let expected_ray_se = B2 | C1;
        assert_eq!(
            expected_ray_ne,
            generate_bishop_ray(sq, Direction::NorthEast)
        );
        assert_eq!(EMPTY, generate_bishop_ray(sq, Direction::NorthWest));
        assert_eq!(EMPTY, generate_bishop_ray(sq, Direction::SouthWest));
        assert_eq!(
            expected_ray_se,
            generate_bishop_ray(sq, Direction::SouthEast)
        );
    }

    #[test]
    fn test_generate_bishop_ray_in_middle() {
        let sq = C3;
        let expected_ray_ne = D4 | E5 | F6 | G7 | H8;
        let expected_ray_nw = B4 | A5;
        let expected_ray_se = D2 | E1;
        let expected_ray_sw = B2 | A1;
        assert_eq!(
            expected_ray_ne,
            generate_bishop_ray(sq, Direction::NorthEast)
        );
        assert_eq!(
            expected_ray_nw,
            generate_bishop_ray(sq, Direction::NorthWest)
        );
        assert_eq!(
            expected_ray_sw,
            generate_bishop_ray(sq, Direction::SouthWest)
        );
        assert_eq!(
            expected_ray_se,
            generate_bishop_ray(sq, Direction::SouthEast)
        );
    }
}
