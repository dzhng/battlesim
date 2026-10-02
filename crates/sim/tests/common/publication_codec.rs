/// Independent test reader for the layout-owned transport, returning exact words.
pub fn group(wire: &[f32], metadata: usize, old: &[f32]) -> Vec<f32> {
    let n = wire[metadata] as usize;
    let mode = wire[metadata + 1] as usize;
    let p = &wire[metadata + 3..metadata + 3 + wire[metadata + 2] as usize];
    let baseline: Vec<_> = old.iter().map(|v| v.to_bits()).collect();
    let words = if mode == 3 {
        let mut bits = Bits {
            words: p.iter().map(|v| v.to_bits()).collect(),
            at: 0,
        };
        let form = bits.read(8);
        let mut result = if form == 0 {
            let mut v = baseline.clone();
            v.resize(n, 0);
            v
        } else {
            Vec::new()
        };
        if form == 1 {
            for _ in 0..n {
                result.push(bits.literal(0));
            }
        } else if form == 0 {
            for _ in 0..bits.integer() {
                let start = bits.integer() as usize;
                let count = bits.integer() as usize;
                for (at, value) in result.iter_mut().enumerate().skip(start).take(count) {
                    *value = bits.literal(baseline.get(at).copied().unwrap_or(0));
                }
            }
        } else {
            assert_eq!(form, 2);
            while result.len() < n {
                let source = bits.integer() as usize;
                let count = bits.integer() as usize;
                if source == 0 {
                    for _ in 0..count {
                        result.push(bits.literal(baseline.get(result.len()).copied().unwrap_or(0)));
                    }
                } else {
                    result.extend_from_slice(&baseline[source - 1..source - 1 + count]);
                }
            }
        }
        assert!(bits.words.len() * 32 - bits.at <= 31);
        while bits.at < bits.words.len() * 32 {
            assert_eq!(bits.read(1), 0);
        }
        result
    } else if mode == 1 {
        p.iter().map(|v| v.to_bits()).collect()
    } else {
        let mut result = if mode == 0 {
            let mut v = baseline.clone();
            v.resize(n, 0);
            v
        } else {
            Vec::new()
        };
        let mut at = 0;
        while at < p.len() {
            let source = p[at];
            let count = p[at + 1] as usize;
            at += 2;
            if mode == 0 {
                for i in 0..count {
                    result[source as usize + i] = p[at + i].to_bits();
                }
                at += count;
            } else if source == -1.0 {
                result.extend(p[at..at + count].iter().map(|v| v.to_bits()));
                at += count;
            } else {
                result.extend_from_slice(&baseline[source as usize..source as usize + count]);
            }
        }
        result
    };
    assert_eq!(words.len(), n);
    words.into_iter().map(f32::from_bits).collect()
}
struct Bits {
    words: Vec<u32>,
    at: usize,
}
impl Bits {
    fn read(&mut self, n: usize) -> u32 {
        assert!(self.at + n <= self.words.len() * 32);
        let i = self.at / 32;
        let shift = self.at % 32;
        let mut v = self.words[i] as u64 >> shift;
        if shift + n > 32 {
            v |= (self.words[i + 1] as u64) << (32 - shift);
        }
        self.at += n;
        (v & ((1u64 << n) - 1)) as u32
    }
    fn integer(&mut self) -> u32 {
        let mut v = 0;
        for byte in 0..4 {
            let b = self.read(8);
            v |= (b & 127) << (byte * 7);
            if b < 128 {
                return v;
            }
        }
        panic!("oversize compact integer");
    }
    fn literal(&mut self, old: u32) -> u32 {
        let tag = self.read(4);
        assert!(tag < 10);
        let n = if tag < 5 { tag } else { tag - 5 };
        let v = if n == 0 { 0 } else { self.read(n as usize * 8) };
        if tag < 5 {
            v
        } else {
            v ^ old
        }
    }
}
