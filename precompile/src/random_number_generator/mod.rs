use rand::Rng;

/// Generates a random u64. This is needed for both Zobrist tables and magic bitboard generation.
pub fn generate_random_u64() -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen::<u64>()
}
