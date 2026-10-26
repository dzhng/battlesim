//! The packed delivery's bit cursor.
use super::{check, Result};

/// Reads lsb-first bit fields from raw u32 carrier words.
pub(super) struct Bits<'a> {
    pub(super) words: &'a [u32],
    pub(super) bit: usize,
}

impl Bits<'_> {
    pub(super) fn read(&mut self, count: usize) -> Result<u32> {
        check(
            self.bit + count <= self.words.len() * 32,
            "truncated packed publication",
        )?;
        let (index, shift) = (self.bit / 32, self.bit % 32);
        let mut value = (self.words[index] as u64) >> shift;
        if shift + count > 32 {
            value |= (self.words[index + 1] as u64) << (32 - shift);
        }
        self.bit += count;
        Ok((value & ((1u64 << count) - 1)) as u32)
    }

    /// A canonical unsigned LEB128 of at most four bytes.
    pub(super) fn integer(&mut self) -> Result<u32> {
        let mut value = 0;
        for byte in 0..4 {
            let part = self.read(8)?;
            value += (part & 127) << (byte * 7);
            if part < 128 {
                check(
                    byte == 0 || part != 0,
                    "noncanonical packed publication integer",
                )?;
                return Ok(value);
            }
        }
        Err("oversized packed publication integer".into())
    }

    /// Zero padding to the next carrier word, and no more.
    pub(super) fn finish(&mut self) -> Result<()> {
        check(
            self.words.len() * 32 - self.bit <= 31,
            "excess packed publication padding",
        )?;
        while self.bit < self.words.len() * 32 {
            check(self.read(1)? == 0, "nonzero packed publication padding")?;
        }
        Ok(())
    }
}
