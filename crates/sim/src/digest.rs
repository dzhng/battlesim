//! FNV-1a 64: a stable, dependency-free digest for replay and parity checks.
use crate::math::V2;

pub struct Digest(u64);

impl Default for Digest {
    fn default() -> Self {
        Digest(0xcbf2_9ce4_8422_2325)
    }
}

impl Digest {
    pub fn bytes(&mut self, bytes: &[u8]) -> &mut Self {
        for b in bytes {
            self.0 ^= *b as u64;
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
        self
    }

    pub fn u64(&mut self, v: u64) -> &mut Self {
        self.bytes(&v.to_le_bytes())
    }

    /// Exact bit pattern: same-build replays must match bit for bit.
    pub fn f64(&mut self, v: f64) -> &mut Self {
        self.u64(v.to_bits())
    }

    /// Optional values carry a presence tag, so different states never hash alike.
    pub fn opt_v2(&mut self, p: Option<V2>) -> &mut Self {
        self.u64(p.is_some() as u64);
        if let Some(p) = p {
            self.f64(p.x).f64(p.y);
        }
        self
    }

    pub fn opt_f64(&mut self, v: Option<f64>) -> &mut Self {
        self.u64(v.is_some() as u64);
        if let Some(v) = v {
            self.f64(v);
        }
        self
    }

    pub fn finish(&self) -> u64 {
        self.0
    }
}

pub fn of_str(s: &str) -> u64 {
    Digest::default().bytes(s.as_bytes()).finish()
}
