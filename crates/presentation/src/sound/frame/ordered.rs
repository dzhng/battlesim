//! The frame's insertion-ordered map.

/// An insertion-ordered map: the order loops and fires were first wanted
/// is the order they are stopped and ranked in.
pub(super) struct Ordered<K, T>(pub(super) Vec<(K, T)>);

impl<K: PartialEq, T> Ordered<K, T> {
    pub(super) fn get(&self, key: &K) -> Option<&T> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
    pub(super) fn get_mut(&mut self, key: &K) -> Option<&mut T> {
        self.0.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v)
    }
    pub(super) fn remove(&mut self, key: &K) -> Option<T> {
        let i = self.0.iter().position(|(k, _)| k == key)?;
        Some(self.0.remove(i).1)
    }
    /// Sets `key`, keeping its place if it was already there.
    pub(super) fn set(&mut self, key: K, value: T) {
        match self.get_mut(&key) {
            Some(v) => *v = value,
            None => self.0.push((key, value)),
        }
    }
}

impl<K, T> Default for Ordered<K, T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}
