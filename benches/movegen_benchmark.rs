use chess::board::color::Color;
use chess::board::piece::Piece;
use chess::board::{square, Board};

use chess::move_generation::generate_rook_moves;
use chess::move_generation::targets::Targets;
use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let targets = &Targets::new();
    c.bench_function("generate rook moves", |b| {
        b.iter(|| test_generate_rook_moves(targets))
    });
}

fn test_generate_rook_moves(targets: &Targets) {
    let mut board = Board::new();
    board.put(square::A3, Piece::Pawn, Color::White).unwrap();
    board.put(square::H3, Piece::Pawn, Color::Black).unwrap();
    board.put(square::C3, Piece::Rook, Color::White).unwrap();
    board.put(square::C1, Piece::King, Color::White).unwrap();
    board.put(square::C7, Piece::Pawn, Color::White).unwrap();
    let mut rook_moves = vec![];
    generate_rook_moves(&mut rook_moves, &board, Color::White, targets);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
