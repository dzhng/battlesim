use super::{Point, Segment};
#[derive(Clone, Copy, Default, Debug)]
pub struct Metrics {
    pub nodes: usize,
    pub primitives: usize,
    pub pairs: usize,
}
#[derive(Clone, Copy)]
struct Node {
    bounds: [f64; 4],
    children: Option<[usize; 2]>,
    range: [usize; 2],
}
pub struct RoadIndex {
    boxes: Vec<[f64; 4]>,
    order: Vec<usize>,
    nodes: Vec<Node>,
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
            order: (0..roads.len()).collect(),
            nodes: Vec::with_capacity(2 * roads.len()),
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
            let b = self.boxes[i];
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
            self.order[first..end].sort_by(|&a, &b| {
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
                    if overlaps(bounds, self.boxes[id]) {
                        found.push(id);
                    }
                }
            }
        }
        found.sort_unstable();
        found
    }
    pub fn near(&self, p: Point, radius: f64, metrics: &mut Metrics) -> Vec<usize> {
        self.query(
            [p.0 - radius, p.1 - radius, p.0 + radius, p.1 + radius],
            metrics,
        )
    }
}
