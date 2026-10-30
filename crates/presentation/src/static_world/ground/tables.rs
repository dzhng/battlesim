//! The ground's GPU tables as the words both renderers upload: the height
//! pages, the plot split's nodes and plots, the terrain material's parameters
//! and the scars' look. Each record is the terrain material's WGSL struct
//! word for word: every field a `vec4`, f32 unless named.
use super::biome::{linear, Road, AREA_KINDS};
use super::field::{Field, FOOTPRINT_M};
use super::{is_road, Ground, RIVER};
use crate::records::{RoadLook, ScarParams, ShoreTuning, TerrainParams, WaterLook};
use std::f64::consts::SQRT_2;
use std::mem::offset_of;

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
/// Words a `WaterLook` row: ten vec4s.
const WATER_LOOK_WORDS: usize = WaterLook::LAYOUT.size / 4;

/// `v` as the f32 a `Float32Array` holds, in its word.
pub fn word(v: f64) -> u32 {
    (v as f32).to_bits()
}

/// Appends a `vec4f`.
pub fn vec4(out: &mut Vec<u32>, v: [f64; 4]) {
    out.extend(v.map(word));
}

/// Writes a `vec4f` at a record layout's byte offset.
pub fn write_vec4(out: &mut [u32], offset: usize, v: [f64; 4]) {
    out[offset / 4..offset / 4 + 4].copy_from_slice(&v.map(word));
}

fn rgb(c: [f64; 3], w: f64) -> [f64; 4] {
    [c[0], c[1], c[2], w]
}

fn tan_deg(deg: f64) -> f64 {
    libm::tan(deg * std::f64::consts::PI / 180.0)
}

/// The surface field's grid as the material's parameters carry it.
pub struct TerrainTables {
    pub heights: Vec<u32>,
    pub nodes: Vec<u32>,
    pub plots: Vec<u32>,
    /// `TerrainParams` but its last `vec4u` (`view`), the frame's own.
    pub params: Vec<u32>,
    /// `ScarParams` from `full` on: the look; the grid and cache words
    /// before it are the frame's.
    pub scars: Vec<u32>,
    pub surfaces: Vec<f32>,
    pub surface_index: Vec<u32>,
    pub water: Vec<f32>,
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
    pub fn plot_records(&self, textured: bool) -> Vec<u32> {
        let mut out = vec![];
        for p in &self.plots.plots {
            let k = &self.biome.plots[p.kind];
            let on = |v: f64| if textured { v } else { 0.0 };
            vec4(&mut out, rgb(linear(p.colour), k.roughness));
            vec4(
                &mut out,
                [p.across[0], p.across[1], k.furrow_m, k.furrow_contrast],
            );
            vec4(&mut out, [k.mottle, p.kind as f64, on(k.row_break), 0.0]);
            vec4(
                &mut out,
                [1.0 / k.grain_m, on(k.grain), 1.0 / k.grain_stretch, 0.0],
            );
            vec4(&mut out, [1.0 / k.patch_m[0], 1.0 / k.patch_m[1], 0.0, 0.0]);
            vec4(
                &mut out,
                [on(k.tram.rows), k.tram.width_m / 2.0, k.tram.contrast, 0.0],
            );
        }
        at_least_one(out, 24)
    }

    /// How far a crater's bowl moves the scars read along a view ray
    /// (`groundScarsSeen`), metres: as deep as the shader reads, at the
    /// relief its uniform holds, both ways. A region's scars load this far past it.
    pub fn scar_reach_m(&self) -> f64 {
        let relief = self.biome.scars.crater.relief_m as f32 as f64;
        2.0 * SCAR_DEPTH_CAP * relief
    }

    /// Every terrain table but the scars' grid, from `field` (`terrain_field`).
    /// `TerrainParams.shoreTuning`: the bank's bands and ground, four vec4s.
    fn shore_tuning(&self, p: &mut [u32]) {
        let t = &self.tuning.terrain.shore;
        let [a, b] = t.turns;
        write_vec4(
            p,
            offset_of!(ShoreTuning, bands),
            [t.wet_whole, t.earth_whole, t.fine, t.fine_share],
        );
        write_vec4(
            p,
            offset_of!(ShoreTuning, turns),
            [libm::cos(a), libm::sin(a), libm::cos(b), libm::sin(b)],
        );
        write_vec4(
            p,
            offset_of!(ShoreTuning, ground),
            [
                t.earth_mottle,
                t.field_ease_m,
                t.earth_roughness,
                t.wet_roughness,
            ],
        );
        write_vec4(
            p,
            offset_of!(ShoreTuning, fade),
            [t.fine_fade[0], t.fine_fade[1], t.bed_ease_m, 0.0],
        );
    }

    /// `TerrainParams.waterLooks`: every water look the biome holds, then
    /// zeros to `WATER_LOOKS_MAX` rows, ten vec4s a row. A row is its
    /// look's own (`biome.water`) over the shared surface
    /// (`presentation.terrain.water`).
    fn water_looks(&self, p: &mut [u32]) {
        let (b, w) = (&self.biome, &self.tuning.terrain.water);
        let (r, lobes) = (&w.normals, &w.lobes);
        // A puddle's row fades its water in over the first third of its
        // depth, which it carries (`fades.w`): its depth is its shape's.
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
        for (tag, (look, puddle_m)) in deep.chain(puddles).enumerate() {
            let p = &mut p[tag * WATER_LOOK_WORDS..(tag + 1) * WATER_LOOK_WORDS];
            let colours = b.palette(&look.palette);
            write_vec4(
                p,
                offset_of!(WaterLook, channel),
                rgb(linear(colours[0]), 0.0),
            );
            write_vec4(p, offset_of!(WaterLook, edge), rgb(linear(colours[1]), 0.0));
            write_vec4(
                p,
                offset_of!(WaterLook, surface),
                rgb(look.extinction_per_m, w.shadow),
            );
            let [primary, macro_m, detail] = r.tiles_m;
            write_vec4(
                p,
                offset_of!(WaterLook, normals),
                [1.0 / primary, 1.0 / macro_m, 1.0 / detail, r.strength],
            );
            write_vec4(
                p,
                offset_of!(WaterLook, drifts),
                [r.drifts[0], r.drifts[1], r.drifts[2], 0.0],
            );
            let shore = if puddle_m > 0.0 {
                puddle_m / 3.0
            } else {
                w.shore_fade_m
            };
            write_vec4(
                p,
                offset_of!(WaterLook, fades),
                [shore, r.fade_m[0], r.fade_m[1], puddle_m],
            );
            write_vec4(
                p,
                offset_of!(WaterLook, roughness),
                rgb(lobes.roughness, 0.0),
            );
            write_vec4(p, offset_of!(WaterLook, lobes), rgb(lobes.weights, 0.0));
            let f = &w.refraction;
            write_vec4(
                p,
                offset_of!(WaterLook, refraction),
                [f.share, f.depth_m, f.most, f.blur_m],
            );
            let c = &w.caustics;
            write_vec4(
                p,
                offset_of!(WaterLook, caustics),
                [1.0 / c.tile_m, c.strength, c.fade_per_m, c.sharpness],
            );
        }
    }

    pub fn terrain_tables(&self, field: Field) -> TerrainTables {
        let (s, b) = (&self.source, &self.biome);
        let mut nodes = vec![];
        for n in self.plots.nodes.as_chunks::<5>().0 {
            nodes.extend([
                word(n[0]),
                word(n[1]),
                word(n[2]),
                0,
                n[3] as i32 as u32,
                n[4] as i32 as u32,
                0,
                0,
            ]);
        }
        let one = |key: &str| linear(b.palette(key)[0]);
        let rules = &b.field_rules;
        let [ox, oy, cell, cols, rows, levels, _] = field.grid;
        let mut p = vec![0; offset_of!(TerrainParams, view) / 4];
        write_vec4(&mut p, offset_of!(TerrainParams, region), self.plots.region);
        write_vec4(
            &mut p,
            offset_of!(TerrainParams, field),
            [ox, oy, 1.0 / cell, FOOTPRINT_M],
        );
        let grid = offset_of!(TerrainParams, field_grid) / 4;
        p[grid..grid + 4].copy_from_slice(&[cols as u32, rows as u32, levels as u32, 0]);
        write_vec4(
            &mut p,
            offset_of!(TerrainParams, shape),
            [
                rules.edge_warp_m,
                1.0 / rules.edge_warp_scale_m,
                1.0 / rules.mottle_m,
                0.0,
            ],
        );
        write_vec4(
            &mut p,
            offset_of!(TerrainParams, verge),
            rgb(one(&b.verge.palette), b.verge.width_m / 2.0),
        );
        let pixels = self.tuning.terrain.distant.plot_pixels;
        write_vec4(
            &mut p,
            offset_of!(TerrainParams, feathers),
            [
                b.verge.feather_m,
                rules.size_m[0] * pixels[0],
                rules.size_m[0] * pixels[1],
                0.0,
            ],
        );
        for (tag, kind) in AREA_KINDS.iter().enumerate() {
            let at = offset_of!(TerrainParams, roads) / 4 + tag * RoadLook::LAYOUT.size / 4;
            self.road_look(
                &mut p[at..at + RoadLook::LAYOUT.size / 4],
                b.road(kind),
                tag,
            );
        }
        let layer = |tag: usize| {
            b.road(AREA_KINDS[tag])
                .layer
                .unwrap_or((AREA_KINDS.len() - tag) as f64)
        };
        let mut order: Vec<usize> = (0..AREA_KINDS.len()).collect();
        order.sort_by(|&x, &y| layer(x).total_cmp(&layer(y)).then(y.cmp(&x)));
        let at = offset_of!(TerrainParams, road_order) / 4;
        for (i, &tag) in order.iter().enumerate() {
            p[at + i] = tag as u32;
        }
        let f = &b.forest_floor;
        let [litter, moss, humus] = [0, 1, 2].map(|i| linear(b.palette(&f.palette)[i]));
        write_vec4(
            &mut p,
            offset_of!(TerrainParams, forest_litter),
            rgb(litter, 0.0),
        );
        write_vec4(
            &mut p,
            offset_of!(TerrainParams, forest_moss),
            rgb(moss, f.moss),
        );
        write_vec4(
            &mut p,
            offset_of!(TerrainParams, forest_humus),
            rgb(humus, f.humus),
        );
        write_vec4(
            &mut p,
            offset_of!(TerrainParams, forest_detail),
            [1.0 / f.patch_m, f.mottle, f.roughness, f.roots],
        );
        write_vec4(
            &mut p,
            offset_of!(TerrainParams, forest_verge),
            [
                f.verge_m,
                f.verge_warp_m,
                1.0 / f.verge_warp_scale_m,
                1.0 / f.roots_m,
            ],
        );
        write_vec4(
            &mut p,
            offset_of!(TerrainParams, forest_dapple),
            [1.0 / f.dapple.size_m, f.dapple.share, f.dapple.sun, 0.0],
        );
        write_vec4(
            &mut p,
            offset_of!(TerrainParams, forest_line),
            [f.tree_line.taper_m, f.tree_line.warp_m, 0.0, 0.0],
        );
        write_vec4(
            &mut p,
            offset_of!(TerrainParams, water_bed),
            rgb(one("water_bed"), s.grid.spacing * SQRT_2),
        );
        let (shore, bank) = (&b.shore, b.palette(&b.shore.palette));
        write_vec4(
            &mut p,
            offset_of!(TerrainParams, shore),
            rgb(linear(bank[0]), shore.wet_m),
        );
        write_vec4(
            &mut p,
            offset_of!(TerrainParams, shore_earth),
            rgb(linear(bank[1]), shore.mud_m),
        );
        write_vec4(
            &mut p,
            offset_of!(TerrainParams, shore_edge),
            [
                shore.wander,
                1.0 / shore.wander_scale_m,
                shore.lift,
                shore.relief,
            ],
        );
        write_vec4(
            &mut p,
            offset_of!(TerrainParams, distant),
            rgb(one("distant"), 0.0),
        );
        let shore = offset_of!(TerrainParams, shore_tuning) / 4;
        self.shore_tuning(&mut p[shore..shore + ShoreTuning::LAYOUT.size / 4]);
        let water = offset_of!(TerrainParams, water_looks) / 4;
        self.water_looks(&mut p[water..]);
        let sc = &b.scars;
        let look = offset_of!(ScarParams, full);
        let mut scars = vec![0; (ScarParams::LAYOUT.size - look) / 4];
        write_vec4(
            &mut scars,
            offset_of!(ScarParams, full) - look,
            [&sc.crater.mark, &sc.scorch, &sc.tracks, &sc.trampled].map(|m| 255.0 / m.full),
        );
        let mark = |out: &mut [u32], offset, palette: &str, strength: f64| {
            write_vec4(out, offset - look, rgb(one(palette), strength))
        };
        mark(
            &mut scars,
            offset_of!(ScarParams, crater),
            &sc.crater.mark.palette,
            sc.crater.mark.strength,
        );
        mark(
            &mut scars,
            offset_of!(ScarParams, ejecta),
            &sc.crater.ejecta_palette,
            sc.crater.ejecta,
        );
        for (offset, m) in [
            (offset_of!(ScarParams, scorch), &sc.scorch),
            (offset_of!(ScarParams, tracks), &sc.tracks),
            (offset_of!(ScarParams, trampled), &sc.trampled),
        ] {
            mark(&mut scars, offset, &m.palette, m.strength);
        }
        write_vec4(
            &mut scars,
            offset_of!(ScarParams, relief) - look,
            [sc.crater.relief_m, sc.crater.rim, 0.0, 0.0],
        );
        write_vec4(
            &mut scars,
            offset_of!(ScarParams, grass) - look,
            [sc.grass.thin, sc.grass.tracks_thin, sc.grass.flatten, 0.0],
        );
        TerrainTables {
            heights: s.grid.pages(),
            nodes: at_least_one(nodes, 8),
            plots: self.plot_records(true),
            params: p,
            scars,
            surfaces: field.records,
            surface_index: field.index,
            water: self.water_cells(),
        }
    }

    /// The look table's row for the paved kind tagged `tag`, drawn as
    /// `road`. Its patches' colour is held to the surface's luminance: a
    /// patch is a change of hue alone.
    fn road_look(&self, out: &mut [u32], road: &Road, tag: usize) {
        let palettes = &self.biome.palettes;
        let first = |key: &str| linear(palettes[key][0]);
        let (core, worn) = (first(&road.palette), linear(palettes[&road.palette][1]));
        let level = luminance(core) / luminance(worn);
        let walk_row = road.walk.as_ref().map_or(tag, |w| {
            AREA_KINDS.iter().position(|k| *k == w.kind).unwrap()
        });
        let walk = |f: fn(&super::biome::Walk) -> f64| road.walk.as_ref().map_or(0.0, f);
        let (s, r) = (&road.shoulder, &road.ruts);
        write_vec4(out, offset_of!(RoadLook, core), rgb(core, road.roughness));
        write_vec4(
            out,
            offset_of!(RoadLook, worn),
            [
                worn[0] * level,
                worn[1] * level,
                worn[2] * level,
                road.mottle,
            ],
        );
        write_vec4(
            out,
            offset_of!(RoadLook, shape),
            [
                road.feather_m,
                1.0 / road.patch_m,
                road.grain,
                1.0 / road.grain_m,
            ],
        );
        write_vec4(
            out,
            offset_of!(RoadLook, join),
            [road.join_m, 0.0, walk(|w| w.width_m), walk_row as f64],
        );
        write_vec4(
            out,
            offset_of!(RoadLook, shoulder),
            rgb(first(&s.palette), s.width_m),
        );
        write_vec4(
            out,
            offset_of!(RoadLook, edge),
            [s.jitter, 1.0 / s.jitter_m, s.grass, s.cover],
        );
        let rut = |i: usize| r.offsets_m.get(i).copied().unwrap_or(0.0);
        write_vec4(
            out,
            offset_of!(RoadLook, ruts),
            [rut(0), rut(1), r.width_m, tan_deg(r.tilt_deg)],
        );
        let strip = &road.centre_strip;
        let carriageway = if is_road(tag as f32) { 1.0 } else { 0.0 };
        write_vec4(
            out,
            offset_of!(RoadLook, track),
            [
                r.tint,
                strip.half_width_m,
                strip.max_road_width_m / 2.0,
                carriageway,
            ],
        );
        let curb = road.curb.as_ref();
        let slab = walk(|w| if w.slab_m != 0.0 { 1.0 / w.slab_m } else { 0.0 });
        write_vec4(
            out,
            offset_of!(RoadLook, slabs),
            [
                slab,
                walk(|w| w.joint),
                curb.map_or(0.0, |c| c.face_m),
                curb.map_or(0.0, |c| tan_deg(c.tilt_deg)),
            ],
        );
        write_vec4(
            out,
            offset_of!(RoadLook, curb),
            curb.map_or([0.0; 4], |c| rgb(first(&c.palette), c.width_m)),
        );
        let marks = road.markings.as_ref();
        write_vec4(
            out,
            offset_of!(RoadLook, paint),
            marks.map_or([0.0; 4], |m| rgb(first(&m.palette), m.cover)),
        );
        write_vec4(
            out,
            offset_of!(RoadLook, marks),
            marks.map_or([0.0, 0.0, 1.0, 0.0], |m| {
                [
                    m.line_m / 2.0,
                    m.dash_m[0],
                    m.dash_m[0] + m.dash_m[1],
                    m.wear,
                ]
            }),
        );
        write_vec4(
            out,
            offset_of!(RoadLook, crossing),
            marks.map_or([1.0, 0.0, 0.0, 0.0], |m| {
                let c = &m.crossing;
                [c.bar_m, c.length_m, c.gap_m, c.inset_m]
            }),
        );
        let slabs = road.slabs.as_ref();
        write_vec4(
            out,
            offset_of!(RoadLook, stones),
            [
                curb.map_or(0.0, |c| 1.0 / c.stone_m),
                curb.map_or(0.0, |c| c.joint),
                slabs.map_or(0.0, |s| 1.0 / s.slab_m),
                slabs.map_or(0.0, |s| s.joint),
            ],
        );
    }
}

fn chunks_of(rivers: &[f32]) -> impl Iterator<Item = &[f32; RIVER]> {
    rivers.as_chunks::<RIVER>().0.iter()
}

/// A storage binding can't be empty: at least one zeroed record of `words`.
fn at_least_one(mut out: Vec<u32>, words: usize) -> Vec<u32> {
    if out.is_empty() {
        out.resize(words, 0);
    }
    out
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
