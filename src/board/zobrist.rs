use super::bitboard::{A_FILE, B_FILE, C_FILE, D_FILE, E_FILE, F_FILE, G_FILE, H_FILE};
use super::color::Color;
use super::piece::Piece;
use super::square::{self, ORDERED};
use super::{
    Board, BLACK_KINGSIDE_RIGHTS, BLACK_QUEENSIDE_RIGHTS, WHITE_KINGSIDE_RIGHTS,
    WHITE_QUEENSIDE_RIGHTS,
};
use rand::Rng;

// Zobrist board hashing
// * One number for each piece at each square ( 12 * 64 )
// * One number to indicate the side to move is black
// * Four numbers to indicate the castling rights, though usually 16 (2^4) are used for speed
// * Eight numbers to indicate the file of a valid En passant square, if any
// Total numbers:  12 * 64 + 1 + 4 + 8 = 781
//
// To get the zobirst hash for any position:
// [Hash for White Rook on a1] xor [Hash for White Knight on b1] xor [Hash for White Bishop on c1] xor ... ( all pieces )
// ... xor [Hash for White king castling] xor [Hash for White queeb castling] xor ... ( all castling rights )

pub struct Zobrist {
    numbers: [u64; 781],
}

impl Zobrist {
    pub fn new() -> Self {
        Self {
            numbers: init_rand_numbers(),
        }
    }

    pub fn hash(&self, board: &Board) -> (u64, u64) {
        let mut position_hash = 0;

        for &sq in &ORDERED {
            let (p, c) = match board.get(sq) {
                Some((p, c)) => (p, c),
                None => continue,
            };

            position_hash ^= self.piece_square_num(p, c, sq);
        }

        let mut boardstate_hash = position_hash;

        boardstate_hash ^= self.current_turn_num(board.turn);
        boardstate_hash ^= self.castling_rights_num(board.peek_castle_rights());
        boardstate_hash ^= self.en_passant_num(board.peek_en_passant_target());

        (position_hash, boardstate_hash)
    }

    // nums 0..768: piece squares
    fn piece_square_num(&self, piece: Piece, color: Color, sq: u64) -> u64 {
        self.numbers[index_of(piece, color, sq) as usize]
    }

    // nums 768..769: current turn
    fn current_turn_num(&self, color: Color) -> u64 {
        match color {
            Color::Black => self.numbers[768],
            Color::White => 0,
        }
    }

    // nums 769..773: castling rights
    fn castling_rights_num(&self, castling_rights: u8) -> u64 {
        if castling_rights == 0 {
            return 0;
        }

        let mut num = 0;
        if castling_rights & WHITE_KINGSIDE_RIGHTS > 0 {
            num ^= self.numbers[769]
        }
        if castling_rights & BLACK_KINGSIDE_RIGHTS > 0 {
            num ^= self.numbers[770]
        }
        if castling_rights & WHITE_QUEENSIDE_RIGHTS > 0 {
            num ^= self.numbers[771]
        }
        if castling_rights & BLACK_QUEENSIDE_RIGHTS > 0 {
            num ^= self.numbers[772]
        }
        num
    }

    // nums 773..781: file of en passant target (if any)
    fn en_passant_num(&self, en_passant_target: u64) -> u64 {
        if en_passant_target == 0 {
            return 0;
        } else if en_passant_target & A_FILE > 0 {
            return self.numbers[773];
        } else if en_passant_target & B_FILE > 0 {
            return self.numbers[774];
        } else if en_passant_target & C_FILE > 0 {
            return self.numbers[775];
        } else if en_passant_target & D_FILE > 0 {
            return self.numbers[776];
        } else if en_passant_target & E_FILE > 0 {
            return self.numbers[777];
        } else if en_passant_target & F_FILE > 0 {
            return self.numbers[778];
        } else if en_passant_target & G_FILE > 0 {
            return self.numbers[779];
        } else if en_passant_target & H_FILE > 0 {
            return self.numbers[780];
        } else {
            // should never happen
            return 0;
        }
    }
}

fn init_rand_numbers() -> [u64; 781] {
    let mut nums = [0; 781];
    let mut rng = rand::thread_rng();

    for i in 0..781 {
        nums[i] = rng.gen();
    }

    nums
}

fn index_of(piece: Piece, color: Color, sq: u64) -> u16 {
    let x = nth_bit(piece as u64) - 1;
    let y = nth_bit(color as u64) - 1;
    let z = nth_bit(square::assert(sq)) - 1;

    let x_max = 6;
    let y_max = 2;

    // we are basically mapping a three-dimensional coordinate to a flat array.
    // (x, y, z), where 0 <= x <= 5, 0 <= y <= 1, and 0 <= z <= 64
    (z * x_max * y_max) + (y * x_max) + x
}

fn nth_bit(x: u64) -> u16 {
    let mut bits = x;
    let mut n = 0;
    while bits > 0 {
        bits = bits >> 1;
        n += 1;
    }

    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::piece::ALL_PIECES;
    use crate::moves::chess_move::ChessMove;
    use std::collections::HashSet;

    #[test]
    fn test_all_nums_random() {
        let zob = Zobrist::new();
        let mut set = HashSet::new();

        let mut i = 0;
        for num in &zob.numbers {
            assert!(
                !set.contains(num),
                "zobrist number {} ({}) is in the set",
                i,
                num
            );
            set.insert(num);
            i += 1;
        }
    }

    #[test]
    fn test_all_pieces_resolve_to_unique_num() {
        let zob = Zobrist::new();
        let mut seen_nums = HashSet::new();
        let mut seen_indexes = HashSet::new();

        for p in &ALL_PIECES {
            for c in &[Color::Black, Color::White] {
                for sq in &ORDERED {
                    let i = index_of(*p, *c, *sq);
                    let num = zob.numbers[i as usize];
                    assert!(
                        !seen_nums.contains(&num),
                        "seen_num={}, i={}, p={}, c={}, sq={}",
                        num,
                        i,
                        p,
                        c,
                        square::to_algebraic(*sq)
                    );
                    seen_nums.insert(num);
                    seen_indexes.insert(i);
                }
            }
        }

        for i in 0..768 {
            assert!(seen_indexes.contains(&i));
        }
    }

    #[test]
    fn test_zobrist_num_changes() {
        let zob = Zobrist::new();

        let mut board = Board::starting_position();
        let (za1, za2) = zob.hash(&board);

        board
            .apply(ChessMove::new(square::E2, square::E4, None))
            .unwrap();
        let (zb1, zb2) = zob.hash(&board);

        assert_ne!(za1, zb1);
        assert_ne!(za2, zb2);
    }
}
