//! The grass field's tables: the biome's growth resolved against the grass
//! appearances, the pass's parameters (`GrassParams` in
//! `frame/grassPass.ts`, word for word) and the props' footprints where no
//! grass grows. Where clumps stand, how many and how tall is decided per
//! frame on the GPU; nothing here is per clump. The renderer packs each
//! kind's blade meshes itself and reports their facts (`KindFacts`).
use super::biome::{GRASS_MIX_MAX, VERGE_GROWTH};
use super::tables::{vec4, word};
use super::{js, Ground, PROP};

/// Plot kinds the growth table holds; the last row is the verge.
const GROWTH_ROWS: usize = 16;
/// Grass appearances one field draws.
const MAX_KINDS: usize = 16;
/// The props table's smallest cell and most cells a side; a prop is listed
/// this far past its bounds, so a point the GPU's f32 puts in the next cell
/// still finds it.
const PROP_CELL_M: f64 = 8.0;
const PROP_GRID_MAX: f64 = 512.0;
const PROP_PAD_M: f64 = 0.05;

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
pub struct Limits {
    pub variation: f64,
    pub tallest: f64,
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

    /// `GrassParams` with its per-frame fields (`planes`, `eye`, `window`'s
    /// corner and tile, `grid`'s tiles) zero, for `facts` in `grass_kinds`
    /// order; refused where a row would grow grass past `limits.tallest`.
    pub fn grass_params(&self, facts: &[KindFacts], limits: &Limits) -> Result<Vec<u32>, String> {
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
        let (mut tallest, mut row_reach) = (0.0, 0.0);
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
            row_reach = js::max(row_reach, furrow / 2.0 * g.rows);
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
                tallest = js::max(tallest, most);
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
        let (w, s) = (&rules.wind, &rules.patch_m);
        let heading = w.heading_deg * std::f64::consts::PI / 180.0;
        let mut out = vec![0; 5 * 4];
        vec4(&mut out, [0.0, 0.0, 0.0, tallest]);
        out.extend([0, 0, rules.capacity[0], rules.capacity[1]]);
        vec4(
            &mut out,
            [
                rules.pixels_per_clump,
                rules.max_clumps_m2,
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
            [js::cos(heading), js::sin(heading), w.lean, w.gust],
        );
        vec4(
            &mut out,
            [
                1.0 / w.gust_m,
                w.gust_mps,
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
                let (c, s) = (js::cos(yaw).abs(), js::sin(yaw).abs());
                let ex = (hx + margin) * c + (hy + margin) * s + PROP_PAD_M;
                let ey = (hx + margin) * s + (hy + margin) * c + PROP_PAD_M;
                (min_x, min_y) = (js::min(min_x, x - ex), js::min(min_y, y - ey));
                (max_x, max_y) = (js::max(max_x, x + ex), js::max(max_y, y + ey));
                [x - ex, y - ey, x + ex, y + ey]
            })
            .collect();
        let any = !props.is_empty();
        let cell = if any {
            js::max(
                PROP_CELL_M,
                js::max(max_x - min_x, max_y - min_y) / PROP_GRID_MAX,
            )
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
                js::cos(yaw),
                js::sin(yaw),
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
