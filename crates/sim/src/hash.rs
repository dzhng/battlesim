//! Hash maps keyed by numbers the simulation makes itself: cell indices,
//! bucket coordinates, float bits. No adversary picks them, and the standard
//! hasher costs more than such a lookup saves.
use std::collections::HashMap;

/// A map hashed a word at a time ([`BuildWords`]). Its order is never read:
/// nothing the simulation decides may follow from how a map lays its keys.
pub type WordMap<K, V> = HashMap<K, V, BuildWords>;

#[derive(Clone, Copy, Debug, Default)]
pub struct BuildWords;

impl std::hash::BuildHasher for BuildWords {
    type Hasher = Words;
    fn build_hasher(&self) -> Words {
        Words(0)
    }
}

pub struct Words(u64);

impl std::hash::Hasher for Words {
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.write_u64(u64::from_le_bytes(word));
        }
    }
    fn write_u64(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn write_usize(&mut self, word: usize) {
        self.write_u64(word as u64);
    }
    fn write_u32(&mut self, word: u32) {
        self.write_u64(u64::from(word));
    }
    fn write_u8(&mut self, word: u8) {
        self.write_u64(u64::from(word));
    }
    fn finish(&self) -> u64 {
        self.0
    }
}
