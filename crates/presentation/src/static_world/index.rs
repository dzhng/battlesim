//! A bounds hierarchy for immutable spatial batches. Leaves retain their input
//! identity; visibility and output ordering belong to the querying population.

type Bounds = [f64; 6];

struct Node {
    bounds: Bounds,
    leaf: Option<usize>,
    // First node after this subtree: rejected branches need no stack or scan.
    end: usize,
}

#[derive(Default)]
pub struct BoundsIndex {
    nodes: Vec<Node>,
}

impl BoundsIndex {
    pub fn new(bounds: impl IntoIterator<Item = Bounds>) -> Self {
        let mut leaves: Vec<_> = bounds.into_iter().enumerate().collect();
        let mut index = Self {
            nodes: Vec::with_capacity(leaves.len().saturating_mul(2)),
        };
        if !leaves.is_empty() {
            build(&mut leaves, &mut index.nodes);
        }
        index
    }

    /// Visit intersecting leaves by original identity and return bounds tested.
    /// The predicate must be conservative for a box enclosing other boxes.
    /// Traversal allocates nothing; consumers own their output buffers/order.
    pub fn query(
        &self,
        mut intersects: impl FnMut(&Bounds) -> bool,
        mut visit: impl FnMut(usize),
    ) -> usize {
        let (mut at, mut tested) = (0, 0);
        while at < self.nodes.len() {
            let node = &self.nodes[at];
            tested += 1;
            if intersects(&node.bounds) {
                if let Some(id) = node.leaf {
                    visit(id);
                }
                at += 1;
            } else {
                at = node.end;
            }
        }
        tested
    }
}

fn build(leaves: &mut [(usize, Bounds)], nodes: &mut Vec<Node>) {
    let mut bounds = leaves[0].1;
    for (_, b) in &leaves[1..] {
        for axis in 0..3 {
            bounds[axis] = bounds[axis].min(b[axis]);
            bounds[axis + 3] = bounds[axis + 3].max(b[axis + 3]);
        }
    }
    let at = nodes.len();
    nodes.push(Node {
        bounds,
        leaf: (leaves.len() == 1).then_some(leaves[0].0),
        end: 0,
    });
    if leaves.len() > 1 {
        let axis = (0..3)
            .max_by(|&a, &b| (bounds[a + 3] - bounds[a]).total_cmp(&(bounds[b + 3] - bounds[b])))
            .unwrap();
        let middle = leaves.len() / 2;
        leaves.select_nth_unstable_by(middle, |(aid, a), (bid, b)| {
            (a[axis] + a[axis + 3])
                .total_cmp(&(b[axis] + b[axis + 3]))
                .then(aid.cmp(bid))
        });
        let (left, right) = leaves.split_at_mut(middle);
        build(left, nodes);
        build(right, nodes);
    }
    nodes[at].end = nodes.len();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overlaps(a: &Bounds, b: &Bounds) -> bool {
        (0..3).all(|axis| a[axis] <= b[axis + 3] && a[axis + 3] >= b[axis])
    }

    #[test]
    fn queries_keep_crossing_and_touching_bounds_with_original_ids() {
        let boxes = [
            [-100., -1., -2., 100., 1., 2.],
            [2., 2., -10., 3., 3., 10.],
            [-4., -4., -4., -3., -3., -3.],
            [0., 0., 0., 0., 0., 0.],
        ];
        let index = BoundsIndex::new(boxes);
        for x in -6..6 {
            for z in -12..12 {
                let b = [x as f64, 0., z as f64, x as f64 + 2., 2., z as f64 + 2.];
                let mut actual = vec![];
                index.query(|a| overlaps(a, &b), |id| actual.push(id));
                actual.sort_unstable();
                let expected: Vec<_> = boxes
                    .iter()
                    .enumerate()
                    .filter_map(|(id, a)| overlaps(a, &b).then_some(id))
                    .collect();
                assert_eq!(actual, expected, "query={b:?}");
            }
        }
    }

    #[test]
    fn remote_growth_keeps_local_results_without_scanning_the_world() {
        let local = [-1., -1., -1., 1., 1., 1.];
        for count in [64_usize, 1024, 16384] {
            let boxes = std::iter::once(local).chain((0..count).map(|i| {
                let x = 1000. + i as f64 * 10.;
                [x, x, 0., x + 1., x + 1., 1.]
            }));
            let index = BoundsIndex::new(boxes);
            let mut ids = vec![];
            let visited = index.query(|b| overlaps(b, &local), |id| ids.push(id));
            assert_eq!(ids, [0]);
            assert!(visited <= 2 * (count + 1).next_power_of_two().ilog2() as usize + 1);
        }
        assert_eq!(
            BoundsIndex::default().query(|_| true, |_| panic!("empty")),
            0
        );
    }
}
