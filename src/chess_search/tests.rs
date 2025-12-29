//! Chess-specific tests for the alpha-beta search.

use super::*;
use crate::alpha_beta_searcher::SearchContext;
use crate::board::castle_rights::CastleRights;
use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::chess_move::capture::Capture;
use crate::chess_move::chess_move_effect::ChessMoveEffect;
use crate::chess_move::standard::StandardChessMove;
use crate::{check_move, checkmate_move, chess_position, std_move};
use common::bitboard::*;

#[test]
fn test_find_mate_in_1_white() {
    let mut context = SearchContext::new(4);

    let mut board = chess_position! {
        .Q......
        ........
        ........
        ........
        ........
        ........
        k.K.....
        ........
    };
    board.set_turn(Color::White);
        board.lose_castle_rights(CastleRights::all());
    println!("Testing board:\n{}", board);

    let chess_move = search_best_move(&mut context, &mut board).unwrap();
    let valid_checkmates = [
        checkmate_move!(std_move!(B8, B2)),
        checkmate_move!(std_move!(B8, A8)),
        checkmate_move!(std_move!(B8, A7)),
    ];
    assert!(
        valid_checkmates.contains(&chess_move),
        "{} does not lead to checkmate",
        chess_move
    );
}

#[test]
fn test_find_mate_in_1_black() {
    let mut context = SearchContext::new(4);

    let mut board = chess_position! {
        .q......
        ........
        ........
        ........
        ........
        ........
        K.k.....
        ........
    };
    board.set_turn(Color::Black);
        board.lose_castle_rights(CastleRights::all());

    println!("Testing board:\n{}", board);

    let chess_move = search_best_move(&mut context, &mut board).unwrap();

    let valid_checkmates = [
        checkmate_move!(std_move!(B8, B2)),
        checkmate_move!(std_move!(B8, A8)),
        checkmate_move!(std_move!(B8, A7)),
    ];
    assert!(
        valid_checkmates.contains(&chess_move),
        "{} does not lead to checkmate",
        chess_move
    );
}

#[test]
fn test_find_back_rank_mate_in_2_white() {
    let mut context = SearchContext::new(4);

    let mut board = chess_position! {
        .k.....r
        ppp.....
        ........
        ........
        ........
        ........
        ...Q....
        K..R....
    };
    board.set_turn(Color::White);
        board.lose_castle_rights(CastleRights::all());

    println!("Testing board:\n{}", board);

    let expected_moves = [
        check_move!(std_move!(D2, D8)),
        std_move!(H8, D8, Capture(Piece::Queen)),
        checkmate_move!(std_move!(D1, D8, Capture(Piece::Rook))),
    ];
    let mut expected_move_iter = expected_moves.iter();

    let move1 = search_best_move(&mut context, &mut board).unwrap();
    move1.apply(&mut board).unwrap();
    board.toggle_turn();
    assert_eq!(expected_move_iter.next().unwrap(), &move1);
    println!("Testing board:\n{}", board);

    let move2 = search_best_move(&mut context, &mut board).unwrap();
    move2.apply(&mut board).unwrap();
    board.toggle_turn();
    assert_eq!(expected_move_iter.next().unwrap(), &move2);
    println!("Testing board:\n{}", board);

    let move3 = search_best_move(&mut context, &mut board).unwrap();
    move3.apply(&mut board).unwrap();
    board.toggle_turn();
    assert_eq!(expected_move_iter.next().unwrap(), &move3);
    println!("Testing board:\n{}", board);
}

#[test]
fn test_find_back_rank_mate_in_2_black() {
    let mut context = SearchContext::new(4);

    let mut board = chess_position! {
        ....r..k
        ....q...
        ........
        ........
        ........
        ........
        .....PPP
        R.....K.
    };
    board.set_turn(Color::Black);
        board.lose_castle_rights(CastleRights::all());

    println!("Testing board:\n{}", board);

    let expected_moves = [
        check_move!(std_move!(E7, E1)),
        std_move!(A1, E1, Capture(Piece::Queen)),
        checkmate_move!(std_move!(E8, E1, Capture(Piece::Rook))),
    ];
    let mut expected_move_iter = expected_moves.iter();

    let move1 = search_best_move(&mut context, &mut board).unwrap();
    move1.apply(&mut board).unwrap();
    board.toggle_turn();
    assert_eq!(
        expected_move_iter.next().unwrap(),
        &move1,
        "failed to find first move of mate in 2"
    );
    println!("Testing board:\n{}", board);

    let move2 = search_best_move(&mut context, &mut board).unwrap();
    move2.apply(&mut board).unwrap();
    board.toggle_turn();
    assert_eq!(
        expected_move_iter.next().unwrap(),
        &move2,
        "failed to find second move of mate in 2"
    );
    println!("Testing board:\n{}", board);

    let move3 = search_best_move(&mut context, &mut board).unwrap();
    move3.apply(&mut board).unwrap();
    board.toggle_turn();
    assert_eq!(
        expected_move_iter.next().unwrap(),
        &move3,
        "failed to find third move of mate in 2"
    );
    println!("Testing board:\n{}", board);
}
