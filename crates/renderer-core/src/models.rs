//! Units, corpses, ghosts and moving wreck pieces from canonical frame records,
//! drawn from authored appearance bundles through the shared model pipelines.
//!
//! At load, `World.appearances` rows share their authored mesh tiers on the GPU,
//! packed into the shared vertex format (`geometry::pack_vertices`) with each
//! mesh's indices ordered by surface class: a posed body and an
//! articulated hull skin by the frame's palettes; a corpse is its body posed
//! once as it lies (`geometry::posed_mesh`, by presentation's palette for the
//! row); a static state is rigid. Each frame the palettes and the units'
//! instances are written whole; the static world's batches are held between
//! frames (`statics.rs`). Each batch is one run of instances over one row's
//! tier.
use std::collections::HashMap;
use std::sync::Arc;

use presentation::records::{Joint, CAST_PASS, GHOST_PASS, HIGH_PASS, NONE, VIEW_PASS};
use presentation::static_world::props::{DRAWS_PROP, DRAWS_PROP_GROUND};
use presentation::view::IMPOSTOR;
use wgpu::*;

use crate::fog::Layer;

use crate::bundle::{Bundle, BundleSource, Coverage, Material, Mesh, Shape, Texture};
use crate::pipelines::Pipelines;
use crate::statics::Statics;

#[cfg(test)]
mod tests;

mod admission;
mod diagnostics;
mod geometry;
mod impostors;
mod visibility;
use crate::impostor::{CardInstance, Cards};
pub use geometry::{pack_vertices, posed_mesh, VERTEX_BYTES};
use impostors::RestPose;
/// Material rows a material takes in the materials buffer (`MATERIAL_ROWS`).
const MATERIAL_ROWS: usize = 5;

/// How a material's surface is drawn, in the order a mesh's indices take
/// (`SURFACE_CLASSES`).
const CUTOUT: usize = 0;
const OPAQUE: usize = 1;
const ROOM: usize = 2;
const BLENDED: usize = 3;

/// What a pipeline draws of a mesh: one surface class, or `Solid`, the
/// opaque surfaces and the rooms together (what the depth-only pipelines
/// draw: the two are neighbours in the index order).
#[derive(Clone, Copy)]
pub enum Surface {
    Solid,
    Cutout,
    Opaque,
    Room,
    Blended,
}

fn class_of(m: &Material) -> usize {
    if m.interior.is_some() {
        return ROOM;
    }
    match m.coverage {
        Coverage::Cutout { .. } => CUTOUT,
        Coverage::Opaque => OPAQUE,
        Coverage::Blended => BLENDED,
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Part {
    first: u32,
    count: u32,
}

/// One immutable authored geometry tier, shared by appearance routing rows.
struct Drawable {
    vertices: Buffer,
    indices: Buffer,
    /// Each surface class's range of the indices.
    parts: [Part; 4],
    count: u32,
}

impl Drawable {
    fn part(&self, surface: Surface) -> Part {
        match surface {
            Surface::Solid => Part {
                first: self.parts[OPAQUE].first,
                count: self.parts[OPAQUE].count + self.parts[ROOM].count,
            },
            Surface::Cutout => self.parts[CUTOUT],
            Surface::Opaque => self.parts[OPAQUE],
            Surface::Room => self.parts[ROOM],
            Surface::Blended => self.parts[BLENDED],
        }
    }
}

/// What a row's instances are, for the counts.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Soldier,
    Vehicle,
    Corpse,
    Piece,
    /// The map's props and those the side knows (`Appearance.draws` 7, 8).
    Prop,
}

struct Row {
    kind: Kind,
    building: bool,
    prop: bool,
    tiers: Arc<[Drawable]>,
    /// Baked atlas layer; the authored rest pose is kept for alternate bakes.
    card: Option<u32>,
    rest: Option<RestPose>,
}

impl Row {
    fn drawable(&self, tier: usize) -> Option<&Drawable> {
        self.tiers.get(tier)
    }
}

/// One batch's draw: a run of instances over one row's tier.
struct Run {
    row: usize,
    tier: usize,
    /// Drawn in view, cast into the sun's cascades, a placement ghost, or a
    /// high-air model in view (`Batch.pass`).
    pass: u32,
    /// The tier it casts with, if it casts (`Batch.caster`).
    caster: Option<usize>,
    cascade_mask: u32,
    /// The fog group it binds (`Batch.fog`).
    fog: Layer,
    /// Its instances: the frame's from `first`, or a held static batch's.
    held: Option<u32>,
    first: u32,
    count: u32,
}

impl Run {
    fn caster_for(&self, cascade: usize) -> Option<usize> {
        self.caster
            .filter(|_| self.cascade_mask & (1 << cascade) != 0)
    }
}

#[cfg(test)]
#[path = "models/cascade_tests.rs"]
mod cascade_tests;

/// What the last prepared frame draws, by kind.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Drawn {
    pub soldiers: u32,
    pub vehicles: u32,
    pub corpses: u32,
    pub pieces: u32,
    pub props: u32,
    pub ghosts: u32,
    /// Of those, actually drawn as relit six-vertex impostor cards.
    pub impostors: u32,
    /// The instances the frame's batches list.
    pub listed: u32,
    pub triangles: u64,
}

/// The installed textures, one layer each in two arrays:
/// albedo (sRGB), and surface (normal and ORM, linear).
#[derive(Default)]
struct Layers {
    albedo: Vec<Arc<Texture>>,
    surface: Vec<Arc<Texture>>,
    at: HashMap<String, usize>,
}

impl Layers {
    fn layer(&mut self, t: Option<&Arc<Texture>>) -> f32 {
        let Some(t) = t else { return -1.0 };
        let next = if t.srgb {
            self.albedo.len()
        } else {
            self.surface.len()
        };
        let layer = *self.at.entry(t.id.clone()).or_insert_with(|| {
            let list = if t.srgb {
                &mut self.albedo
            } else {
                &mut self.surface
            };
            list.push(t.clone());
            next
        });
        layer as f32
    }
}

/// One array texture holding `textures` with every level:
/// the edge is the largest's, a smaller one upsampled into the finer levels.
fn texture_array(
    device: &Device,
    queue: &Queue,
    label: &str,
    format: TextureFormat,
    textures: &[Arc<Texture>],
) -> TextureView {
    let size = textures.iter().map(|t| t.size).max().unwrap_or(1);
    let levels = size.ilog2() + 1;
    let texture = device.create_texture(&TextureDescriptor {
        label: Some(label),
        size: Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: textures.len().max(1) as u32,
        },
        mip_level_count: levels,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (layer, t) in textures.iter().enumerate() {
        let shift = (size / t.size).ilog2();
        for level in 0..levels {
            let edge = size >> level;
            let upsampled;
            let data = if level >= shift {
                &t.levels[(level - shift) as usize]
            } else {
                let (from, k) = (t.size as usize, t.size as f64 / f64::from(edge));
                let src = &t.levels[0];
                upsampled = (0..edge as usize * edge as usize)
                    .flat_map(|i| {
                        let (y, x) = (i / edge as usize, i % edge as usize);
                        let s = ((y as f64 * k) as usize * from + (x as f64 * k) as usize) * 4;
                        src[s..s + 4].to_vec()
                    })
                    .collect::<Vec<u8>>();
                &upsampled
            };
            queue.write_texture(
                TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level,
                    origin: Origin3d {
                        x: 0,
                        y: 0,
                        z: layer as u32,
                    },
                    aspect: TextureAspect::All,
                },
                data,
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(edge * 4),
                    rows_per_image: Some(edge),
                },
                Extent3d {
                    width: edge,
                    height: edge,
                    depth_or_array_layers: 1,
                },
            );
        }
    }
    texture.create_view(&TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..Default::default()
    })
}

/// A material's packed shader rows.
fn material_rows(m: &Material, layer: &mut dyn FnMut(&str) -> f32) -> [f32; MATERIAL_ROWS * 4] {
    let normal = layer("normal");
    let cutoff = match m.coverage {
        Coverage::Cutout { cutoff } => cutoff,
        _ => 0.0,
    };
    let w = m.wear.unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let b = m.base_color.map(|c| c as f32);
    [
        b[0],
        b[1],
        b[2],
        b[3],
        m.metallic,
        m.roughness,
        m.tint,
        m.colour_scale.unwrap_or(1.0),
        layer("albedo"),
        normal,
        layer("orm"),
        if m.wear.is_some() { 1.0 } else { 0.0 },
        w[0],
        w[1],
        w[2],
        w[3],
        cutoff,
        normal,
        0.0,
        0.0,
    ]
}

/// `meshes`' indices merged (each mesh's vertices after the last's) and
/// ordered by surface class.
fn order_surfaces(meshes: &[&Mesh], class: &dyn Fn(usize) -> usize) -> (Vec<u32>, [Part; 4]) {
    let mut indices = Vec::new();
    let mut parts = [Part::default(); 4];
    for (c, part) in parts.iter_mut().enumerate() {
        let start = indices.len();
        let mut base = 0u32;
        for mesh in meshes {
            for d in mesh.draws.iter().filter(|d| class(d.material) == c) {
                indices.extend(
                    mesh.indices[d.first..d.first + d.count]
                        .iter()
                        .map(|i| i + base),
                );
            }
            base += mesh.vertices() as u32;
        }
        *part = Part {
            first: start as u32,
            count: (indices.len() - start) as u32,
        };
    }
    (indices, parts)
}

/// One tier's drawable from `meshes` and their packed vertices.
fn drawable(
    device: &Device,
    queue: &Queue,
    label: &str,
    bundle: &Bundle,
    meshes: &[(&Mesh, Vec<u8>)],
) -> Drawable {
    let list: Vec<&Mesh> = meshes.iter().map(|(m, _)| *m).collect();
    let (indices, parts) = order_surfaces(&list, &|m| class_of(&bundle.materials[m]));
    let vertices: Vec<u8> = meshes.iter().flat_map(|(_, v)| v.iter().copied()).collect();
    Drawable {
        vertices: crate::buffer(device, queue, label, BufferUsages::VERTEX, &vertices),
        indices: crate::buffer(
            device,
            queue,
            label,
            BufferUsages::INDEX,
            bytemuck::cast_slice(&indices),
        ),
        count: indices.len() as u32,
        parts,
    }
}

pub struct Models {
    installed: Vec<String>,
    row_names: Vec<String>,
    pub submissions: crate::submitted::Submissions,
    rows: Vec<Option<Row>>,
    layout: BindGroupLayout,
    materials: Buffer,
    authored_materials: Vec<f32>,
    surfaces: [bool; 4],
    buildings_shown: bool,
    models_shown: bool,
    albedo: TextureView,
    surface: TextureView,
    sampler: Sampler,
    palette: Buffer,
    instances: Buffer,
    statics: Statics,
    material_bindings: crate::model_material::ModelMaterialBindings,
    runs: Vec<Run>,
    drawn: Drawn,
    xray_instance_count: u32,
    has_xray_meshes: bool,
    high_apart: bool,
    cards: Option<Cards>,
}

fn growable(device: &Device, label: &str, usage: BufferUsages, bytes: u64) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some(label),
        size: bytes.max(256),
        usage: usage | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

impl Models {
    /// Writes `frame`'s palettes and instances, takes its static batches,
    /// and makes each batch a run.
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        frame: &presentation::presentation::Frame,
    ) -> Result<(), String> {
        self.prepare_records(
            device,
            queue,
            &frame.palettes,
            &frame.instances,
            &frame.batches,
        )
    }

    /// Prepare copied presentation records, independent of a platform's WASM
    /// memory or native `Frame` ownership.
    pub fn prepare_records(
        &mut self,
        device: &Device,
        queue: &Queue,
        palettes: &[Joint],
        instance_records: &[presentation::records::Instance],
        batches: &[presentation::records::Batch],
    ) -> Result<(), String> {
        self.statics
            .prepare_records(device, queue, instance_records, batches)?;
        let palette: &[u8] = bytemuck::cast_slice(palettes);
        let instances: &[u8] = bytemuck::cast_slice(instance_records);
        if palette.len() as u64 > self.palette.size() {
            self.palette = growable(
                device,
                "model-palette",
                BufferUsages::STORAGE,
                palette.len() as u64 * 2,
            );
            self.material_bindings.rebuild(
                device,
                &self.layout,
                &self.palette,
                &self.materials,
                &self.albedo,
                &self.surface,
                &self.sampler,
            );
        }
        if instances.len() as u64 > self.instances.size() {
            self.instances = growable(
                device,
                "model-instances",
                BufferUsages::VERTEX,
                instances.len() as u64 * 2,
            );
        }
        queue.write_buffer(&self.palette, 0, palette);
        queue.write_buffer(&self.instances, 0, instances);
        self.runs.clear();
        self.drawn = Drawn::default();
        self.xray_instance_count = 0;
        self.has_xray_meshes = false;
        // The units' batches, from the frame's instances, then the static
        // world's, held.
        let units = batches
            .iter()
            .filter(|b| !self.statics.holds(b.appearance))
            .map(|b| {
                let run = (b.tier, b.pass, b.caster, b.cascade_mask, b.fog);
                (b.appearance, run, None, b.first, b.count)
            });
        let statics = self.statics.listed().map(|h| {
            let run = (h.tier, h.pass, h.caster, h.cascade_mask, h.fog);
            (h.row, run, Some(h.key), 0, h.count)
        });
        let batches: Vec<_> = units.chain(statics).collect();
        let mut card_instances = Vec::new();
        for (appearance, (tier, pass, caster, cascade_mask, fog), held, first, count) in batches {
            let Some(Some(row)) = self.rows.get(appearance as usize) else {
                continue;
            };
            if pass != CAST_PASS {
                self.drawn.listed += count;
            }
            let impostor = tier == IMPOSTOR as u32;
            let tier = tier as usize;
            if impostor {
                let atlas = row.card.ok_or_else(|| {
                    format!(
                        "appearance row {appearance}: impostor requested before its atlas is baked"
                    )
                })?;
                let records = held.map_or(instance_records, |key| self.statics.records(key));
                for record in &records[first as usize..(first + count) as usize] {
                    let mut record = *record;
                    record.tint[3] = atlas as f32;
                    card_instances.push(CardInstance {
                        record,
                        pass,
                        fog: Layer::of(fog),
                    });
                }
                self.drawn.impostors += count;
                self.drawn.triangles += u64::from(count) * 2;
                match row.kind {
                    Kind::Corpse => self.drawn.corpses += count,
                    _ => self.drawn.soldiers += count,
                }
                continue;
            }
            let Some(drawable) = row.drawable(tier) else {
                continue;
            };
            let layer = Layer::of(fog);
            if self.in_world(pass)
                && layer == Layer::Units
                && drawable.part(Surface::Opaque).count > 0
            {
                self.xray_instance_count = self.xray_instance_count.max(first + count);
                let records = held.map_or(instance_records, |key| self.statics.records(key));
                self.has_xray_meshes |= records[first as usize..(first + count) as usize]
                    .iter()
                    .any(|instance| instance.scale[3] > 0.0);
            }
            let ghost = pass == GHOST_PASS;
            self.runs.push(Run {
                row: appearance as usize,
                tier,
                pass,
                caster: (caster != NONE).then_some(caster as usize),
                cascade_mask,
                fog: layer,
                held,
                first,
                count,
            });
            if pass == CAST_PASS {
                continue;
            }
            let d = &mut self.drawn;
            match (ghost, row.kind) {
                (true, _) => d.ghosts += count,
                (_, Kind::Soldier) => d.soldiers += count,
                (_, Kind::Vehicle) => d.vehicles += count,
                (_, Kind::Corpse) => d.corpses += count,
                (_, Kind::Piece) => d.pieces += count,
                (_, Kind::Prop) => d.props += count,
            }
            d.triangles += u64::from(count) * u64::from(drawable.count) / 3;
        }
        if let Some(cards) = &mut self.cards {
            cards.prepare(device, queue, &card_instances);
        }
        Ok(())
    }

    /// The static world's batches the last frame listed: the scenery draws
    /// its own from them.
    pub fn statics(&self) -> &Statics {
        &self.statics
    }

    /// What the last prepared frame draws.
    pub fn drawn(&self) -> Drawn {
        self.drawn
    }

    /// Highest instance extent of drawable opaque unit runs. Instance indices
    /// remain those in the publication, including any holes before these runs.
    pub fn xray_instance_count(&self) -> u32 {
        self.xray_instance_count
    }

    /// Whether the prepared publication contains an eligible silhouette cue.
    pub fn has_xray_meshes(&self) -> bool {
        self.models_shown && self.has_xray_meshes
    }

    /// A browser with a separate transparent canvas lifts high air above the
    /// world and its overlays. A single-output shell keeps it in the world.
    pub fn set_high_apart(&mut self, apart: bool) {
        self.high_apart = apart;
    }

    pub(crate) fn in_world(&self, pass: u32) -> bool {
        pass == VIEW_PASS || (pass == HIGH_PASS && !self.high_apart)
    }

    pub fn has_high_meshes(&self) -> bool {
        self.runs.iter().any(|run| {
            run.pass == HIGH_PASS
                && self.rows[run.row]
                    .as_ref()
                    .is_some_and(|row| self.models_shown || row.prop || row.building)
        }) || (self.models_shown
            && self.surfaces[OPAQUE]
            && self
                .cards
                .as_ref()
                .is_some_and(|cards| cards.has_pass(HIGH_PASS)))
    }

    /// The layer's own group, by the layout it fills.
    pub fn group(&self) -> (&'static str, &BindGroup) {
        ("modelLayout", self.material_bindings.bind_group())
    }

    /// Every run `tier_of` gives a tier, through `range` of its drawable.
    fn each_run(
        &self,
        pass: &mut RenderPass,
        tier_of: impl Fn(&Run) -> Option<usize>,
        range: impl Fn(&Drawable) -> Part,
        submitted: Option<crate::submitted::Pass>,
    ) {
        for run in &self.runs {
            let Some(tier) = tier_of(run) else { continue };
            let Some(row) = &self.rows[run.row] else {
                continue;
            };
            let Some(d) = row.drawable(tier) else {
                continue;
            };
            if (!self.models_shown && !row.prop && !row.building)
                || (row.building && !self.buildings_shown)
            {
                continue;
            }
            let part = range(d);
            if part.count == 0 {
                continue;
            }
            let instances = run
                .held
                .map_or(&self.instances, |key| self.statics.buffer(key));
            pass.set_vertex_buffer(1, instances.slice(..));
            pass.set_vertex_buffer(0, d.vertices.slice(..));
            pass.set_index_buffer(d.indices.slice(..), IndexFormat::Uint32);
            for part in visibility::visible_parts(&d.parts, part, &self.surfaces) {
                if part.count != 0 {
                    pass.draw_indexed(
                        part.first..part.first + part.count,
                        0,
                        run.first..run.first + run.count,
                    );
                    if let Some(which) = submitted {
                        self.submissions.record(which, |stats| {
                            let counter = if row.building {
                                &mut stats.buildings
                            } else {
                                &mut stats.models_including_props
                            };
                            counter.record(part.count, run.count);
                        });
                    }
                }
            }
        }
    }

    /// One surface class in the world, into a pass whose groups are set.
    /// Separate high air draws through `draw_high_air` after the world.
    pub fn draw(&self, pass: &mut RenderPass, surface: Surface) {
        let view = |r: &Run| self.in_world(r.pass).then_some(r.tier);
        self.each_run(
            pass,
            view,
            |d| d.part(surface),
            Some(match surface {
                Surface::Solid | Surface::Cutout => crate::submitted::Pass::Depth,
                _ => crate::submitted::Pass::Color,
            }),
        );
    }

    /// World structures occlude paint and hidden-unit coverage before unit depth.
    pub fn draw_buildings(&self, pass: &mut RenderPass, surface: Surface) {
        self.draw_depth_partition(pass, surface, true);
    }

    /// Movable models and props join depth after ground paint and x-ray sampling.
    pub fn draw_non_buildings(&self, pass: &mut RenderPass, surface: Surface) {
        self.draw_depth_partition(pass, surface, false);
    }

    fn draw_depth_partition(&self, pass: &mut RenderPass, surface: Surface, buildings: bool) {
        let view = |r: &Run| {
            (self.in_world(r.pass)
                && self.rows[r.row]
                    .as_ref()
                    .is_some_and(|row| row.building == buildings))
            .then_some(r.tier)
        };
        self.each_run(
            pass,
            view,
            |d| d.part(surface),
            Some(crate::submitted::Pass::Depth),
        );
    }

    /// As `draw`, the runs fog takes as `layer` only.
    pub fn draw_fogged(&self, pass: &mut RenderPass, surface: Surface, layer: Layer) {
        let view = |r: &Run| (self.in_world(r.pass) && r.fog == layer).then_some(r.tier);
        self.each_run(
            pass,
            view,
            |d| d.part(surface),
            Some(crate::submitted::Pass::Color),
        );
    }

    pub fn draw_high_air(&self, pass: &mut RenderPass, surface: Surface) {
        let high = |r: &Run| (r.pass == HIGH_PASS).then_some(r.tier);
        self.each_run(
            pass,
            high,
            |d| d.part(surface),
            Some(if matches!(surface, Surface::Blended) {
                crate::submitted::Pass::HighColor
            } else {
                crate::submitted::Pass::HighDepth
            }),
        );
    }

    pub fn draw_high_air_fogged(&self, pass: &mut RenderPass, surface: Surface, layer: Layer) {
        let high = |r: &Run| (r.pass == HIGH_PASS && r.fog == layer).then_some(r.tier);
        self.each_run(
            pass,
            high,
            |d| d.part(surface),
            Some(crate::submitted::Pass::HighColor),
        );
    }

    /// One surface class of every caster into a cascade.
    pub fn draw_casters(&self, pass: &mut RenderPass, surface: Surface, cascade: usize) {
        self.each_run(
            pass,
            |r| r.caster_for(cascade),
            |d| d.part(surface),
            Some(crate::submitted::Pass::Cascade(cascade)),
        );
    }

    /// The placement ghosts: whole meshes, every surface.
    pub fn draw_ghosts(&self, pass: &mut RenderPass) {
        let ghost = |r: &Run| (r.pass == GHOST_PASS).then_some(r.tier);
        self.each_run(
            pass,
            ghost,
            |d| Part {
                first: d.parts[CUTOUT].first,
                count: d.count,
            },
            None,
        );
    }

    /// Whether any run is a ghost.
    pub fn has_ghosts(&self) -> bool {
        self.models_shown && self.runs.iter().any(|r| r.pass == GHOST_PASS)
    }
}

impl Models {
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        for buffer in [&self.materials, &self.palette, &self.instances] {
            memory.buffer(buffer);
        }
        memory.view(&self.albedo);
        memory.view(&self.surface);
        for row in self.rows.iter().flatten() {
            for drawable in row.tiers.iter() {
                memory.buffer(&drawable.vertices);
                memory.buffer(&drawable.indices);
            }
        }
        self.statics.record_memory(memory);
        if let Some(cards) = &self.cards {
            cards.record_memory(memory);
        }
    }
}
