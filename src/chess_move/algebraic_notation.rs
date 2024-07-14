use common::bitboard::{
    bitboard::Bitboard,
    square::{to_algebraic, C1, C8, E1, E8, G1, G8},
};

use crate::{
    board::{color::Color, piece::Piece, Board},
    evaluate::{current_player_is_in_check, game_ending, GameEnding},
    move_generator::{ChessMoveList, MoveGenerator},
};

use super::{castle::CastleChessMove, ChessMove};

const CAPTURE_CHAR: &str = "x";
const CASTLE_KINGSIDE_CHARS: &str = "O-O";
const CASTLE_QUEENSIDE_CHARS: &str = "O-O-O";
const CHECKMATE_CHAR: &str = "#";
const CHECK_CHAR: &str = "+";
const PROMOTION_CHAR: &str = "=";

const EMPTY_STRING: &str = "";

/// For a given board state, this function lists all candidate moves with their algebraic notation.
///
/// Calculation of algebratic notation requires calculation of check/checkmate,
/// which is expensive and not stored directly on `ChessMove`.
/// As a result, correctly calculating algebraic notation is also an expensive operation,
/// and so we isolate the expensive calculation to this function.
///
/// By enumerating the entire list of moves and their notations, we can avoid
/// needing functions like `to_algebraic_notation` and `from_algebraic_notation`,
/// as the callsite has the complete list of moves and their notations fully described.
///
/// See: https://www.chessprogramming.org/Algebraic_Notation
pub fn enumerate_candidate_moves_in_algebraic_notation(
    board: &mut Board,
    current_player_color: Color,
    move_generator: &mut MoveGenerator,
) -> Vec<(ChessMove, String)> {
    let candidate_moves = move_generator.generate_moves(board, current_player_color);
    let mut moves = Vec::new();

    candidate_moves.iter().for_each(|chess_move| {
        let algebraic_move =
            chess_move_to_algebraic_notation(chess_move, board, &candidate_moves, move_generator)
                .unwrap();
        moves.push((chess_move.clone(), algebraic_move));
    });

    moves
}

fn chess_move_to_algebraic_notation(
    chess_move: &ChessMove,
    board: &mut Board,
    candidate_moves: &ChessMoveList,
    move_generator: &mut MoveGenerator,
) -> Result<String, String> {
    let check_or_checkmate_char = get_check_or_checkmate_char(chess_move, board, move_generator);
    if let ChessMove::Castle(castle_move) = chess_move {
        return Ok(format!(
            "{}{}",
            algebraic_castle(castle_move),
            check_or_checkmate_char
        ));
    }

    let (piece, _) = board.get(chess_move.from_square()).unwrap();
    let ambiguous_moves = get_ambiguous_moves(chess_move, candidate_moves, board);

    let piece_char = piece.to_algebraic_str();
    let disambiguating_char = get_disambiguating_chars(piece, chess_move, ambiguous_moves);
    let capture_char = get_capture_char(chess_move);
    let target_square_chars = to_algebraic(chess_move.to_square());
    let promotion_chars = get_promotion_chars(chess_move);

    let algebraic_move = format!(
        "{}{}{}{}{}{}",
        piece_char,
        disambiguating_char,
        capture_char,
        target_square_chars,
        promotion_chars,
        check_or_checkmate_char
    );
    Ok(algebraic_move)
}

fn algebraic_castle(castle_move: &CastleChessMove) -> String {
    match (castle_move.from_square(), castle_move.to_square()) {
        (E1, G1) => CASTLE_KINGSIDE_CHARS.to_string(),
        (E8, G8) => CASTLE_KINGSIDE_CHARS.to_string(),
        (E1, C1) => CASTLE_QUEENSIDE_CHARS.to_string(),
        (E8, C8) => CASTLE_QUEENSIDE_CHARS.to_string(),
        _ => panic!("Invalid castle move"),
    }
}

fn get_file_char(from_square: Bitboard) -> char {
    let file = to_algebraic(from_square).chars().next().unwrap();
    file
}

fn get_rank_char(from_square: Bitboard) -> char {
    let rank = to_algebraic(from_square).chars().nth(1).unwrap();
    rank
}

fn get_ambiguous_moves(
    chess_move: &ChessMove,
    candidate_moves: &ChessMoveList,
    board: &mut Board,
) -> ChessMoveList {
    let from_square = chess_move.from_square();
    let to_square = chess_move.to_square();
    let (piece, _) = board.get(from_square).unwrap();
    let mut ambiguous_moves = ChessMoveList::new();

    candidate_moves.iter().for_each(|other_move| {
        let (other_piece, _) = board.get(other_move.from_square()).unwrap();

        // Filter down to other moves that share the same piece and target square,
        // but different starting squares. This is possible if, for example, two
        // knights can jump to the same square.
        if other_move.from_square() != from_square
            && other_move.to_square() == to_square
            && other_piece == piece
        {
            ambiguous_moves.push(other_move.clone());
        }
    });

    ambiguous_moves
}

fn get_disambiguating_chars(
    piece: Piece,
    chess_move: &ChessMove,
    ambiguous_moves: ChessMoveList,
) -> String {
    let starting_file_char = get_file_char(chess_move.from_square());
    let starting_rank_char = get_rank_char(chess_move.from_square());

    // Pawn captures are always disambiguated by file (eg. dxc3 instead of just xc3)
    if piece == Piece::Pawn && chess_move.capture().is_some() {
        return starting_file_char.to_string();
    }

    let has_ambiguous_moves_on_same_file = ambiguous_moves.iter().any(|other_move| {
        let other_file_char = get_file_char(other_move.from_square());
        other_file_char == starting_file_char
    });
    let has_ambiguous_moves_on_same_rank = ambiguous_moves.iter().any(|other_move| {
        let other_rank_char = get_rank_char(other_move.from_square());
        other_rank_char == starting_rank_char
    });

    match (
        has_ambiguous_moves_on_same_file,
        has_ambiguous_moves_on_same_rank,
    ) {
        (true, true) => to_algebraic(chess_move.from_square()).to_string(),
        (true, false) => starting_rank_char.to_string(),
        (false, true) => starting_file_char.to_string(),
        (false, false) => EMPTY_STRING.to_string(),
    }
}

fn get_check_or_checkmate_char<'a>(
    chess_move: &ChessMove,
    board: &mut Board,
    move_generator: &mut MoveGenerator,
) -> &'a str {
    if move_puts_opponent_in_checkmate(chess_move, board, move_generator) {
        CHECKMATE_CHAR
    } else if move_puts_opponent_in_check(chess_move, board, move_generator) {
        CHECK_CHAR
    } else {
        EMPTY_STRING
    }
}

fn move_puts_opponent_in_check(
    chess_move: &ChessMove,
    board: &mut Board,
    move_generator: &mut MoveGenerator,
) -> bool {
    chess_move.apply(board).unwrap();
    board.toggle_turn();
    let is_check = current_player_is_in_check(board, move_generator);
    chess_move.undo(board).unwrap();
    board.toggle_turn();

    is_check
}

fn move_puts_opponent_in_checkmate(
    chess_move: &ChessMove,
    board: &mut Board,
    move_generator: &mut MoveGenerator,
) -> bool {
    chess_move.apply(board).unwrap();
    board.toggle_turn();
    let game_ending = game_ending(board, move_generator, board.turn());
    chess_move.undo(board).unwrap();
    board.toggle_turn();

    matches!(game_ending, Some(GameEnding::Checkmate))
}

fn get_capture_char(chess_move: &ChessMove) -> &str {
    if chess_move.capture().is_some() {
        CAPTURE_CHAR
    } else {
        EMPTY_STRING
    }
}

fn get_promotion_chars(chess_move: &ChessMove) -> String {
    if let ChessMove::PawnPromotion(promotion_move) = chess_move {
        format!(
            "{}{}",
            PROMOTION_CHAR,
            promotion_move.promote_to_piece().to_algebraic_str()
        )
    } else {
        EMPTY_STRING.to_string()
    }
}

#[cfg(test)]
mod tests {
    use common::bitboard::square::*;

    use crate::board::castle_rights_bitmask::{
        ALL_CASTLE_RIGHTS, BLACK_KINGSIDE_RIGHTS, BLACK_QUEENSIDE_RIGHTS, WHITE_KINGSIDE_RIGHTS,
    };
    use crate::{
        castle_kingside, castle_queenside, chess_position, en_passant_move, promotion, std_move,
    };

    use super::*;
    use crate::chess_move::castle::CastleChessMove;
    use crate::chess_move::en_passant::EnPassantChessMove;
    use crate::chess_move::pawn_promotion::PawnPromotionChessMove;
    use crate::chess_move::standard::StandardChessMove;

    macro_rules! assert_move_has_algebraic_notation {
        ($board:expr, $color:expr, $move:expr, $notation:expr) => {
            let candidate_moves = MoveGenerator::new().generate_moves(&mut $board, $color);
            assert_eq!(
                chess_move_to_algebraic_notation(
                    &$move,
                    &mut $board,
                    &candidate_moves,
                    &mut MoveGenerator::new()
                )
                .unwrap(),
                $notation
            );
        };
    }

    #[test]
    fn test_algebraic_notation_for_basic_moves() {
        let mut board = Board::starting_position();
        board.set_turn(Color::White);

        assert_move_has_algebraic_notation!(board, Color::White, std_move!(E2, E4), "e4");
        assert_move_has_algebraic_notation!(board, Color::White, std_move!(G1, F3), "Nf3");

        // Change turn without making move, just to test black side
        board.set_turn(Color::Black);

        assert_move_has_algebraic_notation!(board, Color::Black, std_move!(E7, E5), "e5");
        assert_move_has_algebraic_notation!(board, Color::Black, std_move!(G8, F6), "Nf6");
    }

    #[test]
    fn test_algebraic_notation_for_pawn_captures() {
        let mut board = chess_position! {
            k.......
            ........
            ........
            ........
            ....p...
            ...P....
            ........
            K.......
        };
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        assert_move_has_algebraic_notation!(
            &mut board,
            Color::White,
            std_move!(D3, E4, (Piece::Pawn, Color::Black)),
            "dxe4"
        );
    }

    #[test]
    fn test_algebraic_notation_for_castle_moves() {
        let mut board = chess_position! {
            r...k..r
            ........
            ........
            ........
            ........
            ........
            ........
            R...K..R
        };
        board.set_turn(Color::White);

        assert_move_has_algebraic_notation!(
            &mut board,
            Color::White,
            castle_kingside!(Color::White),
            "O-O"
        );
        assert_move_has_algebraic_notation!(
            &mut board,
            Color::White,
            castle_queenside!(Color::White),
            "O-O-O"
        );

        // Change turn without making move, just to test black side
        board.set_turn(Color::Black);

        assert_move_has_algebraic_notation!(
            &mut board,
            Color::Black,
            castle_kingside!(Color::Black),
            "O-O"
        );
        assert_move_has_algebraic_notation!(
            &mut board,
            Color::Black,
            castle_queenside!(Color::Black),
            "O-O-O"
        );
    }

    #[test]
    fn test_algebraic_notation_for_castle_moves_with_check() {
        let mut board = chess_position! {
            ...k....
            ........
            ........
            ........
            ........
            ........
            ........
            R...K...
        };
        board.set_turn(Color::White);
        board.lose_castle_rights(
            WHITE_KINGSIDE_RIGHTS | BLACK_KINGSIDE_RIGHTS | BLACK_QUEENSIDE_RIGHTS,
        );

        assert_move_has_algebraic_notation!(
            &mut board,
            Color::White,
            castle_queenside!(Color::White),
            "O-O-O+"
        );
    }
    #[test]
    fn test_algebraic_notation_for_castle_moves_with_checkmate() {
        let mut board = chess_position! {
            ..rkr...
            ..p.p...
            ........
            ........
            ........
            ........
            ........
            R...K...
        };
        board.set_turn(Color::White);
        board.lose_castle_rights(
            WHITE_KINGSIDE_RIGHTS | BLACK_KINGSIDE_RIGHTS | BLACK_QUEENSIDE_RIGHTS,
        );

        assert_move_has_algebraic_notation!(
            &mut board,
            Color::White,
            castle_queenside!(Color::White),
            "O-O-O#"
        );
    }

    #[test]
    fn test_algebraic_notation_for_captures() {
        let mut board = chess_position! {
            .......k
            ........
            ...p....
            ........
            ..N.....
            ........
            .b......
            .......K
        };
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        assert_move_has_algebraic_notation!(
            &mut board,
            Color::White,
            std_move!(C4, D6, (Piece::Pawn, Color::Black)),
            "Nxd6"
        );
        assert_move_has_algebraic_notation!(
            &mut board,
            Color::White,
            std_move!(C4, B2, (Piece::Bishop, Color::Black)),
            "Nxb2"
        );
    }

    #[test]
    fn test_algebraic_notation_for_ambiguous_moves() {
        let mut board = chess_position! {
            .....n.k
            ...P....
            .....n..
            ........
            ........
            .....N..
            R....R..
            K....N..
        };
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        assert_move_has_algebraic_notation!(&mut board, Color::White, std_move!(F1, D2), "N1d2");
        assert_move_has_algebraic_notation!(&mut board, Color::White, std_move!(F3, D2), "N3d2");
        assert_move_has_algebraic_notation!(&mut board, Color::White, std_move!(A2, B2), "Rab2");
        assert_move_has_algebraic_notation!(&mut board, Color::White, std_move!(F2, B2), "Rfb2");

        board.set_turn(Color::Black);

        assert_move_has_algebraic_notation!(
            &mut board,
            Color::Black,
            std_move!(F8, D7, (Piece::Pawn, Color::White)),
            "N8xd7"
        );
        assert_move_has_algebraic_notation!(
            &mut board,
            Color::Black,
            std_move!(F6, D7, (Piece::Pawn, Color::White)),
            "N6xd7"
        );
    }

    #[test]
    fn test_algebraic_notation_for_rare_ambiguous_moves() {
        let mut board = chess_position! {
            .......k
            ........
            ........
            ........
            ........
            .N...N..
            ........
            KN...N..
        };
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        assert_move_has_algebraic_notation!(&mut board, Color::White, std_move!(F1, D2), "Nf1d2");
        assert_move_has_algebraic_notation!(&mut board, Color::White, std_move!(F3, D2), "Nf3d2");
    }

    #[test]
    fn test_algebraic_notation_for_en_passant() {
        let mut board = chess_position! {
            k.......
            ........
            ........
            ........
            ..p.p...
            ........
            ...P....
            K.......
        };
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);
        let exposes_en_passant = std_move!(D2, D4);
        exposes_en_passant.apply(&mut board).unwrap();
        board.toggle_turn();

        assert_move_has_algebraic_notation!(
            &mut board,
            Color::Black,
            en_passant_move!(C4, D3, (Piece::Pawn, Color::White)),
            "cxd3"
        );

        assert_move_has_algebraic_notation!(
            &mut board,
            Color::Black,
            en_passant_move!(E4, D3, (Piece::Pawn, Color::White)),
            "exd3"
        );
    }

    #[test]
    fn test_algebraic_notation_for_promotion() {
        let mut board = chess_position! {
            ........
            ..P.....
            ........
            .......k
            ........
            ........
            ........
            K.......
        };
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        assert_move_has_algebraic_notation!(
            &mut board,
            Color::White,
            promotion!(C7, C8, None, Piece::Queen),
            "c8=Q"
        );
    }

    #[test]
    fn test_algebraic_notation_for_promotion_with_check() {
        let mut board = chess_position! {
            .......k
            ..P.....
            ........
            ........
            ........
            ........
            ........
            K.......
        };
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        assert_move_has_algebraic_notation!(
            &mut board,
            Color::White,
            promotion!(C7, C8, None, Piece::Queen),
            "c8=Q+"
        );
    }

    #[test]
    fn test_algebraic_notation_for_promotion_with_check_and_capture() {
        let mut board = chess_position! {
            ...r...k
            ..P.....
            ........
            ........
            ........
            ........
            ........
            K.......
        };
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        assert_move_has_algebraic_notation!(
            &mut board,
            Color::White,
            promotion!(C7, D8, Some((Piece::Rook, Color::Black)), Piece::Queen),
            "cxd8=Q+"
        );
    }

    #[test]
    fn test_algebraic_notation_for_discovered_check() {
        let mut board = chess_position! {
            ........
            ........
            ........
            ........
            ..RP...k
            ........
            ........
            ........
        };
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        assert_move_has_algebraic_notation!(&mut board, Color::White, std_move!(D4, D5), "d5+");
    }

    #[test]
    fn test_algebraic_notation_for_discovered_check_with_capture() {
        let mut board = chess_position! {
            ........
            ........
            ........
            ....p...
            ..RP...k
            ........
            ........
            ........
        };
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        assert_move_has_algebraic_notation!(
            &mut board,
            Color::White,
            std_move!(D4, E5, (Piece::Pawn, Color::Black)),
            "dxe5+"
        );
    }
}
