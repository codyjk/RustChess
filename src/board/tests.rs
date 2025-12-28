use crate::chess_move::chess_move_effect::ChessMoveEffect;
use crate::{castle_kingside, std_move};

use super::*;
use crate::chess_move::castle::CastleChessMove;
use crate::chess_move::chess_move::ChessMove;
use crate::chess_move::standard::StandardChessMove;
use common::bitboard::square::*;

#[test]
fn test_zobrist_hashing_is_equal_for_transpositions() {
    let mut board1 = Board::default();
    let mut board2 = Board::default();
    let initial_hash_1 = board1.current_position_hash();
    let initial_hash_2 = board2.current_position_hash();
    assert_eq!(initial_hash_1, initial_hash_2);

    let board1_moves = vec![
        std_move!(E2, E4),
        std_move!(E7, E5),
        std_move!(G1, F3),
        std_move!(B8, C6),
        std_move!(F1, C4),
        std_move!(G8, F6),
        castle_kingside!(Color::White),
    ];

    let board2_moves = vec![
        std_move!(G1, F3),
        std_move!(B8, C6),
        std_move!(E2, E4),
        std_move!(E7, E5),
        std_move!(F1, C4),
        std_move!(G8, F6),
        castle_kingside!(Color::White),
    ];

    let mut board1_hashes = vec![initial_hash_1];
    let mut board2_hashes = vec![initial_hash_2];

    for (move1, move2) in board1_moves.iter().zip(board2_moves.iter()) {
        move1.apply(&mut board1).unwrap();
        move2.apply(&mut board2).unwrap();
        board1_hashes.push(board1.current_position_hash());
        board2_hashes.push(board2.current_position_hash());
    }
    assert_eq!(
        board1.current_position_hash(),
        board2.current_position_hash()
    );

    // undo the moves and see that we get back to the same position
    board1_hashes.pop();
    board2_hashes.pop();
    for (move1, move2) in board1_moves.iter().rev().zip(board2_moves.iter().rev()) {
        println!("undoing moves {} and {}", move1, move2);
        move1.undo(&mut board1).unwrap();
        move2.undo(&mut board2).unwrap();
        println!(
            "hashes: {} and {}",
            board1.current_position_hash(),
            board2.current_position_hash()
        );
        // compare to the last hash in the vec
        assert_eq!(
            board1.current_position_hash(),
            board1_hashes.pop().unwrap(),
            "hash 1 should be equal after undoing moves"
        );
        assert_eq!(
            board2.current_position_hash(),
            board2_hashes.pop().unwrap(),
            "hash 2 should be equal after undoing moves"
        );
    }
    assert_eq!(
        board1.current_position_hash(),
        board2.current_position_hash(),
        "hashes should be equal after undoing moves"
    );
    assert_eq!(
        initial_hash_1,
        board1.current_position_hash(),
        "hashes should be equal to the initial hash"
    );
    assert_eq!(
        initial_hash_2,
        board2.current_position_hash(),
        "hashes should be equal to the initial hash"
    );
}

