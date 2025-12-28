use fastrand;
use std::time::Duration;

use chess::board::Board;
use chess::game::{engine::EngineConfig, mode::ComputerVsComputer, r#loop::GameLoop};
use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    fastrand::seed(1337);

    c.bench_function("computer vs computer (depth 3)", |b| {
        b.iter(|| computer_vs_computer(25, 3))
    });

    c.bench_function("computer vs computer (depth 4)", |b| {
        b.iter(|| computer_vs_computer(10, 4))
    });
}

fn times(n: usize) -> impl Iterator {
    std::iter::repeat(()).take(n)
}

fn computer_vs_computer(game_count: usize, search_depth: u8) {
    for _ in times(game_count) {
        let config = EngineConfig {
            search_depth,
            starting_position: Default::default(),
        };
        let mode = ComputerVsComputer {
            delay_between_moves: Some(Duration::ZERO),
        };
        let mut game = GameLoop::new(mode, config);
        game.run();
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
