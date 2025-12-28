use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::Board;
use common::bitboard::bitboard::Bitboard;
use common::bitboard::square::ORDERED_SQUARES;
use smallvec::{smallvec, SmallVec};

use super::magic_table::MagicTable;

/// A `PieceTarget` is a tuple of a piece's square and the squares it can move to.
pub type PieceTarget = (Bitboard, Bitboard); // (piece_square, targets)

/// A list of PieceTargets that is optimized for small sizes.
/// Similar to `ChessMoveList`, see the documentation for reasoning around performance.
pub type PieceTargetList = SmallVec<[PieceTarget; 16]>;

/// The `Targets` struct is responsible for generating move and attack targets for each piece on the board.
/// It uses precomputed tables for the knight and king pieces, and generates targets for sliding pieces
/// (rooks, bishops, queens) using magic bitboards.
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
            if !pieces.overlaps(sq) {
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
            let square = Bitboard(1 << x);
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

    fn get_precomputed_targets(&self, square: Bitboard, piece: Piece) -> Bitboard {
        let square_i = square.trailing_zeros() as usize;
        match piece {
            Piece::Knight => self.knights[square_i],
            Piece::King => self.kings[square_i],
            _ => panic!("invalid piece type for precomputed targets: {}", piece),
        }
    }
}

pub fn generate_pawn_move_targets(board: &Board, color: Color) -> PieceTargetList {
    let mut piece_targets: PieceTargetList = smallvec![];

    let pawns = board.pieces(color).locate(Piece::Pawn);
    let occupied = board.occupied();

    let single_move_targets = match color {
        Color::White => pawns << 8, // move 1 rank up the board
        Color::Black => pawns >> 8, // move 1 rank down the board
    };
    let double_move_targets = match color {
        Color::White => Bitboard::RANK_4,
        Color::Black => Bitboard::RANK_5,
    };
    let move_targets = (single_move_targets | double_move_targets) & !occupied;

    for x in 0..64 {
        let pawn = Bitboard(1 << x);
        if !pawns.overlaps(pawn) {
            continue;
        }
        let mut targets = Bitboard::EMPTY;

        let single_move = match color {
            Color::White => pawn << 8,
            Color::Black => pawn >> 8,
        };

        if single_move.overlaps(occupied) {
            // pawn is blocked and can make no moves
            continue;
        }

        let double_move = match color {
            Color::White => single_move << 8,
            Color::Black => single_move >> 8,
        };

        targets |= single_move & move_targets;
        targets |= double_move & move_targets;

        if targets.is_empty() {
            continue;
        }

        piece_targets.push((pawn, targets));
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
    let pawns = board.pieces(color).locate(Piece::Pawn);

    for x in 0..64 {
        let pawn = Bitboard(1 << x);
        if !pawns.overlaps(pawn) {
            continue;
        }

        let attack_west = match color {
            Color::White => (pawn << 9) & !Bitboard::A_FILE,
            Color::Black => (pawn >> 7) & !Bitboard::A_FILE,
        };

        let attack_east = match color {
            Color::White => (pawn << 7) & !Bitboard::H_FILE,
            Color::Black => (pawn >> 9) & !Bitboard::H_FILE,
        };

        let targets = attack_east | attack_west;
        piece_targets.push((pawn, targets));
    }
}

pub fn generate_knight_targets_table() -> [Bitboard; 64] {
    let mut table = [Bitboard::EMPTY; 64];

    for (square_i, targets) in table.iter_mut().enumerate() {
        let knight = Bitboard(1 << square_i);

        // nne = north-north-east, nee = north-east-east, etc..
        let move_nne = knight << 17 & !Bitboard::A_FILE;
        let move_nee = knight << 10 & !Bitboard::A_FILE & !Bitboard::B_FILE;
        let move_see = knight >> 6 & !Bitboard::A_FILE & !Bitboard::B_FILE;
        let move_sse = knight >> 15 & !Bitboard::A_FILE;
        let move_nnw = knight << 15 & !Bitboard::H_FILE;
        let move_nww = knight << 6 & !Bitboard::G_FILE & !Bitboard::H_FILE;
        let move_sww = knight >> 10 & !Bitboard::G_FILE & !Bitboard::H_FILE;
        let move_ssw = knight >> 17 & !Bitboard::H_FILE;

        *targets =
            move_nne | move_nee | move_see | move_sse | move_nnw | move_nww | move_sww | move_ssw;
    }

    table
}

pub fn generate_king_targets_table() -> [Bitboard; 64] {
    let mut table = [Bitboard::EMPTY; 64];

    for (square_i, targets) in table.iter_mut().enumerate() {
        let king = Bitboard(1 << square_i);

        // shift the king's position. in the event that it falls off of the boundary,
        // we want to negate the rank/file where the king would fall.
        *targets |= (king << 9) & !Bitboard::RANK_1 & !Bitboard::A_FILE; // northeast
        *targets |= (king << 8) & !Bitboard::RANK_1; // north
        *targets |= (king << 7) & !Bitboard::RANK_1 & !Bitboard::H_FILE; // northwest

        *targets |= (king >> 7) & !Bitboard::RANK_8 & !Bitboard::A_FILE; // southeast
        *targets |= (king >> 8) & !Bitboard::RANK_8; // south
        *targets |= (king >> 9) & !Bitboard::RANK_8 & !Bitboard::H_FILE; // southwest

        *targets |= (king << 1) & !Bitboard::A_FILE; // east
        *targets |= (king >> 1) & !Bitboard::H_FILE; // west
    }

    table
}

#[cfg(test)]
mod tests {
    use crate::chess_move::chess_move_effect::ChessMoveEffect;
    use common::bitboard::square::*;

    use super::*;
    use crate::chess_move::chess_move::ChessMove;
    use crate::chess_move::standard::StandardChessMove;
    use crate::{chess_position, std_move};

    impl Targets {
        pub fn new() -> Self {
            Default::default()
        }
    }

    #[test]
    fn test_generate_attack_targets_1() {
        let targets = Targets::new();
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
        let targets = Targets::new();
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
