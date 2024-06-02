use std::fs::File;
use std::io::{BufWriter, Write};

use common::bitboard::square::{from_rank_file, to_rank_file};
use common::bitboard::bitboard::Bitboard;

use log::debug;

use crate::random_number_generator::generate_random_u64;

// Based off of https://analog-hors.github.io/site/magic-bitboards/

struct SlidingPiece {
    deltas: [(i8, i8); 4],
}

impl SlidingPiece {
    fn moves(&self, square: Bitboard, blockers: Bitboard) -> Bitboard {
        let mut moves = Bitboard::EMPTY;
        for &(d_rank, d_file) in &self.deltas {
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

    fn relevant_blockers(&self, square: Bitboard) -> Bitboard {
        let mut blockers = Bitboard::EMPTY;
        for &(d_rank, d_file) in &self.deltas {
            let mut ray = square;
            while let Some(shifted) = try_offset(ray, d_rank, d_file) {
                blockers |= ray;
                ray = shifted;
            }
        }
        blockers &= !square;
        blockers
    }
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

const ROOK: SlidingPiece = SlidingPiece {
    deltas: [(1, 0), (0, -1), (-1, 0), (0, 1)],
};

const BISHOP: SlidingPiece = SlidingPiece {
    deltas: [(1, 1), (1, -1), (-1, -1), (-1, 1)],
};

struct MagicEntry {
    mask: Bitboard,
    magic: u64,
    shift: u8,
}

fn magic_index(entry: &MagicEntry, blockers: Bitboard) -> usize {
    let blockers = blockers & entry.mask;
    let hash = blockers.0.wrapping_mul(entry.magic);
    (hash >> entry.shift) as usize
}

// Given a sliding piece and a square, finds a magic number that
// perfectly maps input blockers into its solution in a hash table
fn find_magic(
    slider: &SlidingPiece,
    square: Bitboard,
    index_bits: u8,
) -> (MagicEntry, Vec<Bitboard>) {
    let mask = slider.relevant_blockers(square);
    let shift = 64 - index_bits;

    loop {
        // Magics require a low number of active bits, so we AND
        // by two more random values to cut down on the bits set.
        let magic = generate_random_u64() & generate_random_u64() & generate_random_u64();
        let magic_entry = MagicEntry { mask, magic, shift };
        if let Ok(table) = try_make_table(slider, square, &magic_entry) {
            return (magic_entry, table);
        }
    }
}

struct TableFillError;

// Attempt to fill in a hash table using a magic number.
// Fails if there are any non-constructive collisions.
fn try_make_table(
    slider: &SlidingPiece,
    square: Bitboard,
    magic_entry: &MagicEntry,
) -> Result<Vec<Bitboard>, TableFillError> {
    let index_bits = 64 - magic_entry.shift;
    let mut table = vec![Bitboard::EMPTY; 1 << index_bits];
    // Iterate all configurations of blockers
    let mut blockers = Bitboard::EMPTY;
    loop {
        let moves = slider.moves(square, blockers);
        let table_entry = &mut table[magic_index(magic_entry, blockers)];
        if table_entry.is_empty() {
            // Write to empty slot
            *table_entry = moves;
        } else if *table_entry != moves {
            // Having two different move sets in the same slot is a hash collision
            return Err(TableFillError);
        }

        // Carry-Rippler trick that enumerates all subsets of the mask, getting us all blockers.
        // https://www.chessprogramming.org/Traversing_Subsets_of_a_Set#All_Subsets_of_any_Set
        blockers.0 = blockers.0.wrapping_sub(magic_entry.mask.0) & magic_entry.mask.0;
        if blockers.is_empty() {
            // Finished enumerating all blocker configurations
            break;
        }
    }
    Ok(table)
}

fn find_and_write_magics(
    slider: &SlidingPiece,
    slider_name: &str,
    out: &mut BufWriter<File>,
) -> std::io::Result<()> {
    writeln!(out,
        "pub const {}_MAGICS: &[MagicEntry; 64] = &[",
        slider_name
    )?;
    let mut total_table_size = 0;
    for square_i in 0..64 {
        let square = Bitboard(1) << square_i;
        debug!("Finding magic for square: {:?}", square);
        let index_bits = slider.relevant_blockers(square).popcnt() as u8;
        debug!("Index bits: {}", index_bits);
        let (entry, table) = find_magic(slider, square, index_bits);
        // In the final move generator, each table is concatenated into one contiguous table
        // for convenience, so an offset is added to denote the start of each segment.
        writeln!(out,
            "    MagicEntry {{ mask: 0x{:016X}, magic: 0x{:016X}, shift: {}, offset: {} }},",
            entry.mask.0, entry.magic, entry.shift, total_table_size
        )?;
        total_table_size += table.len();
    }
    writeln!(out,"];")?;
    writeln!(out,
        "pub const {}_TABLE_SIZE: usize = {};",
        slider_name, total_table_size
    )?;
    Ok(())
}

pub fn find_and_write_all_magics(out: &mut BufWriter<File>) -> std::io::Result<()> {
    debug!("Finding magics...");
    find_and_write_magics(&ROOK, "ROOK", out)?;
    debug!("Found rook magics!");
    find_and_write_magics(&BISHOP, "BISHOP", out)?;
    debug!("Found bishop magics!");
    Ok(())
}
