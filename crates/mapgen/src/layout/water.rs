//! A plan's water as its neighbours ask about it: how much open ground lies
//! between the water's edge and a point, a line or a ring, and which bank a
//! place is on. The distances are the contract's own
//! (`contract::river::section`), found through buckets.
use super::geometry::{
    add, bearing, scale, segment_bounds, segment_crossing, segment_distance, sub, Grid, Point,
};
use contract::river::{edges, section, River, RiverSample};

pub struct Water<'a> {
    rivers: &'a [River],
    /// Every stretch between two rounded samples: its river and first sample.
    stretches: Vec<(usize, usize)>,
    grid: Grid,
}

impl<'a> Water<'a> {
    pub fn new(rivers: &'a [River], size: [f64; 2]) -> Self {
        let mut grid = Grid::new(size, 64.0);
        let mut stretches = Vec::new();
        for (river, line) in rivers.iter().enumerate() {
            for (sample, pair) in line.samples().windows(2).enumerate() {
                let half = pair[0].half_width_m.max(pair[1].half_width_m);
                grid.insert(
                    segment_bounds(pair[0].xy, pair[1].xy, half),
                    stretches.len() as u32,
                );
                stretches.push((river, sample));
            }
        }
        Self {
            rivers,
            stretches,
            grid,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.stretches.is_empty()
    }

    pub fn rivers(&self) -> &'a [River] {
        self.rivers
    }

    fn stretch(&self, id: u32) -> [&'a RiverSample; 2] {
        let (river, sample) = self.stretches[id as usize];
        let samples = self.rivers[river].samples();
        [&samples[sample], &samples[sample + 1]]
    }

    /// Open ground between `p` and the nearest water's edge, negative in the
    /// water; `within` when there is at least that much.
    pub fn gap(&self, p: Point, within: f64) -> f64 {
        let mut gap = within;
        let bounds = [p[0] - within, p[1] - within, p[0] + within, p[1] + within];
        self.grid.any(bounds, |id| {
            let [a, b] = self.stretch(id);
            gap = gap.min(-section(a, b, p).inside_m);
            false
        });
        gap
    }

    /// Open ground between the segment `ab` and the nearest water's edge,
    /// zero or less when it meets the water; `within` when there is at least
    /// that much.
    pub fn segment_gap(&self, a: Point, b: Point, within: f64) -> f64 {
        let mut gap = within;
        self.grid.any(segment_bounds(a, b, within), |id| {
            let [from, to] = self.stretch(id);
            let half = from.half_width_m.max(to.half_width_m);
            let apart = if segment_crossing(a, b, from.xy, to.xy).is_some() {
                0.0
            } else {
                segment_distance(a, b, from.xy)
                    .min(segment_distance(a, b, to.xy))
                    .min(segment_distance(from.xy, to.xy, a))
                    .min(segment_distance(from.xy, to.xy, b))
            };
            gap = gap.min(apart - half);
            false
        });
        gap
    }

    /// Open ground between a ring's edge and the nearest water's edge. A
    /// river runs from edge to edge of the map, so none lies wholly inside
    /// a ring whose edge it does not meet.
    pub fn ring_gap(&self, ring: &[Point], within: f64) -> f64 {
        contract::ground::edges(ring)
            .map(|(a, b)| self.segment_gap(*a, *b, within))
            .fold(within, f64::min)
    }

    /// A point of the water's edge and the unit vector away from the water
    /// there: `along` is the share of the way down the stretches, and `side`
    /// picks the bank. The water must not be empty.
    pub fn shore(&self, along: f64, side: f64) -> (Point, Point) {
        let id = (along * self.stretches.len() as f64) as usize;
        let [a, b] = self.stretch(id.min(self.stretches.len() - 1) as u32);
        let edge = edges(a, b, 0.0)[usize::from(side >= 0.5)];
        (edge, scale(sub(edge, a.xy), 1.0 / a.half_width_m))
    }

    /// Where the segment `ab` crosses a river's centreline, in order along
    /// it.
    pub fn crossings(&self, a: Point, b: Point) -> Vec<Crossing> {
        let mut ids = Vec::new();
        self.grid.any(segment_bounds(a, b, 0.0), |id| {
            ids.push(id);
            false
        });
        ids.sort_unstable();
        ids.dedup();
        let mut found: Vec<(f64, Crossing)> = ids
            .into_iter()
            .filter_map(|id| {
                let (river, sample) = self.stretches[id as usize];
                let [from, to] = self.stretch(id);
                let (share, along) = segment_crossing(a, b, from.xy, to.xy)?;
                // A crossing at a sample belongs to the stretch it starts.
                let last = sample + 2 == self.rivers[river].samples().len();
                (along < 1.0 || last).then(|| {
                    let crossing = Crossing {
                        at: add(a, scale(sub(b, a), share)),
                        downstream: bearing(from.xy, to.xy),
                    };
                    (share, crossing)
                })
            })
            .collect();
        found.sort_by(|x, y| x.0.total_cmp(&y.0));
        found.into_iter().map(|(_, crossing)| crossing).collect()
    }

    /// Which bank `p` stands on: whether a line from it due west to the
    /// map's edge crosses a river's middle an odd number of times. Rivers
    /// run from the north edge to the south, so with one river two places
    /// with the same answer have dry ground between them.
    pub fn bank(&self, p: Point) -> bool {
        self.crossings(p, [0.0, p[1]]).len() % 2 == 1
    }
}

/// One place a line crosses a river's middle.
#[derive(Clone, Copy, Debug)]
pub struct Crossing {
    pub at: Point,
    /// The bearing the river runs on there.
    pub downstream: f64,
}

/// The water as closed rings, a run of stretches each, bank to bank: what a
/// line of open ground stops at, and what a picture fills.
pub fn rings(rivers: &[River]) -> Vec<Vec<Point>> {
    // A ring is cut every few stretches, so a ray tests only those near it.
    let stretches = 24;
    let mut rings = Vec::new();
    for river in rivers {
        let samples = river.samples();
        // Both banks at every sample: across the stretch it starts, and for
        // the last sample across the stretch it ends.
        let banks: Vec<[Point; 2]> = (0..samples.len())
            .map(|i| {
                if i + 1 < samples.len() {
                    edges(&samples[i], &samples[i + 1], 0.0)
                } else {
                    edges(&samples[i - 1], &samples[i], 1.0)
                }
            })
            .collect();
        let mut from = 0;
        while from + 1 < banks.len() {
            let to = (from + stretches).min(banks.len() - 1);
            let ring = banks[from..=to]
                .iter()
                .map(|bank| bank[0])
                .chain(banks[from..=to].iter().rev().map(|bank| bank[1]))
                .collect();
            rings.push(ring);
            from = to;
        }
    }
    rings
}
