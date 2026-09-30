use super::{Point, Segment};
#[derive(Clone, Copy, Default, Debug)]
pub struct Metrics {
    pub nodes: usize,
    pub primitives: usize,
    pub pairs: usize,
    pub sort_comparisons: usize,
}
#[derive(Clone, Copy)]
struct Node {
    bounds: [f64; 4],
    children: Option<[usize; 2]>,
    range: [usize; 2],
}
pub struct RoadIndex {
    boxes: Vec<[f64; 4]>,
    order: Vec<u32>,
    nodes: Vec<Node>,
    pub build_comparisons: usize,
    pub build_bounds: usize,
}
fn overlaps(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] <= b[2] && a[2] >= b[0] && a[1] <= b[3] && a[3] >= b[1]
}
impl RoadIndex {
    pub fn new(roads: &[Segment]) -> Self {
        let mut index = Self {
            boxes: roads
                .iter()
                .map(|r| {
                    [
                        r.a.0.min(r.b.0),
                        r.a.1.min(r.b.1),
                        r.a.0.max(r.b.0),
                        r.a.1.max(r.b.1),
                    ]
                })
                .collect(),
            order: (0..roads.len())
                .map(|i| u32::try_from(i).expect("source index admission"))
                .collect(),
            nodes: Vec::with_capacity(2 * roads.len()),
            build_comparisons: 0,
            build_bounds: 0,
        };
        if !roads.is_empty() {
            index.build(0, roads.len());
        }
        index
    }
    fn build(&mut self, first: usize, end: usize) -> usize {
        let mut bounds = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for &i in &self.order[first..end] {
            self.build_bounds += 1;
            let b = self.boxes[i as usize];
            bounds[0] = bounds[0].min(b[0]);
            bounds[1] = bounds[1].min(b[1]);
            bounds[2] = bounds[2].max(b[2]);
            bounds[3] = bounds[3].max(b[3]);
        }
        let at = self.nodes.len();
        self.nodes.push(Node {
            bounds,
            children: None,
            range: [first, end],
        });
        if end - first > 8 {
            let axis = usize::from(bounds[3] - bounds[1] > bounds[2] - bounds[0]);
            let boxes = &self.boxes;
            let comparisons = &mut self.build_comparisons;
            self.order[first..end].sort_by(|&a, &b| {
                *comparisons += 1;
                let (a, b) = (a as usize, b as usize);
                ((boxes[a][axis] + boxes[a][axis + 2]) * 0.5)
                    .total_cmp(&((boxes[b][axis] + boxes[b][axis + 2]) * 0.5))
                    .then(a.cmp(&b))
            });
            let mid = first + (end - first) / 2;
            let left = self.build(first, mid);
            let right = self.build(mid, end);
            self.nodes[at].children = Some([left, right]);
        }
        at
    }
    pub fn bounds(&self, id: usize) -> [f64; 4] {
        self.boxes[id]
    }
    pub fn query(&self, bounds: [f64; 4], metrics: &mut Metrics) -> Vec<usize> {
        let mut found = Vec::new();
        let mut stack = if self.nodes.is_empty() {
            Vec::new()
        } else {
            vec![0]
        };
        while let Some(at) = stack.pop() {
            metrics.nodes += 1;
            let n = self.nodes[at];
            if !overlaps(bounds, n.bounds) {
                continue;
            }
            if let Some([left, right]) = n.children {
                stack.push(right);
                stack.push(left);
            } else {
                for &id in &self.order[n.range[0]..n.range[1]] {
                    metrics.primitives += 1;
                    if overlaps(bounds, self.boxes[id as usize]) {
                        found.push(id as usize);
                    }
                }
            }
        }
        found.sort_unstable_by(|a, b| {
            metrics.sort_comparisons += 1;
            a.cmp(b)
        });
        found
    }
    pub fn near(&self, p: Point, radius: f64, metrics: &mut Metrics) -> Vec<usize> {
        self.query(
            [p.0 - radius, p.1 - radius, p.0 + radius, p.1 + radius],
            metrics,
        )
    }
}

// Request-local traversal: no result vector, sort or whole-query scan.
pub struct QueryCursor {
    stack: [usize; 64],
    len: usize,
    leaf: Option<(usize, usize)>,
    bounds: [f64; 4],
    pub nodes: usize,
    pub primitives: usize,
}
pub enum QueryStep {
    Pending,
    Hit(usize),
    Done,
}
impl QueryCursor {
    pub fn new(index: &RoadIndex, p: Point, r: f64) -> Self {
        let mut stack = [0; 64];
        let len = usize::from(!index.nodes.is_empty());
        stack[0] = 0;
        Self {
            stack,
            len,
            leaf: None,
            bounds: [p.0 - r, p.1 - r, p.0 + r, p.1 + r],
            nodes: 0,
            primitives: 0,
        }
    }
    pub fn step(&mut self, index: &RoadIndex) -> QueryStep {
        if let Some((at, end)) = self.leaf {
            self.primitives += 1;
            self.leaf = if at + 1 < end {
                Some((at + 1, end))
            } else {
                None
            };
            let id = index.order[at] as usize;
            return if overlaps(self.bounds, index.boxes[id]) {
                QueryStep::Hit(id)
            } else {
                QueryStep::Pending
            };
        }
        if self.len == 0 {
            return QueryStep::Done;
        }
        self.len -= 1;
        let n = index.nodes[self.stack[self.len]];
        self.nodes += 1;
        if overlaps(self.bounds, n.bounds) {
            if let Some([left, right]) = n.children {
                assert!(self.len + 2 <= 64, "balanced index depth admission");
                self.stack[self.len] = right;
                self.stack[self.len + 1] = left;
                self.len += 2;
            } else if n.range[0] < n.range[1] {
                self.leaf = Some((n.range[0], n.range[1]));
            }
        }
        QueryStep::Pending
    }
    pub fn digest(&self) -> u64 {
        let mut h = super::fold(
            self.len as u64,
            super::fold(self.nodes as u64, self.primitives as u64),
        );
        for &v in &self.stack[..self.len] {
            h = super::fold(h, v as u64);
        }
        if let Some((a, b)) = self.leaf {
            h = super::fold(h, super::fold(a as u64, b as u64));
        }
        h
    }
}
