use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::Board;
use common::bitboard::{Bitboard, Square, ORDERED_SQUARES};
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
        let mut piece_targets: PieceTargetList = smallvec![];
        let mut attack_targets = Bitboard::EMPTY;

        generate_pawn_attack_targets(&mut piece_targets, board, color);
        self.generate_sliding_targets(&mut piece_targets, board, color);
        self.generate_targets_from_precomputed_tables(
            &mut piece_targets,
            board,
            color,
            Piece::Knight,
        );
        self.generate_targets_from_precomputed_tables(
            &mut piece_targets,
            board,
            color,
            Piece::King,
        );

        for (_piece, targets) in piece_targets {
            attack_targets |= targets;
        }

        attack_targets
    }

    pub fn generate_targets_from_precomputed_tables(
        &self,
        piece_targets: &mut PieceTargetList,
        board: &Board,
        color: Color,
        piece: Piece,
    ) {
        let pieces = board.pieces(color).locate(piece);
        let occupied = board.pieces(color).occupied();

        for sq in ORDERED_SQUARES {
            if !sq.overlaps(pieces) {
                continue;
            }

            let candidates = self.get_precomputed_targets(sq, piece) & !occupied;
            if candidates.is_empty() {
                continue;
            }

            piece_targets.push((sq, candidates));
        }
    }

    pub fn generate_sliding_targets(
        &self,
        piece_targets: &mut PieceTargetList,
        board: &Board,
        color: Color,
    ) {
        let occupied = board.occupied();

        for x in 0..64 {
            let square = Square::new(x);
            let piece = match board.pieces(color).get(square) {
                Some(p) => p,
                None => continue,
            };

            let targets_including_own_pieces = match piece {
                Piece::Rook => self.magic_table.get_rook_targets(square, occupied),
                Piece::Bishop => self.magic_table.get_bishop_targets(square, occupied),
                Piece::Queen => {
                    self.magic_table.get_rook_targets(square, occupied)
                        | self.magic_table.get_bishop_targets(square, occupied)
                }
                _ => continue,
            };
            let target_squares = targets_including_own_pieces
                ^ (board.pieces(color).occupied() & targets_including_own_pieces);
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
}

pub fn generate_pawn_move_targets(board: &Board, color: Color) -> PieceTargetList {
    let mut piece_targets: PieceTargetList = smallvec![];

    let mut pawns = board.pieces(color).locate(Piece::Pawn);
    let occupied = board.occupied();

    while !pawns.is_empty() {
        let pawn_sq = pawns.pop_lsb().to_square();
        let mut targets = Bitboard::EMPTY;

        let (single_offset, double_rank) = match color {
            Color::White => (8u8, 1u8), // +8 for single move, rank 1 (0-indexed) for double move
            Color::Black => (8u8, 6u8), // -8 for single move, rank 6 for double move
        };

        let pawn_idx = pawn_sq.index() as u16;

        // Single move forward
        let single_target_idx = match color {
            Color::White => pawn_idx + single_offset as u16,
            Color::Black => pawn_idx.wrapping_sub(single_offset as u16),
        };

        if single_target_idx < 64 {
            let single_target = Square::new(single_target_idx as u8);
            if !single_target.overlaps(occupied) {
                targets |= single_target.to_bitboard();

                // Check for double move from starting rank
                if pawn_sq.rank() == double_rank {
                    let double_target_idx = match color {
                        Color::White => single_target_idx + single_offset as u16,
                        Color::Black => single_target_idx.wrapping_sub(single_offset as u16),
                    };
                    if double_target_idx < 64 {
                        let double_target = Square::new(double_target_idx as u8);
                        if !double_target.overlaps(occupied) {
                            targets |= double_target.to_bitboard();
                        }
                    }
                }
            }
        }

        if !targets.is_empty() {
            piece_targets.push((pawn_sq, targets));
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

    while !pawns.is_empty() {
        let pawn_sq = pawns.pop_lsb().to_square();
        let mut targets = Bitboard::EMPTY;

        let pawn_idx = pawn_sq.index() as u16;
        let pawn_file = pawn_sq.file();

        match color {
            Color::White => {
                // Northeast attack (west)
                if pawn_file < 7 {
                    let target_idx = pawn_idx + 9;
                    if target_idx < 64 {
                        targets |= Square::new(target_idx as u8).to_bitboard();
                    }
                }
                // Northwest attack (east)
                if pawn_file > 0 {
                    let target_idx = pawn_idx + 7;
                    if target_idx < 64 {
                        targets |= Square::new(target_idx as u8).to_bitboard();
                    }
                }
            }
            Color::Black => {
                // Southeast attack (west)
                if pawn_file < 7 {
                    let target_idx = pawn_idx.wrapping_sub(7);
                    if target_idx < 64 {
                        targets |= Square::new(target_idx as u8).to_bitboard();
                    }
                }
                // Southwest attack (east)
                if pawn_file > 0 {
                    let target_idx = pawn_idx.wrapping_sub(9);
                    if target_idx < 64 {
                        targets |= Square::new(target_idx as u8).to_bitboard();
                    }
                }
            }
        }

        piece_targets.push((pawn_sq, targets));
    }
}

pub fn generate_knight_targets_table() -> [Bitboard; 64] {
    let mut table = [Bitboard::EMPTY; 64];

    for square in Square::ALL {
        let knight = square.to_bitboard();

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
        let king = square.to_bitboard();
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
