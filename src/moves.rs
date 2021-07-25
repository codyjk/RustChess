mod bitboard;
mod fen;

use crate::bitboard::color::Color;
use crate::bitboard::piece::Piece;
use crate::bitboard::square::Square;
use crate::bitboard::Bitboard;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ChessMove {
    pub from_square: Square,
    pub to_square: Square,
}

impl ChessMove {
    pub fn new(from_square: Square, to_square: Square) -> Self {
        Self {
            from_square: from_square,
            to_square: to_square,
        }
    }
}

pub fn generate(board: &Bitboard, color: Color) -> Vec<ChessMove> {
    let pieces = board.pieces(color);
    let occupied = board.occupied();
    let mut moves = vec![];

    moves.append(&mut generate_pawn_moves(
        pieces.locate(Piece::Pawn),
        occupied,
        color,
    ));

    moves
}

fn generate_pawn_moves(pawns: u64, occupied: u64, color: Color) -> Vec<ChessMove> {
    let single_move_targets = match color {
        Color::White => pawns << 8, // move 1 rank up the board
        Color::Black => pawns >> 8, // move 1 rank down the board
    };
    let double_move_targets: u64 = match color {
        Color::White => 0x00000000FF000000, // rank 4
        Color::Black => 0x000000FF00000000, // rank 5
    };

    let targets = (single_move_targets | double_move_targets) & !occupied;

    let mut moves: Vec<ChessMove> = vec![];

    for x in 0..64 {
        let pawn = 1 << x;
        if pawns & pawn == 0 {
            continue;
        }

        let single_move = match color {
            Color::White => pawn << 8,
            Color::Black => pawn >> 8,
        };
        if single_move & targets > 0 {
            let mv = ChessMove::new(Square::from_bit(pawn), Square::from_bit(single_move));
            moves.push(mv);
        }

        let double_move = match color {
            Color::White => single_move << 8,
            Color::Black => single_move >> 8,
        };
        if double_move & targets > 0 {
            let mv = ChessMove::new(Square::from_bit(pawn), Square::from_bit(double_move));
            moves.push(mv);
        }
    }

    moves
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_pawn_moves() {
        let mut board = Bitboard::new();
        board.put(Square::A4, Piece::Pawn, Color::White).unwrap();
        board.put(Square::A5, Piece::Pawn, Color::Black).unwrap();
        board.put(Square::D2, Piece::Pawn, Color::White).unwrap();
        board.put(Square::D7, Piece::Pawn, Color::Black).unwrap();
        let occupied = board.occupied();
        println!("Testing board:\n{}", board.to_ascii());

        let expected_white_moves: Vec<ChessMove> = vec![
            ChessMove::new(Square::D2, Square::D3),
            ChessMove::new(Square::D2, Square::D4),
        ];

        let expected_black_moves: Vec<ChessMove> = vec![
            ChessMove::new(Square::D7, Square::D6),
            ChessMove::new(Square::D7, Square::D5),
        ];

        let white_moves = generate_pawn_moves(
            board.pieces(Color::White).locate(Piece::Pawn),
            occupied,
            Color::White,
        );
        assert_eq!(expected_white_moves, white_moves);

        let black_moves = generate_pawn_moves(
            board.pieces(Color::Black).locate(Piece::Pawn),
            occupied,
            Color::Black,
        );
        assert_eq!(expected_black_moves, black_moves);
    }
}
