#!/usr/bin/env python3

import random

# Constants for Zobrist hashing
PIECES = ['pawn', 'rook', 'knight', 'bishop', 'king', 'queen']
COLORS = ['white', 'black']
SQUARES = 64

# Initialize seed for reproducibility
random.seed(1337)

# Function to generate a random 64-bit integer
def generate_random_64bit():
    return random.getrandbits(64)

# Generate ZOBRIST_PIECES_TABLE
zobrist_table = [[[generate_random_64bit() for _ in range(2)] for _ in range(SQUARES)] for _ in range(len(PIECES))]

# Generate ZOBRIST_CASTLING_RIGHTS_TABLE_TABLE
zobrist_castling_rights = [generate_random_64bit() for _ in range(16)]  # 16 possible castling rights combinations

# Generate ZOBRIST_EN_PASSANT_TABLE_TABLE
zobrist_en_passant = [generate_random_64bit() for _ in range(SQUARES)]  # One for each square

# Print the generated values into a format that can be used in a rust module
print("#[rustfmt::skip]");
print("pub const ZOBRIST_PIECES_TABLE: [[[u64; 2]; 64]; 6] = [")
for piece_index, piece in enumerate(zobrist_table):
    print(f"    [  // {PIECES[piece_index]}")
    for square_index, square in enumerate(piece):
        print(f"        [{square[0]}, {square[1]}],  // Square {square_index}")
    print("    ],")
print("];")


print("\n#[rustfmt::skip]");
print("pub const ZOBRIST_CASTLING_RIGHTS_TABLE_TABLE: [u64; 16] = [")
for rights in zobrist_castling_rights:
    print(f"    {rights},")
print("];")

print("\n#[rustfmt::skip]");
print("pub const ZOBRIST_EN_PASSANT_TABLE_TABLE: [u64; 64] = [")
for ep_square in zobrist_en_passant:
    print(f"    {ep_square},")
print("];")
