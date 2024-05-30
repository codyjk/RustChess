use rand::Rng;

pub fn generate_random_u64() -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen::<u64>()
}
