use crate::board::bitboard::{
    A_FILE, B_FILE, EMPTY, G_FILE, H_FILE, RANK_1, RANK_4, RANK_5, RANK_8,
};
use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::{square, Board};
use crate::moves::ray_table::{Direction, RayTable, BISHOP_DIRS, ROOK_DIRS};
use ahash::AHashMap;

pub type PieceTarget = (u64, u64); // (piece_square, targets)

type TargetsTable = AHashMap<u64, u64>;

pub struct Targets {
    knights: TargetsTable,
    rays: RayTable,
    attacks_cache: AHashMap<(u64, u64), u64>,
}

impl Targets {
    pub fn new() -> Self {
        let mut ray_table = RayTable::new();
        ray_table.populate();

        Self {
            knights: generate_knight_targets_table(),
            rays: ray_table,
            attacks_cache: AHashMap::new(),
        }
    }

    pub fn get(&self, square: u64, piece: Piece) -> u64 {
        match piece {
            Piece::Knight => *self.knights.get(&square).unwrap(),
            _ => 0,
        }
    }

    pub fn get_cached_attack(&self, color: Color, board_hash: u64) -> Option<u64> {
        let color_key = match color {
            Color::Black => 0,
            Color::White => 1,
        };
        self.attacks_cache.get(&(color_key, board_hash)).map(|b| *b)
    }

    pub fn cache_attack(&mut self, color: Color, board_hash: u64, attack_targets: u64) -> u64 {
        let color_key = match color {
            Color::Black => 0,
            Color::White => 1,
        };
        match self
            .attacks_cache
            .insert((color_key, board_hash), attack_targets)
        {
            Some(old_targets) => old_targets,
            None => attack_targets,
        }
    }
}

pub fn generate_piece_targets(
    board: &Board,
    color: Color,
    piece: Piece,
    targets: &Targets,
) -> Vec<PieceTarget> {
    let mut piece_targets: Vec<(u64, u64)> = vec![];
    let pieces = board.pieces(color).locate(piece);
    let occupied = board.pieces(color).occupied();

    for &sq in &square::ORDERED {
        if pieces & sq == 0 {
            continue;
        }

        let candidates = targets.get(sq, piece) & !occupied;

        if candidates == 0 {
            continue;
        }

        piece_targets.push((sq, candidates));
    }

    piece_targets
}

pub fn generate_pawn_targets(board: &Board, color: Color) -> Vec<PieceTarget> {
    let mut piece_targets: Vec<PieceTarget> = vec![];

    let pawns = board.pieces(color).locate(Piece::Pawn);
    let occupied = board.occupied();

    let single_move_targets = match color {
        Color::White => pawns << 8, // move 1 rank up the board
        Color::Black => pawns >> 8, // move 1 rank down the board
    };
    let double_move_targets = match color {
        Color::White => RANK_4,
        Color::Black => RANK_5,
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

        if single_move & occupied > 0 {
            // pawn is blocked and can make no moves
            continue;
        }

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

    let attack_targets = board.pieces(color.opposite()).occupied();

    for (pawn, targets) in generate_pawn_attack_targets(board, color) {
        if attack_targets & targets > 0 {
            piece_targets.push((pawn, attack_targets & targets));
        }
    }

    piece_targets
}

// having a separate function for generating pawn attacks is useful for generating
// attack maps. this separates the attacked squares from the ones with enemy pieces
// on them
pub fn generate_pawn_attack_targets(board: &Board, color: Color) -> Vec<PieceTarget> {
    let mut piece_targets: Vec<PieceTarget> = vec![];

    let pawns = board.pieces(color).locate(Piece::Pawn);

    for x in 0..64 {
        let pawn = 1 << x;
        if pawns & pawn == 0 {
            continue;
        }

        let attack_west = match color {
            Color::White => (pawn << 9) & !A_FILE,
            Color::Black => (pawn >> 7) & !A_FILE,
        };

        let attack_east = match color {
            Color::White => (pawn << 7) & !H_FILE,
            Color::Black => (pawn >> 9) & !H_FILE,
        };

        let targets = attack_east | attack_west;

        piece_targets.push((pawn, targets));
    }

    piece_targets
}

pub fn generate_knight_targets_table() -> TargetsTable {
    let mut table = AHashMap::new();

    for x in 0..64 {
        let knight = 1 << x;

        // nne = north-north-east, nee = north-east-east, etc..
        let move_nne = knight << 17 & !A_FILE;
        let move_nee = knight << 10 & !A_FILE & !B_FILE;
        let move_see = knight >> 6 & !A_FILE & !B_FILE;
        let move_sse = knight >> 15 & !A_FILE;
        let move_nnw = knight << 15 & !H_FILE;
        let move_nww = knight << 6 & !G_FILE & !H_FILE;
        let move_sww = knight >> 10 & !G_FILE & !H_FILE;
        let move_ssw = knight >> 17 & !H_FILE;

        let targets =
            move_nne | move_nee | move_see | move_sse | move_nnw | move_nww | move_sww | move_ssw;

        table.insert(knight, targets);
    }

    table
}

fn generate_ray_targets(
    board: &Board,
    color: Color,
    targets: &Targets,
    ray_piece: Piece,
    ray_dirs: [Direction; 4],
) -> Vec<PieceTarget> {
    let pieces = board.pieces(color).locate(ray_piece);
    let occupied = board.occupied();
    let mut piece_targets: Vec<(u64, u64)> = vec![];

    for x in 0..64 {
        let piece = 1 << x;
        if pieces & piece == 0 {
            continue;
        }

        let mut target_squares = EMPTY;

        for dir in ray_dirs.iter() {
            let ray = targets.rays.get(piece, *dir);
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
                Direction::NorthWest => rightmost_bit(intercepts),
                Direction::NorthEast => rightmost_bit(intercepts),
                Direction::SouthWest => leftmost_bit(intercepts),
                Direction::SouthEast => leftmost_bit(intercepts),
            };

            let blocked_squares = targets.rays.get(intercept, *dir);

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

pub fn generate_rook_targets(board: &Board, color: Color, targets: &Targets) -> Vec<PieceTarget> {
    generate_ray_targets(board, color, targets, Piece::Rook, ROOK_DIRS)
}

pub fn generate_bishop_targets(board: &Board, color: Color, targets: &Targets) -> Vec<PieceTarget> {
    generate_ray_targets(board, color, targets, Piece::Bishop, BISHOP_DIRS)
}

pub fn generate_queen_targets(board: &Board, color: Color, targets: &Targets) -> Vec<PieceTarget> {
    let mut piece_targets: Vec<PieceTarget> = vec![];

    piece_targets.append(&mut generate_ray_targets(
        board,
        color,
        targets,
        Piece::Queen,
        ROOK_DIRS,
    ));
    piece_targets.append(&mut generate_ray_targets(
        board,
        color,
        targets,
        Piece::Queen,
        BISHOP_DIRS,
    ));

    piece_targets
}

pub fn generate_king_targets(board: &Board, color: Color) -> Vec<PieceTarget> {
    let king = board.pieces(color).locate(Piece::King);
    let occupied = board.pieces(color).occupied();

    let mut targets = EMPTY;

    // shift the king's position. in the event that it falls off of the boundary,
    // we want to negate the rank/file where the king would fall.
    targets |= (king << 9) & !RANK_1 & !A_FILE & !occupied; // northeast
    targets |= (king << 8) & !RANK_1 & !occupied; // north
    targets |= (king << 7) & !RANK_1 & !H_FILE & !occupied; // northwest

    targets |= (king >> 7) & !RANK_8 & !A_FILE & !occupied; // southeast
    targets |= (king >> 8) & !RANK_8 & !occupied; // south
    targets |= (king >> 9) & !RANK_8 & !H_FILE & !occupied; // southwest

    targets |= (king << 1) & !A_FILE & !occupied; // east
    targets |= (king >> 1) & !H_FILE & !occupied; // west

    vec![(king, targets)]
}

pub fn generate_attack_targets(board: &Board, color: Color, targets: &mut Targets) -> u64 {
    let board_hash = board.current_position_hash();

    match targets.get_cached_attack(color, board_hash) {
        Some(cached_targets) => return cached_targets,
        None => (),
    };

    let mut piece_targets: Vec<PieceTarget> = vec![];
    let mut attack_targets = EMPTY;

    piece_targets.append(&mut generate_pawn_attack_targets(board, color));
    piece_targets.append(&mut generate_piece_targets(
        board,
        color,
        Piece::Knight,
        targets,
    ));
    piece_targets.append(&mut generate_rook_targets(board, color, targets));
    piece_targets.append(&mut generate_bishop_targets(board, color, targets));
    piece_targets.append(&mut generate_queen_targets(board, color, targets));
    piece_targets.append(&mut generate_king_targets(board, color));

    for (_piece, targets) in piece_targets {
        attack_targets |= targets;
    }

    targets.cache_attack(color, board_hash, attack_targets);

    attack_targets
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::bitboard::{render_occupied, RANK_3};
    use crate::board::square;
    use crate::moves::ChessMove;

    #[test]
    fn test_generate_king_targets() {
        let mut board = Board::new();
        board.put(square::H7, Piece::King, Color::White).unwrap();
        println!("Testing board:\n{}", board);
        let occupied = board.pieces(Color::White).occupied();

        let expected_targets =
            EMPTY | square::G6 | square::H6 | square::G7 | square::G8 | square::H8;

        let result = generate_king_targets(&board, Color::White);
        let (_king, targets) = result[0];

        println!("occupied:\n{}", render_occupied(occupied));
        println!("Targets:\n{}", render_occupied(targets));
        assert_eq!(expected_targets, targets);
    }

    #[test]
    fn test_generate_attack_targets() {
        let mut targets = Targets::new();
        let mut board = Board::new();

        board.put(square::A4, Piece::Pawn, Color::White).unwrap();
        board.put(square::B5, Piece::Pawn, Color::Black).unwrap();
        board.put(square::B1, Piece::Rook, Color::White).unwrap();
        board.put(square::H1, Piece::King, Color::Black).unwrap();
        board.put(square::A5, Piece::Queen, Color::White).unwrap();
        println!("Testing board:\n{}", board);

        let expected_white_targets = EMPTY
            // pawn
            | square::B5
            // rook
            | square::B2
            | square::B3
            | square::B4
            | square::B5
            | (RANK_1 ^ square::B1)
            // queen - north
            | square::A6
            | square::A7
            | square::A8
            // queen - northeast
            | square::B6
            | square::C7
            | square::D8
            // queen - east
            | square::B5
            // queen - southeast
            | square::B4
            | square::C3
            | square::D2
            | square::A1;
        let white_targets = generate_attack_targets(&board, Color::White, &mut targets);
        assert_eq!(expected_white_targets, white_targets);

        let expected_black_targets = EMPTY
            // pawn
            | square::A4
            | square::C4
            // king
            | square::G1
            | square::G2
            | square::H2;
        let black_targets = generate_attack_targets(&board, Color::Black, &mut targets);
        assert_eq!(expected_black_targets, black_targets);
    }

    #[test]
    pub fn test_generate_attack_targets_2() {
        let mut targets = Targets::new();
        let mut board = Board::starting_position();

        board
            .apply(ChessMove::new(square::E2, square::E4, None))
            .unwrap();
        board
            .apply(ChessMove::new(square::F7, square::F5, None))
            .unwrap();
        board
            .apply(ChessMove::new(square::D1, square::H5, None))
            .unwrap();
        board
            .apply(ChessMove::new(square::G7, square::G6, None))
            .unwrap();
        println!("Testing board:\n{}", board);

        //   +---+---+---+---+---+---+---+---+
        // 8 | r | n | b | q | k | b | n | r |
        //   +---+---+---+---+---+---+---+---+
        // 7 | p | p | p | p | p |   |   | p |
        //   +---+---+---+---+---+---+---+---+
        // 6 |   |   |   |   |   |   | p |   |
        //   +---+---+---+---+---+---+---+---+
        // 5 |   |   |   |   |   | p |   | Q |
        //   +---+---+---+---+---+---+---+---+
        // 4 |   |   |   |   | P |   |   |   |
        //   +---+---+---+---+---+---+---+---+
        // 3 |   |   |   |   |   |   |   |   |
        //   +---+---+---+---+---+---+---+---+
        // 2 | P | P | P | P |   | P | P | P |
        //   +---+---+---+---+---+---+---+---+
        // 1 | R | N | B |   | K | B | N | R |
        //   +---+---+---+---+---+---+---+---+
        //     A   B   C   D   E   F   G   H

        let expected_white_targets = EMPTY
            // knights
            | RANK_3
            // forward pawn
            | square::D5
            | square::F5
            // queen - north
            | square::H6
            | square::H7
            // queen - nortwest
            | square::G6
            // queen - west
            | square::G5
            | square::F5
            // queen - southwest
            | square::G4
            | square::F3
            | square::E2
            | square::D1
            // queen - south
            | square::H4
            | square::H3
            // bishop
            | square::E2
            | square::D3
            | square::C4
            | square::B5
            | square::A6
            // king
            | square::D1
            | square::E2;

        let white_targets = generate_attack_targets(&board, Color::White, &mut targets);
        println!(
            "expected white targets:\n{}",
            render_occupied(expected_white_targets)
        );
        println!("actual white targets:\n{}", render_occupied(white_targets));
        assert_eq!(expected_white_targets, white_targets);
    }
}
