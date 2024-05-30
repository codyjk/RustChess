use std::fs::File;
use std::io::{BufWriter, Write};

use crate::random_number_generator::generate_random_u64;

// Constants for Zobrist hashing
const PIECES: [&str; 6] = ["pawn", "rook", "knight", "bishop", "king", "queen"];
const SQUARES: usize = 64;

pub fn write_zobrist_tables(out: &mut BufWriter<File>) -> std::io::Result<()> {
    // Generate ZOBRIST_PIECES_TABLE
    let mut zobrist_table = [[[0u64; 2]; SQUARES]; PIECES.len()];
    for piece in 0..PIECES.len() {
        for square in 0..SQUARES {
            for color in 0..2 {
                zobrist_table[piece][square][color] = generate_random_u64();
            }
        }
    }

    // Generate ZOBRIST_CASTLING_RIGHTS_TABLE
    let mut zobrist_castling_rights = [0u64; 16];
    for i in 0..16 {
        zobrist_castling_rights[i] = generate_random_u64();
    }

    // Generate ZOBRIST_EN_PASSANT_TABLE
    let mut zobrist_en_passant = [0u64; SQUARES];
    for i in 0..SQUARES {
        zobrist_en_passant[i] = generate_random_u64();
    }

    // Write the generated values into a format that can be used in a Rust module
    writeln!(out, "#[rustfmt::skip]")?;
    writeln!(out, "pub const ZOBRIST_PIECES_TABLE: [[[u64; 2]; 64]; 6] = [")?;
    for piece_index in 0..PIECES.len() {
        writeln!(out, "    [  // {}", PIECES[piece_index])?;
        for square_index in 0..SQUARES {
            writeln!(
                out,
                "        [{}, {}],  // Square {}",
                zobrist_table[piece_index][square_index][0],
                zobrist_table[piece_index][square_index][1],
                square_index
            )?;
        }
        writeln!(out, "    ],")?;
    }
    writeln!(out, "];")?;

    writeln!(out, "\n#[rustfmt::skip]")?;
    writeln!(out, "pub const ZOBRIST_CASTLING_RIGHTS_TABLE: [u64; 16] = [")?;
    for rights in zobrist_castling_rights.iter() {
        writeln!(out, "    {},", rights)?;
    }
    writeln!(out, "];")?;

    writeln!(out, "\n#[rustfmt::skip]")?;
    writeln!(out, "pub const ZOBRIST_EN_PASSANT_TABLE: [u64; 64] = [")?;
    for ep_square in zobrist_en_passant.iter() {
        writeln!(out, "    {},", ep_square)?;
    }
    writeln!(out, "];")?;

    Ok(())
}
