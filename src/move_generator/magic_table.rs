use common::bitboard::{
    bitboard::Bitboard,
    square::{from_rank_file, to_rank_file, ORDERED_SQUARES},
};

include!(concat!(env!("OUT_DIR"), "/magic_table.rs"));

pub struct MagicEntry {
    mask: u64,
    magic: u64,
    shift: u8,
    offset: u32,
}

pub struct MagicTable {
    rook_table: Vec<Bitboard>,
    bishop_table: Vec<Bitboard>,
}

impl Default for MagicTable {
    fn default() -> Self {
        let rook_table = make_table(
            ROOK_TABLE_SIZE,
            &[(1, 0), (0, -1), (-1, 0), (0, 1)],
            ROOK_MAGICS,
        );
        let bishop_table = make_table(
            BISHOP_TABLE_SIZE,
            &[(1, 1), (1, -1), (-1, -1), (-1, 1)],
            BISHOP_MAGICS,
        );

        Self {
            rook_table,
            bishop_table,
        }
    }
}

impl MagicTable {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_rook_targets(&self, square: Bitboard, blockers: Bitboard) -> Bitboard {
        let magic = &ROOK_MAGICS[square.trailing_zeros() as usize];
        self.rook_table[magic_index(magic, blockers)]
    }

    pub fn get_bishop_targets(&self, square: Bitboard, blockers: Bitboard) -> Bitboard {
        let magic = &BISHOP_MAGICS[square.trailing_zeros() as usize];
        self.bishop_table[magic_index(magic, blockers)]
    }
}

fn make_table(
    table_size: usize,
    slider_deltas: &[(i8, i8)],
    magics: &[MagicEntry; 64],
) -> Vec<Bitboard> {
    let mut table = vec![Bitboard::EMPTY; table_size];
    for &square in &ORDERED_SQUARES {
        let magic_entry = &magics[square.trailing_zeros() as usize];
        let mask = Bitboard(magic_entry.mask);

        let mut blockers = Bitboard::EMPTY;
        loop {
            let moves = slider_moves(slider_deltas, square, blockers);
            table[magic_index(magic_entry, blockers)] = moves;

            // Carry-Rippler trick that enumerates all subsets of the mask, getting us all blockers.
            // https://www.chessprogramming.org/Traversing_Subsets_of_a_Set#All_Subsets_of_any_Set
            blockers.0 = blockers.0.wrapping_sub(mask.0) & mask.0;
            if blockers.is_empty() {
                break;
            }
        }
    }
    table
}

fn slider_moves(slider_deltas: &[(i8, i8)], square: Bitboard, blockers: Bitboard) -> Bitboard {
    let mut moves = Bitboard::EMPTY;
    for &(d_rank, d_file) in slider_deltas {
        let mut ray = square;
        while !blockers.overlaps(ray) {
            if let Some(shifted) = try_offset(ray, d_rank, d_file) {
                ray = shifted;
                moves |= ray;
            } else {
                break;
            }
        }
    }
    moves
}

fn try_offset(square: Bitboard, d_rank: i8, d_file: i8) -> Option<Bitboard> {
    let rank_file_u8 = to_rank_file(square);
    let rank = rank_file_u8.0 as i8;
    let file = rank_file_u8.1 as i8;
    let new_rank = rank.wrapping_add(d_rank);
    let new_file = file.wrapping_add(d_file);
    if !(0..8).contains(&new_rank) || !(0..8).contains(&new_file) {
        None
    } else {
        Some(from_rank_file(new_rank as u8, new_file as u8))
    }
}

fn magic_index(entry: &MagicEntry, blockers: Bitboard) -> usize {
    let blockers = blockers.0 & entry.mask;
    let hash = blockers.wrapping_mul(entry.magic);
    let index = (hash >> entry.shift) as usize;
    entry.offset as usize + index
}

#[cfg(test)]
mod tests {
    use common::bitboard::square::*;

    use crate::{
        board::{color::Color, piece::Piece, Board},
        chess_position,
    };

    use super::*;

    #[test]
    fn test_get_rook_targets() {
        let magic_table = MagicTable::new();
        let board = chess_position! {
            ........
            ..P.....
            ........
            ........
            ........
            P.R..p.p
            ..K.....
            ........
        };
        println!("Testing board:\n{}", board);

        let targets = magic_table.get_rook_targets(C3, board.occupied());
        println!("Rook targets:\n{}", targets);

        // Targets should assume any piece in the way can be taken
        let expected_targets = A3 | B3 | D3 | E3 | F3 | C2 | C4 | C5 | C6 | C7;
        assert_eq!(targets, expected_targets);
    }

    #[test]
    fn test_get_bishop_targets() {
        let magic_table = MagicTable::new();
        let board = chess_position! {
            ........
            .......p
            ........
            ....p...
            ........
            ..B.....
            .b......
            ........
        };

        let targets = magic_table.get_bishop_targets(C3, board.occupied());
        println!("Bishop targets:\n{}", targets);

        // Targets should assume any piece in the way can be taken
        let expected_targets = D4 | E5 | B2 | D2 | E1 | B4 | A5;
        assert_eq!(targets, expected_targets);
    }

    #[test]
    fn test_get_queen_targets() {
        let magic_table = MagicTable::new();
        let board = chess_position! {
            ........
            ........
            ........
            Qp......
            P.......
            ........
            ........
            .R.....k
        };

        println!("Testing board:\n{}", board);
        println!("Occupied squares:\n{}", board.occupied());

        println!("Getting rook targets");
        let rook_targets = magic_table.get_rook_targets(A5, board.occupied());
        println!("Getting bishop targets");
        let bishop_targets = magic_table.get_bishop_targets(A5, board.occupied());
        let targets = rook_targets | bishop_targets;
        println!("Rook targets:\n{}", rook_targets);
        println!("Bishop targets:\n{}", bishop_targets);
        println!("Queen targets:\n{}", targets);

        let expected_targets = Bitboard::EMPTY
            // queen - north
            | A6
            | A7
            | A8
            // queen - northeast
            | B6
            | C7
            | D8
            // queen - east
            | B5
            // queen - southeast
            | B4
            | C3
            | D2
            | E1
            // queen - south
            | A4;

        println!("Expected targets:\n{}", expected_targets);

        assert_eq!(targets, expected_targets);
    }
}
