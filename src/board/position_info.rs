use rustc_hash::FxHashMap;

pub struct PositionInfo {
    position_count: FxHashMap<u64, u8>,
    max_seen_position_count_stack: Vec<u8>,
    current_position_hash: u64,
}

impl Default for PositionInfo {
    fn default() -> Self {
        Self {
            position_count: FxHashMap::default(),
            max_seen_position_count_stack: vec![1],
            current_position_hash: 0,
        }
    }
}

impl PositionInfo {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn count_current_position(&mut self) -> u8 {
        self.position_count
            .entry(self.current_position_hash)
            .and_modify(|count| *count += 1)
            .or_insert(1);
        let count = *self
            .position_count
            .get(&self.current_position_hash)
            .unwrap();
        self.max_seen_position_count_stack.push(count);
        count
    }

    pub fn uncount_current_position(&mut self) -> u8 {
        self.position_count
            .entry(self.current_position_hash)
            .and_modify(|count| *count -= 1);
        self.max_seen_position_count_stack.pop();
        *self
            .position_count
            .get(&self.current_position_hash)
            .unwrap()
    }

    pub fn max_seen_position_count(&self) -> u8 {
        *self.max_seen_position_count_stack.last().unwrap()
    }

    // TODO(codyjk): Replace this with Zobrist hashing

    pub fn current_position_hash(&self) -> u64 {
        self.current_position_hash
    }

    pub fn update_position_hash(&mut self, hash: u64) -> u64 {
        self.current_position_hash = hash;
        hash
    }
}
