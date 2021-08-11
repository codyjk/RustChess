use crate::board::bitboard::{Bitboard, A_FILE, EMPTY, H_FILE, RANK_1, RANK_4, RANK_5, RANK_8};
use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::Board;
use crate::moves::ray_table::{Direction, RayTable};

pub type PieceTarget = (u64, u64); // (piece_square, targets)

fn rightmost_bit(x: u64) -> u64 {
    x & (!x + 1)
}

fn leftmost_bit(x: u64) -> u64 {
    let mut b = x;

    // fill in rightmost bits
    b |= b >> 32;
    b |= b >> 16;
    b |= b >> 8;
    b |= b >> 4;
    b |= b >> 2;
    b |= b >> 1;

    // get the leftmost bit
    b ^ (b >> 1)
}

pub fn generate_pawn_move_targets(board: &Board, color: Color) -> Vec<PieceTarget> {
    let mut piece_targets: Vec<PieceTarget> = vec![];

    let pawns = board.pieces(color).locate(Piece::Pawn);
    let occupied = board.occupied();

    let single_move_targets = match color {
        Color::White => pawns << 8, // move 1 rank up the board
        Color::Black => pawns >> 8, // move 1 rank down the board
    };
    let double_move_targets = match color {
        Color::White => RANK_4, // rank 4
        Color::Black => RANK_5, // rank 5
    };
    let move_targets = (single_move_targets | double_move_targets) & !occupied;

    for x in 0..64 {
        let pawn = 1 << x;
        if pawns & pawn == 0 {
            continue;
        }
        let mut targets = EMPTY;

        let single_move = match color {
            Color::White => pawn << 8,
            Color::Black => pawn >> 8,
        };

        let double_move = match color {
            Color::White => single_move << 8,
            Color::Black => single_move >> 8,
        };

        targets |= single_move & move_targets;
        targets |= double_move & move_targets;

        if targets == EMPTY {
            continue;
        }

        piece_targets.push((pawn, targets));
    }

    piece_targets
}

pub fn generate_pawn_attack_targets(board: &Board, color: Color) -> Vec<PieceTarget> {
    let mut piece_targets: Vec<PieceTarget> = vec![];

    let pawns = board.pieces(color).locate(Piece::Pawn);

    let attack_targets = board.pieces(color.opposite()).occupied();

    for x in 0..64 {
        let pawn = 1 << x;
        if pawns & pawn == 0 {
            continue;
        }

        let mut targets = EMPTY;

        let attack_west = match color {
            Color::White => (pawn << 9) & !A_FILE,
            Color::Black => (pawn >> 7) & !A_FILE,
        };

        let attack_east = match color {
            Color::White => (pawn << 7) & !H_FILE,
            Color::Black => (pawn >> 9) & !H_FILE,
        };

        targets |= attack_west & attack_targets;
        targets |= attack_east & attack_targets;

        if targets == EMPTY {
            continue;
        }

        piece_targets.push((pawn, targets));
    }

    piece_targets
}

pub fn generate_ray_targets(
    board: &Board,
    color: Color,
    ray_table: &RayTable,
    ray_piece: Piece,
    ray_dirs: [Direction; 4],
) -> Vec<PieceTarget> {
    let pieces = board.pieces(color).locate(ray_piece);
    let occupied = board.occupied();
    let mut piece_targets: Vec<(Bitboard, Bitboard)> = vec![];

    for x in 0..64 {
        let piece = 1 << x;
        if pieces & piece == 0 {
            continue;
        }

        let mut target_squares = EMPTY;

        for dir in ray_dirs.iter() {
            let ray = ray_table.get(piece, *dir);
            if ray == 0 {
                continue;
            }

            let intercepts = ray & occupied;

            if intercepts == 0 {
                piece_targets.push((piece, ray));
                continue;
            }

            // intercept = where the piece's ray is terminated.
            // in each direction, the goal is to select the intercept
            // that is closest to the piece. for each direction, this is either
            // the leftmost or rightmost bit.
            let intercept = match dir {
                // ROOKS
                Direction::North => rightmost_bit(intercepts),
                Direction::East => rightmost_bit(intercepts),
                Direction::South => leftmost_bit(intercepts),
                Direction::West => leftmost_bit(intercepts),

                // BISHOPS
                Direction::NorthWest => leftmost_bit(intercepts),
                Direction::NorthEast => rightmost_bit(intercepts),
                Direction::SouthWest => leftmost_bit(intercepts),
                Direction::SouthEast => rightmost_bit(intercepts),
            };

            let blocked_squares = ray_table.get(intercept, *dir);

            target_squares |= ray ^ blocked_squares;

            // if the intercept is the same color piece, remove it from the targets.
            // otherwise, it is a target square because it belongs to the other
            // color and can therefore be captured
            if intercept & board.pieces(color).occupied() > 0 {
                target_squares ^= intercept;
            }
        }

        piece_targets.push((piece, target_squares));
    }

    piece_targets
}

pub fn generate_king_targets(board: &Board, color: Color) -> Vec<PieceTarget> {
    let king = board.pieces(color).locate(Piece::King);
    let occupied = board.pieces(color).occupied();

    let mut targets = EMPTY;

    // shift the king's position. in the event that it falls off of the boundary,
    // we want to negate the rank/file where the king would fall.
    targets |= (king >> 8) & !RANK_1 & !occupied; // north
    targets |= (king << 8) & !RANK_8 & !occupied; // south
    targets |= (king << 1) & !A_FILE & !occupied; // east
    targets |= (king >> 1) & !H_FILE & !occupied; // west
    targets |= (king >> 7) & !RANK_1 & !A_FILE & !occupied; // northeast
    targets |= (king >> 9) & !RANK_1 & !H_FILE & !occupied; // northwest
    targets |= (king << 9) & !RANK_8 & !A_FILE & !occupied; // southeast
    targets |= (king << 7) & !RANK_8 & !H_FILE & !occupied; // southwest

    vec![(king, targets)]
}
