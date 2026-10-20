//! Spatial draw ranges over immutable ground triangles. Whole triangles retain
//! their exact vertex bytes; tile bounds include corners crossing tile edges.
//! Both clients select with the camera matrix they actually submit to the GPU.
use std::collections::BTreeMap;
use std::ops::Range;

use super::mesh::VERTEX_FLOATS;

/// Shared spatial batching size; never a visibility cutoff.
pub const CELL_M: f64 = 256.;

const TRIANGLE_FLOATS: usize = 3 * VERTEX_FLOATS;
type Bounds = [f64; 6];

struct Tile {
    bounds: Bounds,
    vertices: Range<u32>,
}
struct Node {
    bounds: Bounds,
    contents: Contents,
}
enum Contents {
    Tile(Range<u32>),
    Branch(usize, usize),
}

/// One upload and a spatial index. Building is map-wide; selection visits only
/// intersecting hierarchy branches. An infinite-far view remains infinite:
/// looking along the horizon can legitimately select the whole map.
pub struct TerrainPartition {
    vertices: Vec<f32>,
    nodes: Vec<Node>,
}

impl TerrainPartition {
    /// Group triangles by their centroid's tile. `cell_m` is a spatial batching
    /// choice, not a visibility distance or a change to terrain resolution.
    pub fn new(mesh: &[f32], cell_m: f64) -> Self {
        assert!(cell_m.is_finite() && cell_m > 0.0);
        assert_eq!(mesh.len() % TRIANGLE_FLOATS, 0);
        assert!(u32::try_from(mesh.len() / VERTEX_FLOATS).is_ok());
        let mut buckets: BTreeMap<(i64, i64), Vec<&[f32]>> = BTreeMap::new();
        for triangle in mesh.as_chunks::<TRIANGLE_FLOATS>().0 {
            assert!(triangle
                .as_chunks::<VERTEX_FLOATS>()
                .0
                .iter()
                .all(|v| v[..3].iter().all(|x| x.is_finite())));
            let cell = |axis| {
                let centre = (0..3)
                    .map(|i| f64::from(triangle[i * VERTEX_FLOATS + axis]))
                    .sum::<f64>()
                    / 3.0;
                (centre / cell_m).floor() as i64
            };
            buckets
                .entry((cell(0), cell(1)))
                .or_default()
                .push(triangle);
        }
        let mut vertices = Vec::with_capacity(mesh.len());
        let mut tiles = Vec::with_capacity(buckets.len());
        for triangles in buckets.into_values() {
            let start = (vertices.len() / VERTEX_FLOATS) as u32;
            let mut bounds = [f64::INFINITY; 6];
            bounds[3..].fill(f64::NEG_INFINITY);
            for triangle in triangles {
                for vertex in triangle.as_chunks::<VERTEX_FLOATS>().0 {
                    for axis in 0..3 {
                        bounds[axis] = bounds[axis].min(f64::from(vertex[axis]));
                        bounds[axis + 3] = bounds[axis + 3].max(f64::from(vertex[axis]));
                    }
                }
                vertices.extend_from_slice(triangle);
            }
            tiles.push(Tile {
                bounds,
                vertices: start..(vertices.len() / VERTEX_FLOATS) as u32,
            });
        }
        let mut nodes = Vec::with_capacity(tiles.len().saturating_mul(2));
        if !tiles.is_empty() {
            build(&mut tiles, &mut nodes);
        }
        Self { vertices, nodes }
    }

    pub fn vertices(&self) -> &[f32] {
        &self.vertices
    }

    /// Replace `ranges` with visible vertex ranges in buffer order. Returns
    /// bounds tested, so callers can distinguish spatial selection from a
    /// complete-map scan. Retain the caller-owned vector across frames.
    pub fn select(&self, view_projection: &[f32; 16], ranges: &mut Vec<Range<u32>>) -> usize {
        ranges.clear();
        let mut visited = 0;
        let planes = clip_planes(view_projection);
        if !self.nodes.is_empty() {
            self.visit(0, &planes, ranges, &mut visited);
        }
        ranges.sort_unstable_by_key(|range| range.start);
        // Merge adjacent selected tiles into one GPU submission.
        let mut count = 0;
        for i in 0..ranges.len() {
            if count > 0 && ranges[count - 1].end == ranges[i].start {
                ranges[count - 1].end = ranges[i].end;
            } else {
                ranges[count] = ranges[i].clone();
                count += 1;
            }
        }
        ranges.truncate(count);
        visited
    }

    fn visit(
        &self,
        index: usize,
        planes: &[ClipPlane; 6],
        out: &mut Vec<Range<u32>>,
        visited: &mut usize,
    ) {
        *visited += 1;
        let node = &self.nodes[index];
        if !intersects(&node.bounds, planes) {
            return;
        }
        match &node.contents {
            Contents::Tile(range) => out.push(range.clone()),
            Contents::Branch(a, b) => {
                self.visit(*a, planes, out, visited);
                self.visit(*b, planes, out, visited);
            }
        }
    }
}

fn build(tiles: &mut [Tile], nodes: &mut Vec<Node>) -> usize {
    let mut bounds = tiles[0].bounds;
    for tile in &tiles[1..] {
        for axis in 0..3 {
            bounds[axis] = bounds[axis].min(tile.bounds[axis]);
            bounds[axis + 3] = bounds[axis + 3].max(tile.bounds[axis + 3]);
        }
    }
    let index = nodes.len();
    nodes.push(Node {
        bounds,
        contents: Contents::Tile(tiles[0].vertices.clone()),
    });
    if tiles.len() > 1 {
        let axis = usize::from(bounds[4] - bounds[1] > bounds[3] - bounds[0]);
        let middle = tiles.len() / 2;
        tiles.select_nth_unstable_by(middle, |a, b| {
            (a.bounds[axis] + a.bounds[axis + 3])
                .total_cmp(&(b.bounds[axis] + b.bounds[axis + 3]))
                .then(a.vertices.start.cmp(&b.vertices.start))
        });
        let (left, right) = tiles.split_at_mut(middle);
        nodes[index].contents = Contents::Branch(build(left, nodes), build(right, nodes));
    }
    index
}

/// WebGPU clip inequalities: -w <= x,y <= w and 0 <= z <= w.
/// Work directly with the submitted matrix; no camera reconstruction or plane
/// normalization. The infinite projection's degenerate far plane is valid.
struct ClipPlane {
    coefficients: [f64; 4],
    rounding: [f64; 4],
}

fn clip_planes(matrix: &[f32; 16]) -> [ClipPlane; 6] {
    let row = |r: usize| -> [f64; 4] { std::array::from_fn(|c| f64::from(matrix[c * 4 + r])) };
    let w = row(3);
    [
        (0, 1., true),
        (0, -1., true),
        (1, 1., true),
        (1, -1., true),
        (2, 1., false),
        (2, -1., true),
    ]
    .map(|(r, sign, include_w)| {
        let v = row(r);
        ClipPlane {
            coefficients: std::array::from_fn(|i| sign * v[i] + if include_w { w[i] } else { 0. }),
            // GPU matrix products round in f32 before the clip comparison.
            // Bound their multiply/add error before combining rows, including
            // cancellation between a clip coordinate and w at an exact edge.
            rounding: std::array::from_fn(|i| {
                8. * f64::from(f32::EPSILON)
                    * (v[i].abs() + if include_w { w[i].abs() } else { 0. })
            }),
        }
    })
}

fn intersects(bounds: &Bounds, planes: &[ClipPlane; 6]) -> bool {
    planes.iter().all(|plane| {
        let p = plane.coefficients;
        let mut distance = p[3];
        let mut rounding = plane.rounding[3];
        for axis in 0..3 {
            distance += p[axis] * bounds[axis + if p[axis] >= 0. { 3 } else { 0 }];
            rounding += plane.rounding[axis] * bounds[axis].abs().max(bounds[axis + 3].abs());
        }
        distance >= -rounding
    })
}
