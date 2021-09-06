use chess::game::Game;

use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("make random moves 100", |b| {
        b.iter(|| computer_vs_computer())
    });
}

fn computer_vs_computer() {
    let game = &mut Game::new();
    let mut moves = 0;

    loop {
        moves += 1;
        if moves > 250 {
            break;
        }

        match game.make_random_move() {
            Ok(_chessmove) => {
                game.next_turn();
                continue;
            }
            Err(_error) => {
                break;
            }
        }
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
