use crate::board::bitboard::{A_FILE, H_FILE, RANK_1, RANK_8};
use crate::board::square;
use std::collections::HashMap;

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

pub struct RayTable {
    north: HashMap<u64, u64>,
    east: HashMap<u64, u64>,
    south: HashMap<u64, u64>,
    west: HashMap<u64, u64>,
    northeast: HashMap<u64, u64>,
    northwest: HashMap<u64, u64>,
    southeast: HashMap<u64, u64>,
    southwest: HashMap<u64, u64>,
}

impl RayTable {
    pub fn new() -> Self {
        Self {
            north: HashMap::new(),
            east: HashMap::new(),
            south: HashMap::new(),
            west: HashMap::new(),
            northeast: HashMap::new(),
            northwest: HashMap::new(),
            southeast: HashMap::new(),
            southwest: HashMap::new(),
        }
    }

    pub fn populate(&mut self) -> &Self {
        for x in 0..64 {
            let square_bit = 1 << x;
            let square = square::assert(square_bit);

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
        let sq = square::A1;
        let expected_ray_n = 0x0101010101010100; // A_FILE without A1
        let expected_ray_e = 0xFE; // RANK_1 without A1
        assert_eq!(expected_ray_n, generate_rook_ray(sq, Direction::North));
        assert_eq!(expected_ray_e, generate_rook_ray(sq, Direction::East));
        assert_eq!(EMPTY, generate_rook_ray(sq, Direction::South));
        assert_eq!(EMPTY, generate_rook_ray(sq, Direction::West));
    }

    #[test]
    fn test_generate_rook_ray_on_boundary() {
        let sq = square::A2;
        let expected_ray_n = A_FILE ^ square::A1 ^ square::A2;
        let expected_ray_s = square::A1;
        let expected_ray_e = RANK_2 ^ square::A2;
        assert_eq!(expected_ray_n, generate_rook_ray(sq, Direction::North));
        assert_eq!(expected_ray_e, generate_rook_ray(sq, Direction::East));
        assert_eq!(expected_ray_s, generate_rook_ray(sq, Direction::South));
        assert_eq!(EMPTY, generate_rook_ray(sq, Direction::West));
    }

    #[test]
    fn test_generate_rook_ray_in_middle() {
        let sq = square::C3;
        // crude way of building rays...
        let expected_ray_n = C_FILE ^ square::C3 ^ square::C2 ^ square::C1;
        let expected_ray_s =
            C_FILE ^ square::C3 ^ square::C4 ^ square::C5 ^ square::C6 ^ square::C7 ^ square::C8;
        let expected_ray_e = RANK_3 ^ square::C3 ^ square::B3 ^ square::A3;
        let expected_ray_w =
            RANK_3 ^ square::C3 ^ square::D3 ^ square::E3 ^ square::F3 ^ square::G3 ^ square::H3;
        assert_eq!(expected_ray_n, generate_rook_ray(sq, Direction::North));
        assert_eq!(expected_ray_s, generate_rook_ray(sq, Direction::South));
        assert_eq!(expected_ray_e, generate_rook_ray(sq, Direction::East));
        assert_eq!(expected_ray_w, generate_rook_ray(sq, Direction::West));
    }

    #[test]
    fn test_generate_bishop_ray_on_corner() {
        let sq = square::A1;
        let expected_ray_ne = square::B2
            | square::C3
            | square::D4
            | square::E5
            | square::F6
            | square::G7
            | square::H8;
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
        let sq = square::A3;
        let expected_ray_ne = square::B4 | square::C5 | square::D6 | square::E7 | square::F8;
        let expected_ray_se = square::B2 | square::C1;
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
        let sq = square::C3;
        let expected_ray_ne = square::D4 | square::E5 | square::F6 | square::G7 | square::H8;
        let expected_ray_nw = square::B4 | square::A5;
        let expected_ray_se = square::D2 | square::E1;
        let expected_ray_sw = square::B2 | square::A1;
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
