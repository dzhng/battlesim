//! Spatial draw ranges over immutable ground triangles. Whole triangles retain
//! their exact vertex bytes; tile bounds include corners crossing tile edges.
//! Both clients select with the camera matrix they actually submit to the GPU.
use std::collections::BTreeMap;
use std::ops::Range;

use super::mesh::VERTEX_FLOATS;
use crate::clip::{clip_planes, intersects};
use crate::static_world::index::BoundsIndex;

/// Shared spatial batching size; never a visibility cutoff.
pub const CELL_M: f64 = 256.;

const TRIANGLE_FLOATS: usize = 3 * VERTEX_FLOATS;
type Bounds = [f64; 6];

struct Tile {
    bounds: Bounds,
    vertices: Range<u32>,
}

/// One upload and a spatial index. Building is map-wide; selection visits only
/// intersecting hierarchy branches. An infinite-far view remains infinite:
/// looking along the horizon can legitimately select the whole map.
pub struct TerrainPartition {
    vertices: Vec<f32>,
    tiles: Vec<Range<u32>>,
    index: BoundsIndex,
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
        let index = BoundsIndex::new(tiles.iter().map(|t| t.bounds));
        Self {
            vertices,
            tiles: tiles.into_iter().map(|t| t.vertices).collect(),
            index,
        }
    }

    pub fn vertices(&self) -> &[f32] {
        &self.vertices
    }

    /// Replace `ranges` with visible vertex ranges in buffer order. Returns
    /// bounds tested, so callers can distinguish spatial selection from a
    /// complete-map scan. Retain the caller-owned vector across frames.
    pub fn select(&self, view_projection: &[f32; 16], ranges: &mut Vec<Range<u32>>) -> usize {
        ranges.clear();
        let planes = clip_planes(view_projection);
        let visited = self.index.query(
            |bounds| intersects(bounds, &planes),
            |id| ranges.push(self.tiles[id].clone()),
        );
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
}
