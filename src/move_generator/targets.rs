//! Move and attack target generation for chess pieces.
//!
//! **Performance optimizations:**
//! - Iterate only occupied squares instead of all 64 squares for better cache locality
//! - Bitboard shifts for pawn moves instead of index arithmetic
//! - Magic table lookups with `#[inline]` attributes for better inlining
//! - Direct bitboard accumulation for attack targets without intermediate SmallVec allocations
//! - Simplified bit operations (AND-NOT instead of XOR combinations)
//! - Batch processing of pawn attacks using parallel bitboard operations

use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::Board;
use common::bitboard::{Bitboard, Square};
use smallvec::{smallvec, SmallVec};

use super::magic_table::MagicTable;

/// A `PieceTarget` is a tuple of a piece's square and the squares it can move to.
pub type PieceTarget = (Square, Bitboard); // (piece_square, targets)

/// A list of PieceTargets that is optimized for small sizes.
/// Similar to `ChessMoveList`, see the documentation for reasoning around performance.
pub type PieceTargetList = SmallVec<[PieceTarget; 16]>;

/// The `Targets` struct is responsible for generating move and attack targets for each piece on the board.
/// It uses precomputed tables for the knight and king pieces, and generates targets for sliding pieces
/// (rooks, bishops, queens) using magic bitboards.
#[derive(Clone)]
pub struct Targets {
    kings: [Bitboard; 64],
    knights: [Bitboard; 64],
    magic_table: MagicTable,
}

impl Default for Targets {
    fn default() -> Self {
        let magic_table = MagicTable::new();

        Self {
            kings: generate_king_targets_table(),
            knights: generate_knight_targets_table(),
            magic_table,
        }
    }
}

impl Targets {
    pub fn generate_attack_targets(&self, board: &Board, color: Color) -> Bitboard {
        let mut attack_targets = Bitboard::EMPTY;

        attack_targets |= generate_pawn_attack_targets_bitboard(board, color);
        attack_targets |= self.generate_sliding_targets_bitboard(board, color);
        attack_targets |=
            self.generate_targets_from_precomputed_tables_bitboard(board, color, Piece::Knight);
        attack_targets |=
            self.generate_targets_from_precomputed_tables_bitboard(board, color, Piece::King);

        attack_targets
    }

    pub fn generate_targets_from_precomputed_tables(
        &self,
        piece_targets: &mut PieceTargetList,
        board: &Board,
        color: Color,
        piece: Piece,
    ) {
        // Optimized: Iterate only occupied squares instead of all 64 squares
        let mut pieces = board.pieces(color).locate(piece);
        let occupied = board.pieces(color).occupied();

        while !pieces.is_empty() {
            let square = pieces.pop_lsb().to_square();

            let candidates = self.get_precomputed_targets(square, piece) & !occupied;
            if !candidates.is_empty() {
                piece_targets.push((square, candidates));
            }
        }
    }

    pub fn generate_sliding_targets(
        &self,
        piece_targets: &mut PieceTargetList,
        board: &Board,
        color: Color,
    ) {
        let occupied = board.occupied();
        let own_occupied = board.pieces(color).occupied();

        // Iterate only rooks (instead of all 64 squares)
        let mut rooks = board.pieces(color).locate(Piece::Rook);
        while !rooks.is_empty() {
            let square = rooks.pop_lsb().to_square();
            let targets_including_own_pieces = self.magic_table.get_rook_targets(square, occupied);
            let target_squares = targets_including_own_pieces & !own_occupied;
            piece_targets.push((square, target_squares));
        }

        // Iterate only bishops (instead of all 64 squares)
        let mut bishops = board.pieces(color).locate(Piece::Bishop);
        while !bishops.is_empty() {
            let square = bishops.pop_lsb().to_square();
            let targets_including_own_pieces =
                self.magic_table.get_bishop_targets(square, occupied);
            let target_squares = targets_including_own_pieces & !own_occupied;
            piece_targets.push((square, target_squares));
        }

        // Iterate only queens (instead of all 64 squares)
        let mut queens = board.pieces(color).locate(Piece::Queen);
        while !queens.is_empty() {
            let square = queens.pop_lsb().to_square();
            let targets_including_own_pieces = self.magic_table.get_rook_targets(square, occupied)
                | self.magic_table.get_bishop_targets(square, occupied);
            let target_squares = targets_including_own_pieces & !own_occupied;
            piece_targets.push((square, target_squares));
        }
    }

    fn get_precomputed_targets(&self, square: Square, piece: Piece) -> Bitboard {
        match piece {
            Piece::Knight => self.knights[square.index() as usize],
            Piece::King => self.kings[square.index() as usize],
            _ => panic!("invalid piece type for precomputed targets: {}", piece),
        }
    }

    fn generate_sliding_targets_bitboard(&self, board: &Board, color: Color) -> Bitboard {
        let occupied = board.occupied();
        let own_occupied = board.pieces(color).occupied();
        let mut attack_targets = Bitboard::EMPTY;

        let mut rooks = board.pieces(color).locate(Piece::Rook);
        while !rooks.is_empty() {
            let square = rooks.pop_lsb().to_square();
            let targets_including_own_pieces = self.magic_table.get_rook_targets(square, occupied);
            attack_targets |= targets_including_own_pieces & !own_occupied;
        }

        let mut bishops = board.pieces(color).locate(Piece::Bishop);
        while !bishops.is_empty() {
            let square = bishops.pop_lsb().to_square();
            let targets_including_own_pieces =
                self.magic_table.get_bishop_targets(square, occupied);
            attack_targets |= targets_including_own_pieces & !own_occupied;
        }

        let mut queens = board.pieces(color).locate(Piece::Queen);
        while !queens.is_empty() {
            let square = queens.pop_lsb().to_square();
            let targets_including_own_pieces = self.magic_table.get_rook_targets(square, occupied)
                | self.magic_table.get_bishop_targets(square, occupied);
            attack_targets |= targets_including_own_pieces & !own_occupied;
        }

        attack_targets
    }

    fn generate_targets_from_precomputed_tables_bitboard(
        &self,
        board: &Board,
        color: Color,
        piece: Piece,
    ) -> Bitboard {
        let mut pieces = board.pieces(color).locate(piece);
        let occupied = board.pieces(color).occupied();
        let mut attack_targets = Bitboard::EMPTY;

        while !pieces.is_empty() {
            let square = pieces.pop_lsb().to_square();
            let candidates = self.get_precomputed_targets(square, piece) & !occupied;
            attack_targets |= candidates;
        }

        attack_targets
    }
}

pub fn generate_pawn_move_targets(board: &Board, color: Color) -> PieceTargetList {
    let mut piece_targets: PieceTargetList = smallvec![];

    let mut pawns = board.pieces(color).locate(Piece::Pawn);
    let occupied = board.occupied();

    // Optimized: Use bitboard shifts instead of index arithmetic for better performance
    while !pawns.is_empty() {
        let pawn = pawns.pop_lsb();
        let mut targets = Bitboard::EMPTY;

        // Single move forward using bitboard shift (more efficient than index arithmetic)
        let single_target = match color {
            Color::White => pawn << 8,
            Color::Black => pawn >> 8,
        };

        // Check if single move square is empty and within board bounds
        if !single_target.is_empty() && !single_target.overlaps(occupied) {
            targets |= single_target;

            // Check for double move from starting rank
            // Double move is only valid if both the single and double move squares are empty
            let is_starting_rank = match color {
                Color::White => pawn.overlaps(Bitboard::RANK_2),
                Color::Black => pawn.overlaps(Bitboard::RANK_7),
            };
            if is_starting_rank {
                let double_target = match color {
                    Color::White => pawn << 16,
                    Color::Black => pawn >> 16,
                };
                // Double move square must be empty and within bounds
                if !double_target.is_empty() && !double_target.overlaps(occupied) {
                    targets |= double_target;
                }
            }
        }

        if !targets.is_empty() {
            piece_targets.push((pawn.to_square(), targets));
        }
    }

    piece_targets
}

// having a separate function for generating pawn attacks is useful for generating
// attack maps. this separates the attacked squares from the ones with enemy pieces
// on them
pub fn generate_pawn_attack_targets(
    piece_targets: &mut PieceTargetList,
    board: &Board,
    color: Color,
) {
    let mut pawns = board.pieces(color).locate(Piece::Pawn);

    // Optimized: Use bitboard shifts instead of index arithmetic for better performance
    // This matches the approach used in generate_en_passant_moves
    while !pawns.is_empty() {
        let pawn = pawns.pop_lsb();
        let mut targets = Bitboard::EMPTY;

        match color {
            Color::White => {
                // Northeast attack (west): shift left 9 squares, exclude A file
                let attack_west = (pawn << 9) & !Bitboard::A_FILE;
                if !attack_west.is_empty() {
                    targets |= attack_west;
                }
                // Northwest attack (east): shift left 7 squares, exclude H file
                let attack_east = (pawn << 7) & !Bitboard::H_FILE;
                if !attack_east.is_empty() {
                    targets |= attack_east;
                }
            }
            Color::Black => {
                // Southeast attack (west): shift right 7 squares, exclude A file
                let attack_west = (pawn >> 7) & !Bitboard::A_FILE;
                if !attack_west.is_empty() {
                    targets |= attack_west;
                }
                // Southwest attack (east): shift right 9 squares, exclude H file
                let attack_east = (pawn >> 9) & !Bitboard::H_FILE;
                if !attack_east.is_empty() {
                    targets |= attack_east;
                }
            }
        }

        piece_targets.push((pawn.to_square(), targets));
    }
}

pub fn generate_pawn_attack_targets_bitboard(board: &Board, color: Color) -> Bitboard {
    let pawns = board.pieces(color).locate(Piece::Pawn);

    match color {
        Color::White => {
            let attacks_west = (pawns << 9) & !Bitboard::A_FILE;
            let attacks_east = (pawns << 7) & !Bitboard::H_FILE;
            attacks_west | attacks_east
        }
        Color::Black => {
            let attacks_west = (pawns >> 7) & !Bitboard::A_FILE;
            let attacks_east = (pawns >> 9) & !Bitboard::H_FILE;
            attacks_west | attacks_east
        }
    }
}

pub fn generate_knight_targets_table() -> [Bitboard; 64] {
    let mut table = [Bitboard::EMPTY; 64];

    for square in Square::ALL {
        let knight = Bitboard(1 << square.index());

        // nne = north-north-east, nee = north-east-east, etc..
        let move_nne = knight << 17 & !Bitboard::A_FILE;
        let move_nee = knight << 10 & !Bitboard::A_FILE & !Bitboard::B_FILE;
        let move_see = knight >> 6 & !Bitboard::A_FILE & !Bitboard::B_FILE;
        let move_sse = knight >> 15 & !Bitboard::A_FILE;
        let move_nnw = knight << 15 & !Bitboard::H_FILE;
        let move_nww = knight << 6 & !Bitboard::G_FILE & !Bitboard::H_FILE;
        let move_sww = knight >> 10 & !Bitboard::G_FILE & !Bitboard::H_FILE;
        let move_ssw = knight >> 17 & !Bitboard::H_FILE;

        table[square.index() as usize] =
            move_nne | move_nee | move_see | move_sse | move_nnw | move_nww | move_sww | move_ssw;
    }

    table
}

pub fn generate_king_targets_table() -> [Bitboard; 64] {
    let mut table = [Bitboard::EMPTY; 64];

    for square in Square::ALL {
        let king = Bitboard(1 << square.index());
        let mut targets = Bitboard::EMPTY;

        // shift the king's position. in the event that it falls off of the boundary,
        // we want to negate the rank/file where the king would fall.
        targets |= (king << 9) & !Bitboard::RANK_1 & !Bitboard::A_FILE; // northeast
        targets |= (king << 8) & !Bitboard::RANK_1; // north
        targets |= (king << 7) & !Bitboard::RANK_1 & !Bitboard::H_FILE; // northwest

        targets |= (king >> 7) & !Bitboard::RANK_8 & !Bitboard::A_FILE; // southeast
        targets |= (king >> 8) & !Bitboard::RANK_8; // south
        targets |= (king >> 9) & !Bitboard::RANK_8 & !Bitboard::H_FILE; // southwest

        targets |= (king << 1) & !Bitboard::A_FILE; // east
        targets |= (king >> 1) & !Bitboard::H_FILE; // west

        table[square.index() as usize] = targets;
    }

    table
}

#[cfg(test)]
mod tests {
    use crate::chess_move::chess_move_effect::ChessMoveEffect;
    use common::bitboard::*;

    use super::*;
    use crate::chess_move::chess_move::ChessMove;
    use crate::chess_move::standard::StandardChessMove;
    use crate::{chess_position, std_move};

    #[test]
    fn test_generate_attack_targets_1() {
        let targets = Targets::default();
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

        let expected_white_targets = Bitboard::EMPTY
            // pawn
            | B5
            // rook
            | B2
            | B3
            | B4
            | B5
            | (Bitboard::RANK_1 ^ B1)
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
            | A1;
        let white_targets = targets.generate_attack_targets(&board, Color::White);
        println!("expected white targets:\n{}", expected_white_targets,);
        println!("actual white targets:\n{}", white_targets);
        assert_eq!(expected_white_targets, white_targets);

        let expected_black_targets = Bitboard::EMPTY
            // pawn
            | A4
            | C4
            // king
            | G1
            | G2
            | H2;
        let black_targets = targets.generate_attack_targets(&board, Color::Black);
        println!("expected black targets:\n{}", expected_black_targets);
        println!("actual black targets:\n{}", black_targets);
        assert_eq!(expected_black_targets, black_targets);
    }

    #[test]
    pub fn test_generate_attack_targets_2() {
        let targets = Targets::default();
        let mut board = Board::default();
        let moves = [
            std_move!(E2, E4),
            std_move!(F7, F5),
            std_move!(D1, H5),
            std_move!(G7, G6),
        ];

        for m in moves.iter() {
            m.apply(&mut board).unwrap();
            board.toggle_turn();
        }
        println!("Testing board:\n{}", board);

        let expected_white_targets = Bitboard::EMPTY
            // knights
            | Bitboard::RANK_3
            // forward pawn
            | D5
            | F5
            // queen - north
            | H6
            | H7
            // queen - nortwest
            | G6
            // queen - west
            | G5
            | F5
            // queen - southwest
            | G4
            | F3
            | E2
            | D1
            // queen - south
            | H4
            | H3
            // bishop
            | E2
            | D3
            | C4
            | B5
            | A6
            // king
            | D1
            | E2;

        let white_targets = targets.generate_attack_targets(&board, Color::White);
        println!("expected white targets:\n{}", expected_white_targets,);
        println!("actual white targets:\n{}", white_targets);
        assert_eq!(expected_white_targets, white_targets);
    }
}
