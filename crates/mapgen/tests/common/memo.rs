//! Maps the tests of a file share, made once.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Values made once per key, whichever tests ask and however many at once:
/// a second asker waits for the first to finish rather than making its own.
pub struct Memo<K, V>(Mutex<BTreeMap<K, Arc<OnceLock<Arc<V>>>>>);

impl<K: Ord, V> Memo<K, V> {
    pub const fn new() -> Self {
        Self(Mutex::new(BTreeMap::new()))
    }

    pub fn get(&self, key: K, make: impl FnOnce() -> V) -> Arc<V> {
        let cell = self.0.lock().unwrap().entry(key).or_default().clone();
        cell.get_or_init(|| Arc::new(make())).clone()
    }
}
