//! One learned tile of 16 × 16 cells, as maximal runs of equal cells: a run's
//! exclusive end, its four marks as one word (crater, scorch, tracks,
//! trampled, a byte each), and its clearing.

pub const TILE: usize = 16;
pub const CELLS: usize = TILE * TILE;

#[derive(Clone, Copy, PartialEq, Debug)]
struct Run {
    end: u16,
    word: u32,
    cleared: u8,
}

/// One edit of a publication's run: `len` cells from `start` set to `word`
/// and `cleared`.
#[derive(Clone, Copy)]
pub struct Edit {
    pub start: usize,
    pub len: usize,
    pub word: u32,
    pub cleared: u8,
}

#[derive(Clone)]
pub struct Page {
    runs: Vec<Run>,
    /// The visible word every valid cell shares, if they do.
    pub uniform: Option<u32>,
    /// Pool words the page takes when it varies (0 when uniform).
    pub scar_words: u32,
}

/// The word a cell is drawn with: clearing shows as full track wear.
pub fn projected(word: u32, cleared: u8) -> u32 {
    (word & 0xff00_ffff) | (((word >> 16) & 255).max(cleared as u32) << 16)
}

impl Page {
    pub fn blank() -> Page {
        Page {
            runs: vec![Run {
                end: CELLS as u16,
                word: 0,
                cleared: 0,
            }],
            uniform: Some(0),
            scar_words: 0,
        }
    }

    fn at(&self, c: usize) -> Run {
        self.runs[self.runs.partition_point(|r| r.end as usize <= c)]
    }

    /// Cell `c`'s four marks and its clearing, as published.
    pub fn cell(&self, c: usize) -> ([u8; 4], u8) {
        let r = self.at(c);
        (r.word.to_le_bytes(), r.cleared)
    }

    pub fn marked(&self) -> bool {
        self.runs.iter().any(|r| r.word != 0 || r.cleared > 0)
    }

    /// Applies one tile's edits in order; returns the change in cleared cells.
    pub fn apply(&mut self, edits: &[Edit]) -> i64 {
        let mut words = [0u32; CELLS];
        let mut cleared = [0u8; CELLS];
        let mut start = 0;
        for r in &self.runs {
            words[start..r.end as usize].fill(r.word);
            cleared[start..r.end as usize].fill(r.cleared);
            start = r.end as usize;
        }
        let mut delta = 0;
        for e in edits {
            for c in e.start..e.start + e.len {
                delta += (e.cleared > 0) as i64 - (cleared[c] > 0) as i64;
                words[c] = e.word;
                cleared[c] = e.cleared;
            }
        }
        self.runs.clear();
        for c in 0..CELLS {
            match self.runs.last_mut() {
                Some(r) if r.word == words[c] && r.cleared == cleared[c] => r.end += 1,
                _ => self.runs.push(Run {
                    end: c as u16 + 1,
                    word: words[c],
                    cleared: cleared[c],
                }),
            }
        }
        delta
    }

    /// The visible word all `width` × `height` valid cells share, if any.
    pub fn projected_uniform(&self, width: usize, height: usize) -> Option<u32> {
        let mut words = (0..height)
            .flat_map(|y| (0..width).map(move |x| y * TILE + x))
            .map(|c| self.at(c))
            .map(|r| projected(r.word, r.cleared));
        let first = words.next()?;
        words.all(|w| w == first).then_some(first)
    }

    /// The runs the GPU reads, (exclusive end, visible word), coalescing
    /// runs that look alike; returns how many.
    pub fn scar_runs(&self, out: &mut Vec<[u32; 2]>) -> usize {
        out.clear();
        for r in &self.runs {
            let value = projected(r.word, r.cleared);
            match out.last_mut() {
                Some(last) if last[1] == value => last[0] = r.end as u32,
                _ => out.push([r.end as u32, value]),
            }
        }
        out.len()
    }

    /// The pool words a varying page takes: a power of two from 16 to 256.
    pub fn words_for(runs: usize) -> u32 {
        ((runs * 2).clamp(16, CELLS) as u32).next_power_of_two()
    }

    /// Each maximal span of cleared cells, (start, length).
    pub fn cleared_spans(&self, mut visit: impl FnMut(usize, usize)) {
        let (mut lo, mut start) = (None, 0);
        for r in &self.runs {
            match (r.cleared > 0, lo) {
                (true, None) => lo = Some(start),
                (false, Some(l)) => {
                    visit(l, start - l);
                    lo = None;
                }
                _ => {}
            }
            start = r.end as usize;
        }
        if let Some(l) = lo {
            visit(l, CELLS - l);
        }
    }
}
