use core::fmt;

use crate::board::bitboard::{A_FILE, H_FILE, RANK_1, RANK_8};

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

impl Direction {
    pub fn all() -> [Direction; 8] {
        [
            Direction::East,
            Direction::North,
            Direction::NorthEast,
            Direction::NorthWest,
            Direction::South,
            Direction::SouthEast,
            Direction::SouthWest,
            Direction::West,
        ]
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let dir = match self {
            Direction::East => "East",
            Direction::North => "North",
            Direction::NorthEast => "NorthEast",
            Direction::NorthWest => "NorthWest",
            Direction::South => "South",
            Direction::SouthEast => "SouthEast",
            Direction::SouthWest => "SouthWest",
            Direction::West => "West",
        };
        write!(f, "{}", dir)
    }
}

pub struct RayTable {
    rays: [u64; 64 * 8], // One entry for each square and direction combination
}

impl Default for RayTable {
    fn default() -> Self {
        let mut table = RayTable { rays: [0; 64 * 8] };
        table.populate();
        table
    }
}

impl RayTable {
    pub fn new() -> Self {
        Default::default()
    }

    fn populate(&mut self) {
        for square_i in 0..64 {
            let square = 1 << square_i;
            for &dir in &Direction::all() {
                let index = Self::index(square, dir);
                self.rays[index] = generate_ray(square, dir);
            }
        }
    }

    pub fn get(&self, square: u64, dir: Direction) -> u64 {
        let index = Self::index(square, dir);
        self.rays[index]
    }

    fn index(square: u64, dir: Direction) -> usize {
        let square_i = square.trailing_zeros(); // nth bit set to 1
        let dir_i = dir as usize; // nth direction
        (square_i as usize) * 8 + dir_i
    }
}

#[rustfmt::skip]
fn generate_ray(square_bit: u64, dir: Direction) -> u64 {
    let mut ray = 0;
    let mut pos = square_bit;

    loop {
        pos = match dir {
            Direction::North => if pos & RANK_8 != 0 { break } else { pos << 8 },
            Direction::South => if pos & RANK_1 != 0 { break } else { pos >> 8 },
            Direction::East => if pos & H_FILE != 0 { break } else { pos << 1 },
            Direction::West => if pos & A_FILE != 0 { break } else { pos >> 1 },
            Direction::NorthEast => if pos & (RANK_8 | H_FILE) != 0 { break } else { pos << 9 },
            Direction::NorthWest => if pos & (RANK_8 | A_FILE) != 0 { break } else { pos << 7 },
            Direction::SouthEast => if pos & (RANK_1 | H_FILE) != 0 { break } else { pos >> 7 },
            Direction::SouthWest => if pos & (RANK_1 | A_FILE) != 0 { break } else { pos >> 9 },
        };
        ray |= pos;
    }

    ray
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{
        bitboard::EMPTY,
        square::{A1, A4, B2, B4, C1, C2, C3, C4, D4, D5, E5, E6, F6, F7, G7, G8, H8},
    };

    #[test]
    fn test_generate_ray() {
        let ray = generate_ray(A1, Direction::North);
        assert_eq!(ray, A_FILE ^ A1);

        let ray = generate_ray(A1, Direction::East);
        assert_eq!(ray, RANK_1 ^ A1);

        let ray = generate_ray(A1, Direction::NorthEast);
        assert_eq!(ray, B2 | C3 | D4 | E5 | F6 | G7 | H8);

        let ray = generate_ray(A1, Direction::NorthWest);
        assert_eq!(ray, EMPTY);

        let ray = generate_ray(A1, Direction::SouthEast);
        assert_eq!(ray, EMPTY);

        let ray = generate_ray(A1, Direction::SouthWest);
        assert_eq!(ray, EMPTY);

        let ray = generate_ray(C4, Direction::South);
        assert_eq!(ray, C3 | C2 | C1);

        let ray = generate_ray(C4, Direction::West);
        assert_eq!(ray, B4 | A4);

        let ray = generate_ray(C4, Direction::NorthEast);
        assert_eq!(ray, D5 | E6 | F7 | G8);
    }

    #[test]
    fn test_construct_ray_table() {
        let table = RayTable::new();
        assert_eq!(table.get(A1, Direction::North), A_FILE ^ A1);
        assert_eq!(table.get(A1, Direction::East), RANK_1 ^ A1);
        assert_eq!(
            table.get(A1, Direction::NorthEast),
            B2 | C3 | D4 | E5 | F6 | G7 | H8
        );
        assert_eq!(table.get(A1, Direction::NorthWest), EMPTY);
        assert_eq!(table.get(A1, Direction::SouthEast), EMPTY);
        assert_eq!(table.get(A1, Direction::SouthWest), EMPTY);
        assert_eq!(table.get(C4, Direction::South), C3 | C2 | C1);
        assert_eq!(table.get(C4, Direction::West), B4 | A4);
        assert_eq!(table.get(C4, Direction::NorthEast), D5 | E6 | F7 | G8);
    }
}
