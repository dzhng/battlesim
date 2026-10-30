//! The ground's GPU tables as the words both renderers upload: the height
//! pages, the plot split's nodes and plots, the terrain material's parameters
//! and the scars' look. Reflected records own the fixed WGSL layouts.
use super::biome::{linear, Road, AREA_KINDS, WATER_LOOKS_MAX};
use super::field::{Field, FOOTPRINT_M};
use super::plots::NODE;
use super::{is_road, Ground, RIVER};
use crate::records::{
    PlotNode, PlotRecord, RoadLook, ScarLook, ShoreTuning, TerrainParams, WaterLook,
};
use std::f64::consts::SQRT_2;

/// The most levels the material's surface field may build: the widest is
/// far past any map.
const LEVELS: usize = 32;
/// Grass thins past a bare margin over this distance, metres.
pub const GRASS_EDGE_M: f64 = 1.2;
/// The forest floor's soft edge as a share of its verge.
pub const FOREST_VERGE_FEATHER: f64 = 0.1;
pub const BANK_EASE_M: f64 = 0.75;
pub const MARKS_CLEAR_M: f64 = 0.3;
/// The deepest sampled scar, before biome relief is applied.
pub const SCAR_DEPTH_CAP: f64 = 1.6;
use crate::color::luminance;
/// The terrain material's paved look slots, in the map's kind order.
pub const ROAD_LOOKS_MAX: usize = AREA_KINDS.len();

/// `v` as the f32 a `Float32Array` holds, in its word.
pub fn word(v: f64) -> u32 {
    (v as f32).to_bits()
}

/// The f32 values a `Float32Array` holds, rounded after f64 arithmetic.
pub(super) fn f4(v: [f64; 4]) -> [f32; 4] {
    v.map(|v| v as f32)
}

fn rgb(c: [f64; 3], w: f64) -> [f64; 4] {
    [c[0], c[1], c[2], w]
}

fn tan_deg(deg: f64) -> f64 {
    libm::tan(deg * std::f64::consts::PI / 180.0)
}

/// Material tables derived from a ground surface field.
pub struct TerrainTables {
    pub nodes: Vec<u32>,
    pub params: TerrainParams,
    pub scars: ScarLook,
    pub surfaces: Vec<f32>,
    pub surface_index: Vec<u32>,
}

impl Ground {
    /// How far from its edge each rule (paved, forest, water) is read by a
    /// pixel `footprint` metres wide: what the material's surface field lists.
    /// Each term is one reader: beside a road its shoulder or walk, its edge's
    /// feather, a crossing's bars and the dash short of them; a forest's
    /// ragged verge, feathered by a pixel, and the grass's margin past it; the
    /// shore's bands, the water's pixel-wide edge, the grass's margin and the
    /// bank's shading out to `bank_m` from the water.
    pub fn reach(&self, footprint: f64, bank_m: f64) -> [f64; 3] {
        let b = &self.biome;
        let floor = &b.forest_floor;
        let grass = b.grass.clear_m.area + GRASS_EDGE_M;
        let most =
            |f: &dyn Fn(&Road) -> f64| b.roads.values().map(f).fold(f64::NEG_INFINITY, f64::max);
        let marks = most(&|r| {
            r.markings.as_ref().map_or(0.0, |m| {
                m.crossing.gap_m + m.crossing.length_m + MARKS_CLEAR_M + m.dash_m[0]
            })
        });
        let walk = most(&|r| r.walk.as_ref().map_or(0.0, |w| w.width_m));
        let paved = most(&|r| r.shoulder.width_m)
            .max(marks.max(walk + most(&|r| r.feather_m).max(footprint) / 2.0));
        let forest_m = floor.verge_m * FOREST_VERGE_FEATHER;
        let forest = floor.verge_m + floor.verge_warp_m + footprint.max(forest_m).max(grass);
        let bank = bank_m + 2.0 * footprint.max(BANK_EASE_M);
        let water = b.shore.mud_m.max(footprint / 2.0).max(bank.max(grass));
        [paved, forest, water]
    }

    /// The material's surface field: the strokes as drawn, each rule read
    /// out to its reach at every level.
    pub fn terrain_field(&self) -> Result<Field, String> {
        let s = &self.source;
        let longest = chunks_of(&s.rivers).fold(0.0f64, |run, r| {
            run.max((r[8] / r[6]) as f64).max((r[9] / r[7]) as f64)
        });
        let bank_m = longest + s.grid.spacing * SQRT_2;
        let reach: Vec<[f64; 3]> = (0..LEVELS)
            .map(|level| self.reach(FOOTPRINT_M * libm::pow(2.0, level as f64), bank_m))
            .collect();
        self.field(true, &reach)
    }

    /// The plots' records; without `textured`, their rows and dry patches
    /// alone (a paired cost measure).
    pub fn plot_records(&self, textured: bool) -> Vec<PlotRecord> {
        let mut records = Vec::with_capacity(self.plots.plots.len().max(1));
        for p in &self.plots.plots {
            let k = &self.biome.plots[p.kind];
            let on = |v: f64| if textured { v } else { 0.0 };
            records.push(PlotRecord {
                colour: f4(rgb(linear(p.colour), k.roughness)),
                rows: f4([p.across[0], p.across[1], k.furrow_m, k.furrow_contrast]),
                detail: f4([k.mottle, p.kind as f64, on(k.row_break), 0.0]),
                grain: f4([1.0 / k.grain_m, on(k.grain), 1.0 / k.grain_stretch, 0.0]),
                dry: f4([1.0 / k.patch_m[0], 1.0 / k.patch_m[1], 0.0, 0.0]),
                tram: f4([on(k.tram.rows), k.tram.width_m / 2.0, k.tram.contrast, 0.0]),
            });
        }
        // A storage binding cannot be empty.
        records.resize(records.len().max(1), PlotRecord::default());
        records
    }

    /// How far a crater's bowl moves the scars read along a view ray
    /// (`groundScarsSeen`), metres: as deep as the shader reads, at the
    /// relief its uniform holds, both ways. A region's scars load this far past it.
    pub fn scar_reach_m(&self) -> f64 {
        let relief = self.biome.scars.crater.relief_m as f32 as f64;
        2.0 * SCAR_DEPTH_CAP * relief
    }

    /// The bank's bands and ground shading.
    fn shore_tuning(&self) -> ShoreTuning {
        let t = &self.tuning.terrain.shore;
        let [a, b] = t.turns;
        ShoreTuning {
            bands: f4([t.wet_whole, t.earth_whole, t.fine, t.fine_share]),
            turns: f4([libm::cos(a), libm::sin(a), libm::cos(b), libm::sin(b)]),
            ground: f4([
                t.earth_mottle,
                t.field_ease_m,
                t.earth_roughness,
                t.wet_roughness,
            ]),
            fade: f4([t.fine_fade[0], t.fine_fade[1], t.bed_ease_m, 0.0]),
        }
    }

    /// Every biome water look over the shared surface tuning, with unused slots zeroed.
    fn water_looks(&self) -> [WaterLook; WATER_LOOKS_MAX] {
        let (b, w) = (&self.biome, &self.tuning.terrain.water);
        let (r, lobes) = (&w.normals, &w.lobes);
        // A puddle fades its water in over the first third of its depth.
        let puddles = b
            .water
            .puddles
            .iter()
            .flat_map(|q| q.looks.iter().map(|l| (l, q.depth_m)));
        let deep = b
            .water
            .rivers
            .iter()
            .chain(&b.water.swamps)
            .map(|l| (l, 0.0));
        let mut looks = [WaterLook::default(); WATER_LOOKS_MAX];
        for (tag, (look, puddle_m)) in deep.chain(puddles).enumerate() {
            let colours = b.palette(&look.palette);
            let [primary, macro_m, detail] = r.tiles_m;
            let shore = if puddle_m > 0.0 {
                puddle_m / 3.0
            } else {
                w.shore_fade_m
            };
            let (f, c) = (&w.refraction, &w.caustics);
            looks[tag] = WaterLook {
                channel: f4(rgb(linear(colours[0]), 0.0)),
                edge: f4(rgb(linear(colours[1]), 0.0)),
                surface: f4(rgb(look.extinction_per_m, w.shadow)),
                normals: f4([1.0 / primary, 1.0 / macro_m, 1.0 / detail, r.strength]),
                drifts: f4([r.drifts[0], r.drifts[1], r.drifts[2], 0.0]),
                fades: f4([shore, r.fade_m[0], r.fade_m[1], puddle_m]),
                roughness: f4(rgb(lobes.roughness, 0.0)),
                lobes: f4(rgb(lobes.weights, 0.0)),
                refraction: f4([f.share, f.depth_m, f.most, f.blur_m]),
                caustics: f4([1.0 / c.tile_m, c.strength, c.fade_per_m, c.sharpness]),
            };
        }
        looks
    }

    /// Terrain material inputs and scar appearance from `field` (`terrain_field`).
    pub fn terrain_tables(&self, field: Field) -> TerrainTables {
        let (s, b) = (&self.source, &self.biome);
        let mut nodes: Vec<PlotNode> = self
            .plots
            .nodes
            .as_chunks::<NODE>()
            .0
            .iter()
            .map(|n| PlotNode {
                line: f4([n[0], n[1], n[2], 0.0]),
                children: [n[3] as i32, n[4] as i32, 0, 0],
            })
            .collect();
        // A storage binding cannot be empty.
        nodes.resize(nodes.len().max(1), PlotNode::default());
        let one = |key: &str| linear(b.palette(key)[0]);
        let rules = &b.field_rules;
        let [ox, oy, cell, cols, rows, levels, _] = field.grid;
        let layer = |tag: usize| {
            b.road(AREA_KINDS[tag])
                .layer
                .unwrap_or((AREA_KINDS.len() - tag) as f64)
        };
        let mut order: [usize; ROAD_LOOKS_MAX] = std::array::from_fn(|tag| tag);
        order.sort_by(|&x, &y| layer(x).total_cmp(&layer(y)).then(y.cmp(&x)));
        let f = &b.forest_floor;
        let [litter, moss, humus] = [0, 1, 2].map(|i| linear(b.palette(&f.palette)[i]));
        let (shore, bank) = (&b.shore, b.palette(&b.shore.palette));
        let pixels = self.tuning.terrain.distant.plot_pixels;
        let params = TerrainParams {
            region: f4(self.plots.region),
            field: f4([ox, oy, 1.0 / cell, FOOTPRINT_M]),
            field_grid: [cols as u32, rows as u32, levels as u32, 0],
            shape: f4([
                rules.edge_warp_m,
                1.0 / rules.edge_warp_scale_m,
                1.0 / rules.mottle_m,
                0.0,
            ]),
            verge: f4(rgb(one(&b.verge.palette), b.verge.width_m / 2.0)),
            feathers: f4([
                b.verge.feather_m,
                rules.size_m[0] * pixels[0],
                rules.size_m[0] * pixels[1],
                0.0,
            ]),
            roads: std::array::from_fn(|tag| self.road_look(b.road(AREA_KINDS[tag]), tag)),
            road_order: order.map(|tag| tag as u32),
            forest_litter: f4(rgb(litter, 0.0)),
            forest_moss: f4(rgb(moss, f.moss)),
            forest_humus: f4(rgb(humus, f.humus)),
            forest_detail: f4([1.0 / f.patch_m, f.mottle, f.roughness, f.roots]),
            forest_verge: f4([
                f.verge_m,
                f.verge_warp_m,
                1.0 / f.verge_warp_scale_m,
                1.0 / f.roots_m,
            ]),
            forest_dapple: f4([1.0 / f.dapple.size_m, f.dapple.share, f.dapple.sun, 0.0]),
            forest_line: f4([f.tree_line.taper_m, f.tree_line.warp_m, 0.0, 0.0]),
            water_bed: f4(rgb(one("water_bed"), s.grid.spacing * SQRT_2)),
            shore: f4(rgb(linear(bank[0]), shore.wet_m)),
            shore_earth: f4(rgb(linear(bank[1]), shore.mud_m)),
            shore_edge: f4([
                shore.wander,
                1.0 / shore.wander_scale_m,
                shore.lift,
                shore.relief,
            ]),
            distant: f4(rgb(one("distant"), 0.0)),
            shore_tuning: self.shore_tuning(),
            water_looks: self.water_looks(),
            view: [0; 4],
        };
        let sc = &b.scars;
        let mark = |palette: &str, strength: f64| f4(rgb(one(palette), strength));
        let scars = ScarLook {
            full: f4(
                [&sc.crater.mark, &sc.scorch, &sc.tracks, &sc.trampled].map(|m| 255.0 / m.full)
            ),
            crater: mark(&sc.crater.mark.palette, sc.crater.mark.strength),
            ejecta: mark(&sc.crater.ejecta_palette, sc.crater.ejecta),
            scorch: mark(&sc.scorch.palette, sc.scorch.strength),
            tracks: mark(&sc.tracks.palette, sc.tracks.strength),
            trampled: mark(&sc.trampled.palette, sc.trampled.strength),
            relief: f4([sc.crater.relief_m, sc.crater.rim, 0.0, 0.0]),
            grass: f4([sc.grass.thin, sc.grass.tracks_thin, sc.grass.flatten, 0.0]),
        };
        TerrainTables {
            nodes: bytemuck::cast_slice(&nodes).to_vec(),
            params,
            scars,
            surfaces: field.records,
            surface_index: field.index,
        }
    }

    /// The look table's row for the paved kind tagged `tag`, drawn as
    /// `road`. Its patches' colour is held to the surface's luminance: a
    /// patch is a change of hue alone.
    fn road_look(&self, road: &Road, tag: usize) -> RoadLook {
        let palettes = &self.biome.palettes;
        let first = |key: &str| linear(palettes[key][0]);
        let (core, worn) = (first(&road.palette), linear(palettes[&road.palette][1]));
        let level = luminance(core) / luminance(worn);
        let walk_row = road.walk.as_ref().map_or(tag, |w| {
            AREA_KINDS.iter().position(|k| *k == w.kind).unwrap()
        });
        let walk = |f: fn(&super::biome::Walk) -> f64| road.walk.as_ref().map_or(0.0, f);
        let (s, r) = (&road.shoulder, &road.ruts);
        let rut = |i: usize| r.offsets_m.get(i).copied().unwrap_or(0.0);
        let strip = &road.centre_strip;
        let carriageway = if is_road(tag as f32) { 1.0 } else { 0.0 };
        let curb = road.curb.as_ref();
        let slab = walk(|w| if w.slab_m != 0.0 { 1.0 / w.slab_m } else { 0.0 });
        let marks = road.markings.as_ref();
        let slabs = road.slabs.as_ref();
        RoadLook {
            core: f4(rgb(core, road.roughness)),
            worn: f4([
                worn[0] * level,
                worn[1] * level,
                worn[2] * level,
                road.mottle,
            ]),
            shape: f4([
                road.feather_m,
                1.0 / road.patch_m,
                road.grain,
                1.0 / road.grain_m,
            ]),
            join: f4([road.join_m, 0.0, walk(|w| w.width_m), walk_row as f64]),
            shoulder: f4(rgb(first(&s.palette), s.width_m)),
            edge: f4([s.jitter, 1.0 / s.jitter_m, s.grass, s.cover]),
            ruts: f4([rut(0), rut(1), r.width_m, tan_deg(r.tilt_deg)]),
            track: f4([
                r.tint,
                strip.half_width_m,
                strip.max_road_width_m / 2.0,
                carriageway,
            ]),
            slabs: f4([
                slab,
                walk(|w| w.joint),
                curb.map_or(0.0, |c| c.face_m),
                curb.map_or(0.0, |c| tan_deg(c.tilt_deg)),
            ]),
            curb: f4(curb.map_or([0.0; 4], |c| rgb(first(&c.palette), c.width_m))),
            paint: f4(marks.map_or([0.0; 4], |m| rgb(first(&m.palette), m.cover))),
            marks: f4(marks.map_or([0.0, 0.0, 1.0, 0.0], |m| {
                [
                    m.line_m / 2.0,
                    m.dash_m[0],
                    m.dash_m[0] + m.dash_m[1],
                    m.wear,
                ]
            })),
            crossing: f4(marks.map_or([1.0, 0.0, 0.0, 0.0], |m| {
                let c = &m.crossing;
                [c.bar_m, c.length_m, c.gap_m, c.inset_m]
            })),
            stones: f4([
                curb.map_or(0.0, |c| 1.0 / c.stone_m),
                curb.map_or(0.0, |c| c.joint),
                slabs.map_or(0.0, |s| 1.0 / s.slab_m),
                slabs.map_or(0.0, |s| s.joint),
            ]),
        }
    }
}

fn chunks_of(rivers: &[f32]) -> impl Iterator<Item = &[f32; RIVER]> {
    rivers.as_chunks::<RIVER>().0.iter()
}

/// The public sampled ground: absent vertex pages are flat at zero.
#[derive(Default)]
pub struct Grid {
    pub nx: usize,
    pub ny: usize,
    pub spacing: f64,
    pub page_size: usize,
    pub min_height: f64,
    /// Sorted ids in the `ceil(nx / page_size)`-wide page directory.
    pub page_ids: Vec<u32>,
    /// `page_size²` row-major heights per present page.
    pub heights: Vec<f32>,
}

impl Grid {
    /// The pages as the GPU reads them: page size, columns, directory
    /// length, the directory (present page + 1, or 0), then the heights.
    pub fn pages(&self) -> Vec<u32> {
        let cols = self.nx.div_ceil(self.page_size);
        let count = if self.page_ids.is_empty() {
            0
        } else {
            cols * self.ny.div_ceil(self.page_size)
        };
        let mut out = vec![self.page_size as u32, cols as u32, count as u32];
        out.resize(3 + count, 0);
        for (p, &id) in self.page_ids.iter().enumerate() {
            out[3 + id as usize] = p as u32 + 1;
        }
        out.extend(self.heights.iter().map(|h| h.to_bits()));
        out
    }

    /// The height of vertex `(i, j)`.
    pub fn sample(&self, i: usize, j: usize) -> f64 {
        let size = self.page_size;
        let id = ((j / size) * self.nx.div_ceil(size) + i / size) as u32;
        match self.page_ids.binary_search(&id) {
            Ok(page) => self.heights[page * size * size + (j % size) * size + i % size] as f64,
            Err(_) => 0.0,
        }
    }

    /// The ground's height at `(x, y)`, clamped to the grid's edge, by the
    /// simulation's south-west to north-east triangle rule.
    pub fn height(&self, x: f64, y: f64) -> f64 {
        let (nx, ny) = ((self.nx - 1) as f64, (self.ny - 1) as f64);
        let fx = (x / self.spacing).max(0.0).min(nx);
        let fy = (y / self.spacing).max(0.0).min(ny);
        let i = fx.floor().min(nx - 1.0) as usize;
        let j = fy.floor().min(ny - 1.0) as usize;
        let (u, v) = (fx - i as f64, fy - j as f64);
        let (h00, h11) = (self.sample(i, j), self.sample(i + 1, j + 1));
        if u >= v {
            let h10 = self.sample(i + 1, j);
            h00 + u * (h10 - h00) + v * (h11 - h10)
        } else {
            let h01 = self.sample(i, j + 1);
            h00 + v * (h01 - h00) + u * (h11 - h01)
        }
    }
}
