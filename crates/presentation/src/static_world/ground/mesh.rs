//! The ground's meshes, in the renderer's one vertex format: ten floats a
//! vertex (position, normal, rgba), flat-shaded triangle lists. The vertex
//! colour is a tint over the biome's material, its alpha the tint's weight
//! (0 draws the biome, 1 the tint alone). Computed in f64, so every
//! float matches the recorded vectors (`tests/ground_meshes.rs`).

/// Floats a vertex.
pub const VERTEX_FLOATS: usize = 10;

type V3 = [f64; 3];
type Rgba = [f64; 4];

/// No tint: the biome's material alone.
const BIOME: Rgba = [0.0, 0.0, 0.0, 0.0];
/// The traversal view's colours: ground units can or cannot enter.
const OPEN: Rgba = [0.52, 0.56, 0.5, 1.0];
const BLOCKED: Rgba = [0.78, 0.3, 0.26, 1.0];
/// The skirt's earth, and how far it reaches below the lowest ground (m).
const SKIRT: Rgba = [0.33, 0.3, 0.26, 1.0];
const SKIRT_DEPTH_M: f64 = 6.0;

/// The simulation's own ground triangles (its exported vertices, in its
/// exported index order), flat-shaded, never resampled. Untinted; with
/// `blocked` (the exported triangle surfaces, two bytes a triangle, and the
/// blocked flag), tinted by whether ground units can enter each: the
/// traversal view.
pub fn terrain(positions: &[f32], indices: &[u32], blocked: Option<(&[u8], u8)>) -> Vec<f32> {
    let mut out = Vec::with_capacity(indices.len() * VERTEX_FLOATS);
    let corner =
        |i: u32| -> V3 { std::array::from_fn(|k| f64::from(positions[i as usize * 3 + k])) };
    for (t, tri) in indices.as_chunks::<3>().0.iter().enumerate() {
        let tint = match blocked {
            Some((surfaces, flag)) if surfaces[t * 2 + 1] & flag != 0 => BLOCKED,
            Some(_) => OPEN,
            None => BIOME,
        };
        triangle(
            &mut out,
            [corner(tri[0]), corner(tri[1]), corner(tri[2])],
            tint,
        );
    }
    out
}

/// Vertical faces down from the map's boundary vertices, so the bounded map
/// reads as solid ground rather than a paper-thin sheet.
pub fn skirt(positions: &[f32]) -> Vec<f32> {
    let vertices: Vec<V3> = positions
        .as_chunks::<3>()
        .0
        .iter()
        .map(|p| [p[0], p[1], p[2]].map(f64::from))
        .collect();
    let (mut max_x, mut max_y, mut min_z) = (0.0f64, 0.0f64, f64::INFINITY);
    for v in &vertices {
        max_x = max_x.max(v[0]);
        max_y = max_y.max(v[1]);
        min_z = min_z.min(v[2]);
    }
    let bottom = min_z - SKIRT_DEPTH_M;
    let mut edges: [Vec<V3>; 4] = Default::default();
    for &v in &vertices {
        if v[1] == 0.0 {
            edges[0].push(v);
        }
        if v[0] == max_x {
            edges[1].push(v);
        }
        if v[1] == max_y {
            edges[2].push(v);
        }
        if v[0] == 0.0 {
            edges[3].push(v);
        }
    }
    // South west to east, east south to north, north east to west, west
    // north to south: each face outward.
    let by = |key: fn(&V3) -> f64| {
        move |a: &V3, b: &V3| {
            key(a)
                .partial_cmp(&key(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    };
    edges[0].sort_by(by(|v| v[0]));
    edges[1].sort_by(by(|v| v[1]));
    edges[2].sort_by(by(|v| -v[0]));
    edges[3].sort_by(by(|v| -v[1]));
    let mut out = vec![];
    for edge in &edges {
        for pair in edge.windows(2) {
            let [a, b] = [pair[0], pair[1]];
            quad(
                &mut out,
                [a, b, [b[0], b[1], bottom], [a[0], a[1], bottom]],
                SKIRT,
            );
        }
    }
    out
}

/// The water's surface: each cell (`Ground::water_cells`: its south-west
/// corner, surface height, side, look, and a puddle's centre and radius) a
/// flat square. The water takes no tint, so its vertex colour carries the
/// cell instead: red is its row of `TerrainParams.waterLooks`, then a
/// puddle's centre and radius (zero radius for water as deep as its ground).
pub fn water(cells: &[f32]) -> Vec<f32> {
    let (cells, rest) = cells.as_chunks::<{ super::water::CELL_FLOATS }>();
    assert!(
        rest.is_empty(),
        "water cells are {} floats each",
        super::water::CELL_FLOATS
    );
    let mut out = vec![];
    for c in cells {
        let [x, y, z, side, look, cx, cy, radius] = c.map(f64::from);
        quad(
            &mut out,
            [
                [x, y, z],
                [x + side, y, z],
                [x + side, y + side, z],
                [x, y + side, z],
            ],
            [look, cx, cy, radius],
        );
    }
    out
}

/// The land past the map's edge: a flat frame round the map's box
/// (`shadow::map_box`, which lies a metre below the lowest ground, so the
/// frame sits at that ground's height) out to `reach_m`, untinted.
pub fn backdrop(map_box: &[f64; 6], reach_m: f64) -> Vec<f32> {
    let [min_x, min_y, min_z, max_x, max_y, _] = *map_box;
    let z = min_z + 1.0;
    let r = reach_m;
    let mut out = vec![];
    for [x0, y0, x1, y1] in [
        [min_x - r, min_y - r, max_x + r, min_y], // south, full width
        [min_x - r, max_y, max_x + r, max_y + r], // north, full width
        [min_x - r, min_y, min_x, max_y],         // west
        [max_x, min_y, max_x + r, max_y],         // east
    ] {
        quad(
            &mut out,
            [[x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z]],
            BIOME,
        );
    }
    out
}

/// Two flat-shaded triangles, a b c and a c d.
fn quad(out: &mut Vec<f32>, [a, b, c, d]: [V3; 4], tint: Rgba) {
    triangle(out, [a, b, c], tint);
    triangle(out, [a, c, d], tint);
}

/// One flat-shaded triangle; its normal follows the winding.
fn triangle(out: &mut Vec<f32>, corners: [V3; 3], tint: Rgba) {
    let n = normal(corners);
    for p in corners {
        out.extend(p.iter().chain(&n).chain(&tint).map(|&v| v as f32));
    }
}

/// `triangle3.normal`: the unit normal, or zero for a degenerate triangle.
fn normal([a, b, c]: [V3; 3]) -> V3 {
    let (abx, aby, abz) = (b[0] - a[0], b[1] - a[1], b[2] - a[2]);
    let (acx, acy, acz) = (c[0] - a[0], c[1] - a[1], c[2] - a[2]);
    let n = [
        aby * acz - abz * acy,
        abz * acx - abx * acz,
        abx * acy - aby * acx,
    ];
    let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if length > 0.0 {
        n.map(|v| v / length)
    } else {
        n
    }
}
