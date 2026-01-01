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
#[cfg(feature = "instrumentation")]
use tracing::instrument;

use super::magic_table::MagicTable;

/// A `PieceTarget` is a tuple of a piece's square and the squares it can move to.
pub type PieceTarget = (Square, Bitboard); // (piece_square, targets)

/// A list of PieceTargets that is optimized for small sizes.
/// Similar to `ChessMoveList`, see the documentation for reasoning around performance.
pub type PieceTargetList = SmallVec<[PieceTarget; 16]>;

/// Information about pinned pieces and their movement constraints.
///
/// A piece is pinned when moving it would expose the king to attack from a sliding piece.
/// Pinned pieces can only move along the pin ray (the line from attacker through piece to king).
#[derive(Debug, Clone)]
pub struct PinInfo {
    /// Bitboard of all pinned pieces for the given color
    pub pinned_pieces: Bitboard,
    /// For each square, the bitboard of squares a pinned piece can move to (pin ray).
    /// Only valid for squares in `pinned_pieces`.
    pub pin_rays: [Bitboard; 64],
}

impl PinInfo {
    /// Creates empty pin information (no pins)
    pub fn empty() -> Self {
        Self {
            pinned_pieces: Bitboard::EMPTY,
            pin_rays: [Bitboard::EMPTY; 64],
        }
    }

    /// Returns true if the given square contains a pinned piece
    #[inline]
    pub fn is_pinned(&self, square: Square) -> bool {
        self.pinned_pieces.overlaps(Bitboard(1 << square.index()))
    }

    /// Returns the pin ray for a given square (only valid if the square is pinned)
    #[inline]
    pub fn pin_ray(&self, square: Square) -> Bitboard {
        self.pin_rays[square.index() as usize]
    }
}

/// Information about checks and checking pieces.
///
/// Used to generate legal moves when the king is in check. Only certain moves are legal:
/// - King moves to safe squares
/// - Capturing the checking piece (if single check)
/// - Blocking the check ray (if single sliding check)
#[derive(Debug, Clone)]
pub struct CheckInfo {
    /// Bitboard of pieces giving check
    pub checkers: Bitboard,
    /// Bitboard of squares between checker and king (for blocking).
    /// Only valid for single sliding checks. Empty for knight/pawn checks or double checks.
    pub check_ray: Bitboard,
}

impl CheckInfo {
    /// Creates empty check information (not in check)
    pub fn empty() -> Self {
        Self {
            checkers: Bitboard::EMPTY,
            check_ray: Bitboard::EMPTY,
        }
    }

    /// Returns true if in check
    #[inline]
    pub fn in_check(&self) -> bool {
        !self.checkers.is_empty()
    }

    /// Returns true if in double check (2+ checking pieces)
    #[inline]
    pub fn in_double_check(&self) -> bool {
        self.checkers.count_ones() >= 2
    }
}

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
    /// Calculate pinned pieces for the given color.
    ///
    /// A piece is pinned if it's on a line between the king and an opponent sliding piece,
    /// and removing it would expose the king to attack. Pinned pieces can only move along
    /// the pin ray (the line from attacker through the piece to the king).
    ///
    /// Uses magic bitboards to efficiently find all pins in a single pass.
    pub fn calculate_pins(&self, board: &Board, color: Color) -> PinInfo {
        let king = board.pieces(color).locate(Piece::King);

        // If no king exists (invalid position), return empty pin info
        if king.is_empty() {
            return PinInfo::empty();
        }

        let occupied = board.occupied();
        let own_pieces = board.pieces(color).occupied();
        let opponent_pieces = board.pieces(color.opposite()).occupied();

        let mut pin_info = PinInfo::empty();

        // Check for pins from opponent rooks and queens (orthogonal)
        let mut rook_attackers = (board.pieces(color.opposite()).locate(Piece::Rook)
            | board.pieces(color.opposite()).locate(Piece::Queen))
            & self
                .magic_table
                .get_rook_targets(king.to_square(), opponent_pieces);

        while !rook_attackers.is_empty() {
            let attacker = rook_attackers.pop_lsb();

            // Get the ray from attacker to king (including both squares)
            let ray = self
                .magic_table
                .get_rook_targets(attacker.to_square(), Bitboard::EMPTY)
                & self
                    .magic_table
                    .get_rook_targets(king.to_square(), Bitboard::EMPTY);

            // Count pieces between attacker and king
            let pieces_on_ray = ray & occupied;

            // Exactly one piece means it's pinned
            if pieces_on_ray.count_ones() == 1 {
                // Only pin our own pieces
                if pieces_on_ray.overlaps(own_pieces) {
                    pin_info.pinned_pieces |= pieces_on_ray;
                    // Pin ray includes the attacker (can capture) and squares between pinned piece and king
                    pin_info.pin_rays[pieces_on_ray.to_square().index() as usize] = ray | attacker;
                }
            }
        }

        // Check for pins from opponent bishops and queens (diagonal)
        let mut bishop_attackers = (board.pieces(color.opposite()).locate(Piece::Bishop)
            | board.pieces(color.opposite()).locate(Piece::Queen))
            & self
                .magic_table
                .get_bishop_targets(king.to_square(), opponent_pieces);

        while !bishop_attackers.is_empty() {
            let attacker = bishop_attackers.pop_lsb();

            // Get the ray from attacker to king (including both squares)
            let ray = self
                .magic_table
                .get_bishop_targets(attacker.to_square(), Bitboard::EMPTY)
                & self
                    .magic_table
                    .get_bishop_targets(king.to_square(), Bitboard::EMPTY);

            // Count pieces between attacker and king
            let pieces_on_ray = ray & occupied;

            // Exactly one piece means it's pinned
            if pieces_on_ray.count_ones() == 1 {
                // Only pin our own pieces
                if pieces_on_ray.overlaps(own_pieces) {
                    pin_info.pinned_pieces |= pieces_on_ray;
                    // Pin ray includes the attacker (can capture) and squares between pinned piece and king
                    pin_info.pin_rays[pieces_on_ray.to_square().index() as usize] = ray | attacker;
                }
            }
        }

        pin_info
    }

    /// Calculate check information for the given color.
    ///
    /// Determines which opponent pieces are giving check and computes the check ray
    /// (squares between checker and king) for blocking moves. For double checks,
    /// only king moves are legal, so the check ray is not computed.
    pub fn calculate_checks(&self, board: &Board, color: Color) -> CheckInfo {
        let king = board.pieces(color).locate(Piece::King);

        // If no king exists (invalid position), return empty check info
        if king.is_empty() {
            return CheckInfo::empty();
        }

        let opponent_attacks = self.generate_attack_targets(board, color.opposite());

        // If king not attacked, no check
        if !opponent_attacks.overlaps(king) {
            return CheckInfo::empty();
        }

        let mut check_info = CheckInfo::empty();
        let occupied = board.occupied();

        // Find checking pieces
        // Check for pawn attacks
        let opponent_pawns = board.pieces(color.opposite()).locate(Piece::Pawn);
        let pawn_attack_squares = match color {
            // White king attacked by black pawns from above
            Color::White => (king << 7 & !Bitboard::H_FILE) | (king << 9 & !Bitboard::A_FILE),
            // Black king attacked by white pawns from below
            Color::Black => (king >> 7 & !Bitboard::A_FILE) | (king >> 9 & !Bitboard::H_FILE),
        };
        check_info.checkers |= opponent_pawns & pawn_attack_squares;

        // Check for knight attacks
        let opponent_knights = board.pieces(color.opposite()).locate(Piece::Knight);
        let knight_attacks = self.knights[king.to_square().index() as usize];
        check_info.checkers |= opponent_knights & knight_attacks;

        // Check for sliding piece attacks (rooks, bishops, queens)
        let rook_attacks = self
            .magic_table
            .get_rook_targets(king.to_square(), occupied);
        let opponent_rooks = board.pieces(color.opposite()).locate(Piece::Rook)
            | board.pieces(color.opposite()).locate(Piece::Queen);
        check_info.checkers |= opponent_rooks & rook_attacks;

        let bishop_attacks = self
            .magic_table
            .get_bishop_targets(king.to_square(), occupied);
        let opponent_bishops = board.pieces(color.opposite()).locate(Piece::Bishop)
            | board.pieces(color.opposite()).locate(Piece::Queen);
        check_info.checkers |= opponent_bishops & bishop_attacks;

        // Calculate check ray for blocking (only for single sliding checks)
        if check_info.checkers.count_ones() == 1 {
            let checker = check_info.checkers;

            // Check if it's a sliding piece check
            let is_sliding_check = (opponent_rooks | opponent_bishops).overlaps(checker);

            if is_sliding_check {
                // Check ray is squares between checker and king (excluding both)
                // Use the appropriate magic table based on piece type
                let ray_from_king = if opponent_rooks.overlaps(checker) {
                    self.magic_table
                        .get_rook_targets(king.to_square(), Bitboard::EMPTY)
                } else {
                    self.magic_table
                        .get_bishop_targets(king.to_square(), Bitboard::EMPTY)
                };

                let ray_from_checker = if opponent_rooks.overlaps(checker) {
                    self.magic_table
                        .get_rook_targets(checker.to_square(), Bitboard::EMPTY)
                } else {
                    self.magic_table
                        .get_bishop_targets(checker.to_square(), Bitboard::EMPTY)
                };

                // Squares between checker and king
                check_info.check_ray = ray_from_king & ray_from_checker;
            }
        }

        check_info
    }

    #[cfg_attr(feature = "instrumentation", instrument(skip_all))]
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

    #[cfg_attr(feature = "instrumentation", instrument(skip_all))]
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

    #[cfg_attr(feature = "instrumentation", instrument(skip_all))]
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
        assert_eq!(expected_white_targets, white_targets);
    }

    #[test]
    fn test_calculate_pins_orthogonal_rook() {
        let targets = Targets::default();
        let board = chess_position! {
            ........
            ........
            ........
            ........
            r...K...
            ........
            ........
            ........
        };

        let pin_info = targets.calculate_pins(&board, Color::White);

        // No pieces between rook and king, so no pins
        assert!(pin_info.pinned_pieces.is_empty());
    }

    #[test]
    fn test_calculate_pins_with_pinned_piece() {
        let targets = Targets::default();
        let board = chess_position! {
            ........
            ........
            ........
            ........
            r.N.K...
            ........
            ........
            ........
        };

        let pin_info = targets.calculate_pins(&board, Color::White);

        // Knight at C4 is pinned by rook at A4
        assert_eq!(pin_info.pinned_pieces.count_ones(), 1);
        assert!(pin_info.is_pinned(C4));

        // Pin ray should include squares from knight to attacker (and attacker itself)
        let pin_ray = pin_info.pin_ray(C4);
        assert!(pin_ray.overlaps(Bitboard(1 << B4.index()))); // Between knight and rook
        assert!(pin_ray.overlaps(Bitboard(1 << A4.index()))); // Attacker
        assert!(pin_ray.overlaps(Bitboard(1 << D4.index()))); // Between knight and king
    }

    #[test]
    fn test_calculate_pins_diagonal_bishop() {
        let targets = Targets::default();
        let board = chess_position! {
            b.......
            ........
            ..N.....
            ........
            ....K...
            ........
            ........
            ........
        };

        let pin_info = targets.calculate_pins(&board, Color::White);

        // Knight at C6 is pinned by bishop at A8
        assert_eq!(pin_info.pinned_pieces.count_ones(), 1);
        assert!(pin_info.is_pinned(C6));

        // Pin ray should include diagonal from knight to attacker
        let pin_ray = pin_info.pin_ray(C6);
        assert!(pin_ray.overlaps(Bitboard(1 << B7.index()))); // Between knight and bishop
        assert!(pin_ray.overlaps(Bitboard(1 << A8.index()))); // Attacker
        assert!(pin_ray.overlaps(Bitboard(1 << D5.index()))); // Between knight and king
    }

    #[test]
    fn test_calculate_pins_multiple_pins() {
        let targets = Targets::default();
        let board = chess_position! {
            q.......
            ........
            ..B.....
            ........
            r.R.K...
            ........
            ........
            ........
        };

        let pin_info = targets.calculate_pins(&board, Color::White);

        // Bishop at C6 pinned by queen at A8 (diagonal)
        // Rook at C4 pinned by rook at A4 (orthogonal)
        assert_eq!(pin_info.pinned_pieces.count_ones(), 2);
        assert!(pin_info.is_pinned(C6));
        assert!(pin_info.is_pinned(C4));
    }

    #[test]
    fn test_calculate_pins_no_pin_two_pieces_on_ray() {
        let targets = Targets::default();
        let board = chess_position! {
            ........
            ........
            ........
            ........
            r.N.BK..
            ........
            ........
            ........
        };

        let pin_info = targets.calculate_pins(&board, Color::White);

        // Two pieces on the ray means no pin
        assert!(pin_info.pinned_pieces.is_empty());
    }

    #[test]
    fn test_calculate_checks_no_check() {
        let targets = Targets::default();
        let board = chess_position! {
            ........
            ........
            ........
            ........
            ....K...
            ........
            ........
            r.......
        };

        let check_info = targets.calculate_checks(&board, Color::White);

        assert!(!check_info.in_check());
        assert!(check_info.checkers.is_empty());
    }

    #[test]
    fn test_calculate_checks_single_check_rook() {
        let targets = Targets::default();
        let board = chess_position! {
            ........
            ........
            ........
            ........
            r...K...
            ........
            ........
            ........
        };

        let check_info = targets.calculate_checks(&board, Color::White);

        assert!(check_info.in_check());
        assert!(!check_info.in_double_check());
        assert_eq!(check_info.checkers.count_ones(), 1);

        // Check ray should include squares between rook and king
        assert!(check_info.check_ray.overlaps(Bitboard(1 << B4.index())));
        assert!(check_info.check_ray.overlaps(Bitboard(1 << C4.index())));
        assert!(check_info.check_ray.overlaps(Bitboard(1 << D4.index())));
    }

    #[test]
    fn test_calculate_checks_single_check_bishop() {
        let targets = Targets::default();
        let board = chess_position! {
            b.......
            ........
            ........
            ........
            ....K...
            ........
            ........
            ........
        };

        let check_info = targets.calculate_checks(&board, Color::White);

        assert!(check_info.in_check());
        assert!(!check_info.in_double_check());
        assert_eq!(check_info.checkers.count_ones(), 1);

        // Check ray should include diagonal squares
        assert!(check_info.check_ray.overlaps(Bitboard(1 << B7.index())));
        assert!(check_info.check_ray.overlaps(Bitboard(1 << C6.index())));
        assert!(check_info.check_ray.overlaps(Bitboard(1 << D5.index())));
    }

    #[test]
    fn test_calculate_checks_knight_no_check_ray() {
        let targets = Targets::default();
        let board = chess_position! {
            ........
            ........
            ........
            ........
            ....K...
            ......n.
            ........
            ........
        };

        let check_info = targets.calculate_checks(&board, Color::White);

        assert!(check_info.in_check());
        assert!(!check_info.in_double_check());
        assert_eq!(check_info.checkers.count_ones(), 1);

        // Knight check has no check ray (can't block knight checks)
        assert!(check_info.check_ray.is_empty());
    }

    #[test]
    fn test_calculate_checks_pawn_check() {
        let targets = Targets::default();
        let board = chess_position! {
            ........
            ........
            ........
            ...p....
            ....K...
            ........
            ........
            ........
        };

        let check_info = targets.calculate_checks(&board, Color::White);

        assert!(check_info.in_check());
        assert!(!check_info.in_double_check());
        assert_eq!(check_info.checkers.count_ones(), 1);

        // Pawn check has no check ray
        assert!(check_info.check_ray.is_empty());
    }

    #[test]
    fn test_calculate_checks_double_check() {
        let targets = Targets::default();
        let board = chess_position! {
            ........
            ........
            ........
            ........
            r...K...
            ......n.
            ........
            ........
        };

        let check_info = targets.calculate_checks(&board, Color::White);

        assert!(check_info.in_check());
        assert!(check_info.in_double_check());
        assert_eq!(check_info.checkers.count_ones(), 2);

        // Double check: check ray not meaningful (only king moves allowed)
        // Implementation doesn't compute it for double checks
    }
}
