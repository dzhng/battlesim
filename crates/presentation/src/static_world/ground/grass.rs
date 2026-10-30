//! The grass field on the CPU: the biome's growth resolved against the grass
//! appearances, the grass kinds' blade meshes packed for the GPU
//! ([`pack_shapes`]), the pass's parameters (`GrassParams`, word for word as
//! the grass shaders declare it) with each frame's fields ([`frame_params`]),
//! the window of world tiles a frame grows grass over ([`window`]) and the
//! props' footprints where no grass grows. Where clumps stand, how many and
//! how tall is decided per frame on the GPU; nothing here is per clump.
use super::biome::{GrassRules, GRASS_MIX_MAX, VERGE_GROWTH};
use super::tables::{vec4, word};
use super::{Ground, PROP};

/// Plot kinds the growth table holds; the last row is the verge.
pub const GROWTH_ROWS: usize = 16;
/// Grass appearances one field draws.
pub const MAX_KINDS: usize = 16;
/// The props table's smallest cell and most cells a side; a prop is listed
/// this far past its bounds, so a point the GPU's f32 puts in the next cell
/// still finds it.
const PROP_CELL_M: f64 = 8.0;
const PROP_GRID_MAX: f64 = 512.0;
const PROP_PAD_M: f64 = 0.05;

/// World tiles the field is grown over, metres a side.
pub const TILE_M: f64 = 4.0;
/// Tiles a side the window may hold (the build dispatch's bound).
pub const MAX_TILES: u32 = 256;
/// Segments a blade in each LOD tier of a grass kind, finest first (the
/// scene assets' `GRASS_SEGMENTS`, which the bake writes).
const KIND_SEGMENTS: [u32; 4] = [4, 3, 2, 1];
/// The field's two tiers, near then far: which of a kind's LOD tiers each
/// draws, and so their segments a blade.
const FIELD_LODS: [usize; 2] = [0, 2];
pub const FIELD_SEGMENTS: [u32; 2] = [KIND_SEGMENTS[FIELD_LODS[0]], KIND_SEGMENTS[FIELD_LODS[1]]];
/// Floats a clump vertex on the GPU: spine (xyz, height fraction), side
/// (xyz, phase), normal (xyz, 0), tint (rgb, how far it answers the wind).
pub const SHAPE_FLOATS: usize = 16;
/// Per-clump height variation shared by packing, authoring and the shader.
pub const HEIGHT_VARIATION: [f64; 2] = [0.8, 1.2];

/// Grass stays below half a soldier, so its look never promises physical cover.
pub const LIMITS: Limits = Limits {
    variation: HEIGHT_VARIATION[1],
    tallest: 0.9,
};

/// What the renderer's packing found of one grass kind's meshes: the
/// highest vertex of any tier, its first state's top, its blades and a
/// blade's root width, and where its near and far tiers start among the
/// shape vertices.
pub struct KindFacts {
    pub mesh_height: f64,
    pub top: f64,
    pub blades: f64,
    pub root_width: f64,
    pub bases: [u32; 2],
}

/// The scene assets' own limits on a clump: its height variation's top,
/// and the tallest any grass may be drawn (half a soldier).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub variation: f64,
    pub tallest: f64,
}

/// The player's graphics setting for the field: a scale on the most clumps a
/// square metre holds (1: the biome's own).
pub struct Quality {
    pub density: f64,
}

impl Ground {
    /// The grass appearances the biome's growth names, in kind order.
    pub fn grass_kinds(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .biome
            .grass
            .growth
            .values()
            .flat_map(|g| g.mix.iter().map(|s| s.appearance.clone()))
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// `GrassParams` under the player's `quality`, with its per-frame fields (`planes`, `eye`, `window`'s
    /// corner and tile, `grid`'s tiles) zero, for `facts` in `grass_kinds`
    /// order; refused where a row would grow grass past `limits.tallest`.
    pub fn grass_params(
        &self,
        facts: &[KindFacts],
        limits: &Limits,
        quality: &Quality,
    ) -> Result<Vec<u32>, String> {
        let (rules, names) = (&self.biome.grass, self.grass_kinds());
        if self.biome.plots.len() > GROWTH_ROWS - 1 {
            return Err(format!(
                "grass: at most {} plot kinds grow grass",
                GROWTH_ROWS - 1
            ));
        }
        if names.len() > MAX_KINDS || facts.len() != names.len() {
            return Err(format!(
                "grass: {} kinds, at most {MAX_KINDS}, with facts for each",
                names.len()
            ));
        }
        let mut growth = [[0.0; 4]; GROWTH_ROWS];
        let mut patches = [[0.0; 4]; GROWTH_ROWS];
        let mut mixes = [[0.0; 4]; GROWTH_ROWS * GRASS_MIX_MAX];
        let (mut tallest, mut row_reach) = (0.0f64, 0.0f64);
        let rows = self
            .biome
            .plots
            .iter()
            .enumerate()
            .map(|(k, p)| (k, p.name.as_str(), p.furrow_m));
        for (at, key, furrow) in rows.chain([(GROWTH_ROWS - 1, VERGE_GROWTH, 0.0)]) {
            let Some(g) = rules.growth.get(key) else {
                continue;
            };
            row_reach = row_reach.max(furrow / 2.0 * g.rows);
            let shares = g.mix.iter().fold(0.0, |sum, s| sum + s.share);
            for (i, s) in g.mix.iter().enumerate() {
                let kind = names.iter().position(|n| *n == s.appearance).unwrap();
                // The shader's f32 inputs and products.
                let f = |v: f64| v as f32;
                let most = f(facts[kind].mesh_height)
                    * f(g.height)
                    * f(limits.variation)
                    * f(g.patches.height[1]);
                let most = most as f64;
                if !(most > 0.0 && most <= limits.tallest) {
                    return Err(format!("grass.growth.{key}: {}'s effective field height {most} m exceeds the {} m cap", s.appearance, limits.tallest));
                }
                tallest = tallest.max(most);
                mixes[at * GRASS_MIX_MAX + i] = [kind as f64, s.share / shares, s.drift, s.dry];
            }
            growth[at] = [g.density, g.height, g.mix.len() as f64, g.rows];
            patches[at] = [
                g.patches.height[0],
                g.patches.height[1],
                g.patches.thin,
                g.patches.dry,
            ];
        }
        let grid = &self.source.grid;
        let (w, s) = (&rules.sway, &rules.patch_m);
        let mut out = vec![0; 5 * 4];
        vec4(&mut out, [0.0, 0.0, 0.0, tallest]);
        out.extend([0, 0, rules.capacity[0], rules.capacity[1]]);
        vec4(
            &mut out,
            [
                rules.pixels_per_clump,
                rules.max_clumps_m2 * quality.density,
                rules.fade_m_per_px[0],
                rules.fade_m_per_px[1],
            ],
        );
        vec4(
            &mut out,
            [
                rules.near_tier_px,
                rules.min_blade_px,
                rules.soften_m_per_px[0],
                rules.soften_m_per_px[1],
            ],
        );
        vec4(
            &mut out,
            [rules.soften, rules.blade_facing, rules.clump_value, 0.0],
        );
        vec4(&mut out, [rules.clear_m.area, row_reach, 0.0, 0.0]);
        out.extend([grid.nx as u32, grid.ny as u32, 0, 0]);
        vec4(&mut out, [grid.spacing, 0.0, 0.0, 0.0]);
        vec4(
            &mut out,
            [
                w.lean,
                w.gust,
                w.flutter,
                w.flutter_hz * 2.0 * std::f64::consts::PI,
            ],
        );
        for v in growth.iter().chain(&patches).chain(&mixes) {
            vec4(&mut out, *v);
        }
        vec4(
            &mut out,
            [1.0 / s.height, 1.0 / s.thin, 1.0 / s.dry, 1.0 / s.drift],
        );
        vec4(&mut out, [1.0 / s.grain, rules.dry_lift, 0.0, 0.0]);
        for k in 0..MAX_KINDS {
            vec4(
                &mut out,
                facts
                    .get(k)
                    .map_or([0.0; 4], |f| [f.top, f.blades, f.root_width, 0.0]),
            );
        }
        for k in 0..MAX_KINDS {
            out.extend(
                facts
                    .get(k)
                    .map_or([0; 4], |f| [f.bases[0], f.bases[1], 0, 0]),
            );
        }
        Ok(out)
    }

    /// The window of tiles a camera at `eye` grows grass over, by the
    /// rules the field grows by and down to the map's lowest ground.
    pub fn grass_window(&self, eye: [f64; 3], metres_per_px: f64) -> Window {
        window(
            eye,
            self.source.grid.min_height,
            metres_per_px,
            &self.biome.grass,
        )
    }

    /// Where no grass grows: every static prop's footprint widened by the
    /// biome's margin, bucketed by a grid of cells so a clump asks only the
    /// props of its own cell. In `vec4` elements: the grid's origin and cell
    /// side (f32); cells across and up (u32); two cells an element, each
    /// (first record's element, record count); then each cell's records, two
    /// elements each: `x, y, cos yaw, sin yaw` and the half extents with the
    /// margin.
    pub fn grass_props(&self) -> Vec<u32> {
        let margin = self.biome.grass.clear_m.prop;
        let props = self.source.props.as_chunks::<PROP>().0;
        let footprint = |p: &[f32; PROP]| [p[3], p[4], p[5], p[6], p[7]].map(f64::from);
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        let bounds: Vec<[f64; 4]> = props
            .iter()
            .map(|p| {
                let [x, y, yaw, hx, hy] = footprint(p);
                let (c, s) = (libm::cos(yaw).abs(), libm::sin(yaw).abs());
                let ex = (hx + margin) * c + (hy + margin) * s + PROP_PAD_M;
                let ey = (hx + margin) * s + (hy + margin) * c + PROP_PAD_M;
                (min_x, min_y) = (min_x.min(x - ex), min_y.min(y - ey));
                (max_x, max_y) = (max_x.max(x + ex), max_y.max(y + ey));
                [x - ex, y - ey, x + ex, y + ey]
            })
            .collect();
        let any = !props.is_empty();
        let cell = if any {
            PROP_CELL_M.max((max_x - min_x).max(max_y - min_y) / PROP_GRID_MAX)
        } else {
            PROP_CELL_M
        };
        let (nx, ny) = if any {
            (
                ((max_x - min_x) / cell).floor() as usize + 1,
                ((max_y - min_y) / cell).floor() as usize + 1,
            )
        } else {
            (0, 0)
        };
        let span = |b: &[f64; 4]| {
            let at = |v: f64, lo: f64| ((v - lo) / cell).floor() as usize;
            (
                at(b[0], min_x),
                at(b[1], min_y),
                at(b[2], min_x),
                at(b[3], min_y),
            )
        };
        let mut counts = vec![0u32; nx * ny];
        for b in &bounds {
            let (i0, j0, i1, j1) = span(b);
            for j in j0..=j1 {
                for k in i0..=i1 {
                    counts[j * nx + k] += 1;
                }
            }
        }
        let records: usize = counts.iter().map(|&c| c as usize).sum();
        let base = 2 + (nx * ny).div_ceil(2);
        let mut out = vec![0; (base + records * 2) * 4];
        let origin = if any { [min_x, min_y] } else { [0.0; 2] };
        out[..4].copy_from_slice(&[word(origin[0]), word(origin[1]), word(cell), 0]);
        out[4..6].copy_from_slice(&[nx as u32, ny as u32]);
        let mut next = vec![0u32; nx * ny];
        let mut first = base as u32;
        for c in 0..nx * ny {
            out[8 + c * 2..10 + c * 2].copy_from_slice(&[first, counts[c]]);
            next[c] = first;
            first += counts[c] * 2;
        }
        for (p, b) in props.iter().zip(&bounds) {
            let [x, y, yaw, hx, hy] = footprint(p);
            let record = [
                x,
                y,
                libm::cos(yaw),
                libm::sin(yaw),
                hx + margin,
                hy + margin,
                0.0,
                0.0,
            ]
            .map(word);
            let (i0, j0, i1, j1) = span(b);
            for j in j0..=j1 {
                for k in i0..=i1 {
                    let at = next[j * nx + k] as usize * 4;
                    out[at..at + 8].copy_from_slice(&record);
                    next[j * nx + k] += 2;
                }
            }
        }
        out
    }
}

/// One LOD tier of a grass kind's installed mesh, as its bundle stores it:
/// xyz positions, snorm16 xyzw normals, uvs (u the blade's phase, v its
/// height fraction) and unorm8 rgba colours (a the wind's weight).
pub struct TierMesh<'a> {
    pub positions: &'a [f32],
    pub normals: &'a [i16],
    pub uvs: &'a [f32],
    pub colors: &'a [u8],
}

/// A grass kind's first state: its name (for a refusal), every LOD tier,
/// finest first, and the top of its bounds.
pub struct KindMesh<'a> {
    pub name: &'a str,
    pub tiers: Vec<TierMesh<'a>>,
    pub top: f64,
}

/// Every kind's two field tiers as clump vertices ([`SHAPE_FLOATS`] each),
/// each kind's facts for [`Ground::grass_params`], and the most blades any
/// kind has (the index lists' blades).
pub struct PackedShapes {
    pub shapes: Vec<f32>,
    pub facts: Vec<KindFacts>,
    pub max_blades: u32,
}

/// Vertices of one blade of `segments` segments: a left and a right at the
/// foot of each segment, then the tip.
pub fn blade_vertices(segments: u32) -> u32 {
    2 * segments + 1
}

/// The canonical index list of `blades` blade strips of `segments`
/// segments, every blade's vertices one after another.
pub fn strip_indices(blades: u32, segments: u32) -> Vec<u16> {
    let per = blade_vertices(segments);
    let mut out = Vec::with_capacity((blades * (2 * segments - 1) * 3) as usize);
    for b in 0..blades {
        let o = b * per;
        for k in 0..segments - 1 {
            let [l0, r0, l1, r1] = [0, 1, 2, 3].map(|i| o + 2 * k + i);
            out.extend([l0, r0, l1, r0, r1, l1]);
        }
        let last = o + 2 * (segments - 1);
        out.extend([last, last + 1, o + per - 1]);
    }
    out.into_iter().map(|i| i as u16).collect()
}

/// Packs `kinds` (in `Ground::grass_kinds` order) for the field: each field
/// tier's vertices as spine and side, each vertex's tint its colour against
/// the tier's tip-weighted mean colour (which the ground it grows on
/// replaces). Refused by name where a tier is not its kind's blade strips.
pub fn pack_shapes(kinds: &[KindMesh]) -> Result<PackedShapes, String> {
    let per = FIELD_SEGMENTS.map(|s| blade_vertices(s) as usize);
    let linear = |c: u8| libm::pow(f64::from(c) / 255.0, 2.2);
    let mut blades = Vec::with_capacity(kinds.len());
    for kind in kinds {
        let lod0 = kind.tiers.first().map_or(0, |t| t.positions.len() / 3);
        if lod0 == 0 || lod0 % per[0] != 0 {
            return Err(format!("grass: {} LOD0 is not blade strips", kind.name));
        }
        blades.push(lod0 / per[0]);
    }
    let total: usize = blades.iter().map(|b| b * (per[0] + per[1])).sum();
    let mut shapes = vec![0.0; total.max(1) * SHAPE_FLOATS];
    let mut facts = Vec::with_capacity(kinds.len());
    let mut at = 0;
    for (kind, &blades) in kinds.iter().zip(&blades) {
        let lod0 = &kind.tiers[0];
        let mut mesh_height = 0.0f64;
        for tier in &kind.tiers {
            for z in tier.positions.iter().skip(2).step_by(3) {
                mesh_height = mesh_height.max(f64::from(*z));
            }
        }
        let mut bases = [0; 2];
        for (t, &lod) in FIELD_LODS.iter().enumerate() {
            bases[t] = at as u32;
            let per = per[t];
            let mesh = kind
                .tiers
                .get(lod)
                .filter(|m| m.positions.len() == blades * per * 3)
                .ok_or_else(|| {
                    format!("grass: {} LOD{lod} is not {blades} blade strips", kind.name)
                })?;
            let p = |v: usize, c: usize| f64::from(mesh.positions[v * 3 + c]);
            // The camera mostly sees tips, so weighting the mean toward them
            // makes the near field average to the ground colour it stands on.
            let target: [f64; 3] = std::array::from_fn(|c| {
                let mut sum = 0.0;
                let mut weight_sum = 0.0;
                for v in 0..blades * per {
                    let h = f64::from(mesh.uvs[v * 2 + 1]);
                    let w = h * h + 0.15;
                    sum += w * linear(mesh.colors[v * 4 + c]);
                    weight_sum += w;
                }
                (sum / weight_sum.max(1e-4)).max(1e-4)
            });
            for v in 0..blades * per {
                let o = at * SHAPE_FLOATS;
                let in_blade = v % per;
                let tip = in_blade == per - 1;
                // A strip pair's spine is its midpoint; the tip is its own spine.
                let left = if tip { v } else { v - in_blade % 2 };
                let right = if tip { v } else { left + 1 };
                let h = f64::from(mesh.uvs[v * 2 + 1]);
                for c in 0..3 {
                    let spine = (p(left, c) + p(right, c)) / 2.0;
                    shapes[o + c] = spine as f32;
                    shapes[o + 4 + c] = if c == 2 {
                        0.0
                    } else {
                        (p(v, c) - spine) as f32
                    };
                    shapes[o + 8 + c] = (f64::from(mesh.normals[v * 4 + c]) / 32767.0) as f32;
                    let raw = linear(mesh.colors[v * 4 + c]) / target[c];
                    // Bright tips ease toward 1.3 rather than clip; the floor
                    // leaves dark gaps between stems but no black tips.
                    let floor = 0.3 + 0.5 * h;
                    let cap = 1.0 + 0.3 * (1.0 - libm::exp(-2.5 * (raw - 1.0).max(0.0)));
                    let tint = raw.max(floor).min(cap);
                    shapes[o + 12 + c] = tint as f32;
                }
                shapes[o + 3] = mesh.uvs[v * 2 + 1];
                shapes[o + 7] = mesh.uvs[v * 2];
                shapes[o + 15] = (f64::from(mesh.colors[v * 4 + 3]) / 255.0) as f32;
                at += 1;
            }
        }
        let p0 = |i: usize| f64::from(lod0.positions[i]);
        facts.push(KindFacts {
            mesh_height,
            top: kind.top,
            blades: blades as f64,
            root_width: libm::hypot(p0(0) - p0(3), p0(1) - p0(4)),
            bases,
        });
    }
    let max_blades = blades.iter().fold(1, |m, &b| m.max(b as u32));
    Ok(PackedShapes {
        shapes,
        facts,
        max_blades,
    })
}

/// The window of world tiles a frame grows grass over.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Window {
    /// The south-west corner of tile (0, 0), on the tile grid.
    pub x0: f64,
    pub y0: f64,
    pub tiles_x: u32,
    pub tiles_y: u32,
    /// The farthest a clump stands from the eye.
    pub reach: f64,
}

impl Window {
    /// The ground the window covers: min x, min y, max x, max y.
    pub fn bounds(&self) -> [f64; 4] {
        [
            self.x0,
            self.y0,
            self.x0 + f64::from(self.tiles_x) * TILE_M,
            self.y0 + f64::from(self.tiles_y) * TILE_M,
        ]
    }
}

/// Metres one pixel spans per metre of distance from the eye, for a lens of
/// `fov_y` on a viewport `height_px` tall.
pub fn metres_per_pixel(fov_y: f64, height_px: f64) -> f64 {
    2.0 * libm::tan(fov_y / 2.0) / height_px
}

/// The tiles round `eye` within the grass's reach: where one pixel spans
/// less than the fade's end, down to the `lowest` ground. None when the eye
/// is higher than that (the strategic height).
pub fn window(eye: [f64; 3], lowest: f64, metres_per_px: f64, rules: &GrassRules) -> Window {
    let reach = rules.fade_m_per_px[1] / metres_per_px;
    let drop = (eye[2] - lowest).max(0.0);
    if drop >= reach {
        return Window {
            reach,
            ..Window::default()
        };
    }
    let flat = (reach * reach - drop * drop).sqrt();
    let lo = |v: f64| ((v - flat) / TILE_M).floor() * TILE_M;
    let count =
        |v: f64, from: f64| f64::from(MAX_TILES).min(((v + flat - from) / TILE_M).ceil()) as u32;
    let (x0, y0) = (lo(eye[0]), lo(eye[1]));
    Window {
        x0,
        y0,
        tiles_x: count(eye[0], x0),
        tiles_y: count(eye[1], y0),
        reach,
    }
}

/// Writes a frame's fields into `GrassParams`' words (as
/// [`Ground::grass_params`] laid them): the view's four side planes from its
/// view-projection (inward normal, offset; near and far are left out, the
/// reach bounds the field), the eye and `metres_per_px`, the window's corner
/// and tile, and its tiles.
pub fn frame_params(
    words: &mut [u32],
    view_proj: &[f32; 16],
    eye: [f64; 3],
    metres_per_px: f64,
    window: &Window,
) {
    // `frustum.setFromViewProjectionMatrixSides` with an identity view:
    // the view-projection's rows 0, 1 and 3, as its products sum them.
    let p = view_proj.map(f64::from);
    let row = |r: usize| -> [f64; 4] {
        std::array::from_fn(|c| {
            let v = |k: usize| if k == c { 1.0 } else { 0.0 };
            p[r] * v(0) + p[4 + r] * v(1) + p[8 + r] * v(2) + p[12 + r] * v(3)
        })
    };
    let (r0, r1, r3) = (row(0), row(1), row(3));
    let plane = |b: [f64; 4], sign: f64| {
        let n: [f64; 4] = std::array::from_fn(|k| {
            if sign > 0.0 {
                r3[k] + b[k]
            } else {
                r3[k] - b[k]
            }
        });
        let mut inv = n[0] * n[0] + n[1] * n[1] + n[2] * n[2];
        if inv > 0.0 {
            inv = 1.0 / inv.sqrt();
        }
        n.map(|v| v * inv)
    };
    // Left, right, bottom, top.
    let planes = [
        plane(r0, 1.0),
        plane(r0, -1.0),
        plane(r1, 1.0),
        plane(r1, -1.0),
    ];
    for (i, pl) in planes.iter().enumerate() {
        for (k, &v) in pl.iter().enumerate() {
            words[i * 4 + k] = word(v);
        }
    }
    let frame = [
        eye[0],
        eye[1],
        eye[2],
        metres_per_px,
        window.x0,
        window.y0,
        TILE_M,
    ];
    for (k, v) in frame.into_iter().enumerate() {
        words[16 + k] = word(v);
    }
    words[24] = window.tiles_x;
    words[25] = window.tiles_y;
}
