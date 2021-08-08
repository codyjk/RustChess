use crate::board::bitboard::{Bitboard, A_FILE, H_FILE, RANK_1, RANK_8};
use crate::board::square::Square;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Debug, Eq, Hash)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

pub const ROOK_DIRS: [Direction; 4] = [
    Direction::North,
    Direction::South,
    Direction::East,
    Direction::West,
];

type Ray = (Square, Direction);

pub struct RayTable {
    table: HashMap<Ray, Bitboard>,
}

impl RayTable {
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
        }
    }

    pub fn populate(&mut self) -> &Self {
        for x in 0..64 {
            let square_bit = 1 << x;
            let square = Square::from_bitboard(square_bit);

            for dir in ROOK_DIRS.iter() {
                self.table
                    .insert((square, *dir), generate_rook_ray(square_bit, *dir));
            }
        }

        self
    }

    pub fn get(&self, square: Square, dir: Direction) -> Bitboard {
        let ray = (square, dir);
        *self.table.get(&ray).unwrap()
    }
}

fn generate_rook_ray(square_bit: Bitboard, dir: Direction) -> Bitboard {
    let mut ray = square_bit;

    let boundary = match dir {
        Direction::North => RANK_8,
        Direction::South => RANK_1,
        Direction::East => H_FILE,
        Direction::West => A_FILE,
    };

    while ray & boundary == 0 {
        let next_ray = match dir {
            Direction::North => ray << 8,
            Direction::South => ray >> 8,
            Direction::East => ray << 1,
            Direction::West => ray >> 1,
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
        let sq = Square::A1;
        let sq_bit = sq.to_bitboard();
        let expected_ray_n = 0x0101010101010100; // A_FILE without A1
        let expected_ray_e = 0xFE; // RANK_1 without A1
        assert_eq!(expected_ray_n, generate_rook_ray(sq_bit, Direction::North));
        assert_eq!(expected_ray_e, generate_rook_ray(sq_bit, Direction::East));
        assert_eq!(EMPTY, generate_rook_ray(sq_bit, Direction::South));
        assert_eq!(EMPTY, generate_rook_ray(sq_bit, Direction::West));
    }

    #[test]
    fn test_generate_rook_ray_on_boundary() {
        let sq = Square::A2;
        let sq_bit = sq.to_bitboard();
        let expected_ray_n = A_FILE ^ Square::A1.to_bitboard() ^ Square::A2.to_bitboard();
        let expected_ray_s = Square::A1.to_bitboard();
        let expected_ray_e = RANK_2 ^ Square::A2.to_bitboard();
        assert_eq!(expected_ray_n, generate_rook_ray(sq_bit, Direction::North));
        assert_eq!(expected_ray_e, generate_rook_ray(sq_bit, Direction::East));
        assert_eq!(expected_ray_s, generate_rook_ray(sq_bit, Direction::South));
        assert_eq!(EMPTY, generate_rook_ray(sq_bit, Direction::West));
    }

    #[test]
    fn test_generate_rook_ray_in_middle() {
        let sq = Square::C3;
        let sq_bit = sq.to_bitboard();
        // crude way of building rays...
        let expected_ray_n =
            C_FILE ^ Square::C3.to_bitboard() ^ Square::C2.to_bitboard() ^ Square::C1.to_bitboard();
        let expected_ray_s = C_FILE
            ^ Square::C3.to_bitboard()
            ^ Square::C4.to_bitboard()
            ^ Square::C5.to_bitboard()
            ^ Square::C6.to_bitboard()
            ^ Square::C7.to_bitboard()
            ^ Square::C8.to_bitboard();
        let expected_ray_e =
            RANK_3 ^ Square::C3.to_bitboard() ^ Square::B3.to_bitboard() ^ Square::A3.to_bitboard();
        let expected_ray_w = RANK_3
            ^ Square::C3.to_bitboard()
            ^ Square::D3.to_bitboard()
            ^ Square::E3.to_bitboard()
            ^ Square::F3.to_bitboard()
            ^ Square::G3.to_bitboard()
            ^ Square::H3.to_bitboard();
        assert_eq!(expected_ray_n, generate_rook_ray(sq_bit, Direction::North));
        assert_eq!(expected_ray_s, generate_rook_ray(sq_bit, Direction::South));
        assert_eq!(expected_ray_e, generate_rook_ray(sq_bit, Direction::East));
        assert_eq!(expected_ray_w, generate_rook_ray(sq_bit, Direction::West));
    }
}
