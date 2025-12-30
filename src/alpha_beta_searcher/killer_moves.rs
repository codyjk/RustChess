//! Killer move storage using thread-local storage for parallel search.

use thread_local::ThreadLocal;

type KillerMovePair = [Option<Box<dyn std::any::Any + Send + Sync>>; 2];
type KillerMovesVec = Vec<KillerMovePair>;
type KillerMovesStorage = std::cell::RefCell<Option<KillerMovesVec>>;
static KILLER_MOVES: ThreadLocal<KillerMovesStorage> = ThreadLocal::new();

/// Creates a new killer move storage vector.
fn create_killer_vec(max_ply: usize) -> KillerMovesVec {
    (0..=max_ply).map(|_| [None, None]).collect()
}

/// Manages killer moves using thread-local storage for parallel search.
///
/// Killer moves are quiet moves that caused beta cutoffs at the same ply in
/// other branches of the search tree. Storing them per-ply improves move
/// ordering by prioritizing moves likely to cause cutoffs.
pub(crate) struct KillerMovesManager {
    max_depth: usize,
}

impl KillerMovesManager {
    pub fn new(max_depth: u8) -> Self {
        Self {
            max_depth: max_depth as usize,
        }
    }

    fn ensure_storage(&self) {
        let storage = KILLER_MOVES.get_or(|| std::cell::RefCell::new(None));
        let mut storage_ref = storage.borrow_mut();

        let needs_init = storage_ref.is_none()
            || storage_ref
                .as_ref()
                .map_or(false, |killers| killers.len() <= self.max_depth);

        if needs_init {
            *storage_ref = Some(create_killer_vec(self.max_depth));
        }
    }

    pub fn store<M: Clone + Send + Sync + 'static>(&self, ply: u8, killer: M) {
        let ply = ply as usize;
        self.ensure_storage();

        let storage = KILLER_MOVES.get().expect("storage should be initialized");
        if let Some(ref mut killers) = *storage.borrow_mut() {
            if ply < killers.len() {
                let old_first = killers[ply][0].take();
                killers[ply][1] = old_first;
                killers[ply][0] = Some(Box::new(killer));
            }
        }
    }

    pub fn get<M: Clone + 'static>(&self, ply: u8) -> [Option<M>; 2] {
        let ply = ply as usize;
        self.ensure_storage();

        let storage = KILLER_MOVES.get().expect("storage should be initialized");
        if let Some(ref killers) = *storage.borrow() {
            if ply < killers.len() {
                let mut result = [None, None];
                for (i, stored) in killers[ply].iter().enumerate() {
                    if let Some(boxed) = stored {
                        if let Some(killer) = boxed.downcast_ref::<M>() {
                            result[i] = Some(killer.clone());
                        }
                    }
                }
                return result;
            }
        }
        [None, None]
    }

    pub fn clear(&self) {
        if let Some(storage) = KILLER_MOVES.get() {
            if let Some(ref mut killers) = *storage.borrow_mut() {
                for killer in killers.iter_mut() {
                    *killer = [
                        None::<Box<dyn std::any::Any + Send + Sync>>,
                        None::<Box<dyn std::any::Any + Send + Sync>>,
                    ];
                }
            }
        }
    }
}
