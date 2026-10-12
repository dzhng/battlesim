//! The model layer (`models/modelLayer.ts`): the units, corpses, ghosts and
//! moving wreck pieces a `Frame` lists, drawn from the browser's appearance
//! bundles through the browser's model pipelines.
//!
//! At load, every `World.appearances` row becomes its mesh tiers on the GPU,
//! packed into the browser's 48-byte vertex (`packVertices`) with each mesh's
//! indices ordered by surface class (`surfaceParts.ts`): a posed body and an
//! articulated hull skin by the frame's palettes; a corpse is its body posed
//! once as it lies (`posedMesh`, by the presentation crate's palette for the
//! row); a static state is rigid. Each frame the palettes and instances are
//! written whole, and each batch is one run of instances over one row's tier.
use std::collections::HashMap;

use presentation::presentation::{Presentation, GHOST_PASS};
use presentation::records::{Joint, NONE};
use presentation::static_world::batches::{CAST_PASS, VIEW_PASS};
use presentation::view::IMPOSTOR;
use wgpu::*;

use crate::bundle::{Bundle, Coverage, Library, Material, Mesh, Shape, Texture};
use crate::gpu::bind;
use crate::pipelines::Pipelines;

pub const VERTEX_BYTES: usize = 48;
/// Material rows a material takes in the materials buffer (`MATERIAL_ROWS`).
const MATERIAL_ROWS: usize = 5;
/// Shadow casters draw this many tiers coarser than the view: a cascade
/// texel is larger than the detail between tiers (`CASTER_COARSER`).
const CASTER_COARSER: usize = 1;

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

/// One tier of one row on the GPU.
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
}

struct Row {
    kind: Kind,
    /// A drawable of the static world: it casts only in its caster batches.
    placed: bool,
    tiers: Vec<Drawable>,
    /// A body's coarsest mesh posed in its far pose: what its impostor card
    /// shows, drawn in the card's place.
    card: Option<Drawable>,
}

impl Row {
    /// The drawable at `tier`. No impostor cards are baked: an impostor is
    /// the coarsest mesh in the pose its card would show, the far pose or
    /// the way a corpse lies. Only bodies and corpses take the impostor tier.
    fn drawable(&self, tier: usize) -> Option<&Drawable> {
        if tier == IMPOSTOR as usize {
            return self.card.as_ref().or(self.tiers.last());
        }
        self.tiers.get(tier)
    }
}

/// One batch's draw: a run of the frame's instances over one row's tier.
struct Run {
    row: usize,
    tier: usize,
    /// Drawn in view, cast into the sun's cascades, or a placement ghost
    /// (`Batch.pass`).
    pass: u32,
    /// The tier it casts with, if it casts: a unit in view casts one tier
    /// coarser and never as a card; the static world in its caster batches.
    caster: Option<usize>,
    first: u32,
    count: u32,
}

/// What the last prepared frame draws, by kind.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Drawn {
    pub soldiers: u32,
    pub vehicles: u32,
    pub corpses: u32,
    pub pieces: u32,
    pub ghosts: u32,
    /// Of those, at the impostor tier: drawn as the mesh their card shows.
    pub impostors: u32,
    /// The instances the frame's batches list.
    pub listed: u32,
    pub triangles: u64,
}

/// A mesh's vertices in the 48-byte layout (`packVertices`): `rigid` binds
/// every vertex wholly to one palette joint, else the mesh's own skin;
/// `track` is 1 or 2 for a track's links (their scroll), else 0.
pub fn pack_vertices(mesh: &Mesh, material_base: usize, rigid: Option<u8>, track: u16) -> Vec<u8> {
    let n = mesh.vertices();
    let mut material = vec![0u16; n];
    for d in &mesh.draws {
        for &i in &mesh.indices[d.first..d.first + d.count] {
            material[i as usize] = (material_base + d.material) as u16;
        }
    }
    let mut out = vec![0u8; n * VERTEX_BYTES];
    for (v, o) in out.as_chunks_mut::<VERTEX_BYTES>().0.iter_mut().enumerate() {
        for c in 0..3 {
            o[c * 4..c * 4 + 4].copy_from_slice(&mesh.positions[v * 3 + c].to_le_bytes());
        }
        for c in 0..4 {
            o[12 + c * 2..14 + c * 2].copy_from_slice(&mesh.normals[v * 4 + c].to_le_bytes());
        }
        o[20..24].copy_from_slice(&mesh.uvs[v * 2].to_le_bytes());
        o[24..28].copy_from_slice(&mesh.uvs[v * 2 + 1].to_le_bytes());
        o[28..32].copy_from_slice(&mesh.colors[v * 4..v * 4 + 4]);
        let weights = match (rigid, &mesh.joints, &mesh.weights) {
            (None, Some(joints), Some(weights)) => {
                o[32..36].copy_from_slice(&joints[v * 4..v * 4 + 4]);
                // Narrowed to unorm8, still summing to exactly one.
                let w: [u16; 4] = std::array::from_fn(|c| weights[v * 4 + c]);
                let mut narrow = w.map(|x| js_round(f64::from(x) / 65535.0 * 255.0) as i32);
                let heaviest = (0..4).fold(0, |h, c| if w[c] > w[h] { c } else { h });
                narrow[heaviest] += 255 - narrow.iter().sum::<i32>();
                narrow.map(|x| x as u8)
            }
            _ => {
                o[32..36].fill(rigid.unwrap_or(0));
                [255, 0, 0, 0]
            }
        };
        o[36..40].copy_from_slice(&weights);
        if let Some(t) = &mesh.tangents {
            for c in 0..4 {
                o[40 + c] = js_round(f64::from(t[v * 4 + c]) / 258.0) as i8 as u8;
            }
        }
        o[44..46].copy_from_slice(&material[v].to_le_bytes());
        o[46..48].copy_from_slice(&track.to_le_bytes());
    }
    out
}

/// JavaScript's `Math.round`: halves round up.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// `mesh` skinned once by `palette` (bind to model) into a rigid mesh
/// (`posedMesh`): positions, normals and tangents posed, the skin dropped.
pub fn posed_mesh(mesh: &Mesh, palette: &[Joint]) -> Mesh {
    let mut posed = mesh.clone();
    posed.joints = None;
    posed.weights = None;
    let (Some(joints), Some(weights)) = (&mesh.joints, &mesh.weights) else {
        return posed;
    };
    let moved = |j: &Joint, p: [f32; 3], w: f32| -> [f32; 3] {
        let row = |r: &[f32; 4]| r[0] * p[0] + r[1] * p[1] + r[2] * p[2] + r[3] * w;
        [row(&j.x), row(&j.y), row(&j.z)]
    };
    let unit = |s: &[i16]| [s[0], s[1], s[2]].map(|c| f32::from(c) / 32767.0);
    let n = mesh.vertices();
    posed.normals.fill(0);
    if let Some(t) = &mut posed.tangents {
        t.fill(0);
    }
    for v in 0..n {
        let p = [0, 1, 2].map(|c| mesh.positions[v * 3 + c]);
        let normal = unit(&mesh.normals[v * 4..]);
        let tangent = mesh.tangents.as_ref().map(|t| unit(&t[v * 4..]));
        let (mut sp, mut sn, mut st) = ([0.0f32; 3], [0.0f32; 3], [0.0f32; 3]);
        for k in 0..4 {
            let w = f32::from(weights[v * 4 + k]) / 65535.0;
            if w == 0.0 {
                continue;
            }
            let j = &palette[joints[v * 4 + k] as usize];
            let add = |sum: &mut [f32; 3], a: [f32; 3]| (0..3).for_each(|c| sum[c] += a[c] * w);
            add(&mut sp, moved(j, p, 1.0));
            add(&mut sn, moved(j, normal, 0.0));
            if let Some(t) = tangent {
                add(&mut st, moved(j, t, 0.0));
            }
        }
        posed.positions[v * 3..v * 3 + 3].copy_from_slice(&sp);
        let len = |a: [f32; 3]| (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
        let snorm = |a: [f32; 3], l: f32| a.map(|c| js_round(f64::from(c / l * 32767.0)) as i16);
        let ln = len(sn);
        if ln > 0.0 {
            posed.normals[v * 4..v * 4 + 3].copy_from_slice(&snorm(sn, ln));
        }
        if let (Some(out), Some(src)) = (&mut posed.tangents, &mesh.tangents) {
            let lt = len(st);
            if lt > 1e-8 {
                out[v * 4..v * 4 + 3].copy_from_slice(&snorm(st, lt));
                out[v * 4 + 3] = src[v * 4 + 3];
            }
        }
    }
    posed
}

/// The installed textures, one layer each in two arrays (`TextureLayers`):
/// albedo (sRGB), and surface (normal and ORM, linear).
#[derive(Default)]
struct Layers {
    albedo: Vec<Texture>,
    surface: Vec<Texture>,
    at: HashMap<String, usize>,
}

impl Layers {
    fn layer(&mut self, t: Option<&Texture>) -> f32 {
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

/// One array texture holding `textures` with every level (`uploadTextureArray`):
/// the edge is the largest's, a smaller one upsampled into the finer levels.
fn texture_array(
    device: &Device,
    queue: &Queue,
    label: &str,
    format: TextureFormat,
    textures: &[Texture],
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

/// A material's rows (`materialRow`).
fn material_rows(m: &Material, layer: &mut dyn FnMut(&str) -> f32) -> [f32; MATERIAL_ROWS * 4] {
    let normal = layer("normal");
    let cutoff = match m.coverage {
        Coverage::Cutout { cutoff } => cutoff,
        _ => 0.0,
    };
    let w = m.wear.unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let b = m.base_color;
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
/// ordered by surface class (`orderSurfaces`).
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

fn upload(device: &Device, label: &str, usage: BufferUsages, bytes: &[u8]) -> Buffer {
    crate::gpu::buffer(device, label, usage, bytes)
}

/// One tier's drawable from `meshes` and their packed vertices.
fn drawable(
    device: &Device,
    label: &str,
    bundle: &Bundle,
    meshes: &[(&Mesh, Vec<u8>)],
) -> Drawable {
    let list: Vec<&Mesh> = meshes.iter().map(|(m, _)| *m).collect();
    let (indices, parts) = order_surfaces(&list, &|m| class_of(&bundle.materials[m]));
    let vertices: Vec<u8> = meshes.iter().flat_map(|(_, v)| v.iter().copied()).collect();
    Drawable {
        vertices: upload(device, label, BufferUsages::VERTEX, &vertices),
        indices: upload(
            device,
            label,
            BufferUsages::INDEX,
            bytemuck::cast_slice(&indices),
        ),
        count: indices.len() as u32,
        parts,
    }
}

pub struct Models {
    rows: Vec<Option<Row>>,
    layout: BindGroupLayout,
    materials: Buffer,
    albedo: TextureView,
    surface: TextureView,
    sampler: Sampler,
    palette: Buffer,
    instances: Buffer,
    group: BindGroup,
    runs: Vec<Run>,
    drawn: Drawn,
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
    /// Installs every row of the presentation's world from `library`.
    pub fn new(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        library: &Library,
        presentation: &Presentation,
    ) -> Result<Self, String> {
        let world = presentation.world();
        let text = |at: u32, len: u32| &world.strings[at as usize..(at + len) as usize];
        let mut bundles: HashMap<String, (Bundle, usize)> = HashMap::new();
        let mut materials: Vec<f32> = Vec::new();
        let mut layers = Layers::default();
        let mut rows = Vec::new();
        for (r, a) in world.appearances.iter().enumerate() {
            let (name, state) = (text(a.name, a.name_len), text(a.state, a.state_len));
            if !bundles.contains_key(name) {
                let bundle = library.bundle(name)?;
                let base = materials.len() / (MATERIAL_ROWS * 4);
                for m in &bundle.materials {
                    let mut layer =
                        |c: &str| layers.layer(m.textures.get(c).map(|&i| &bundle.textures[i]));
                    materials.extend(material_rows(m, &mut layer));
                }
                bundles.insert(name.to_owned(), (bundle, base));
            }
            let (bundle, base) = &bundles[name];
            let label = format!("{name}-{r}");
            let single = |mesh: &Mesh, rigid| {
                drawable(
                    device,
                    &label,
                    bundle,
                    &[(mesh, pack_vertices(mesh, *base, rigid, 0))],
                )
            };
            let rest = || {
                presentation
                    .rest_palette(r as u32)
                    .ok_or(format!("{name}: no rest pose for row {r}"))
            };
            let row = match &bundle.shape {
                Shape::Skinned(tiers) if a.lies != NONE => {
                    let palette = rest()?;
                    let tiers = tiers
                        .iter()
                        .map(|m| single(&posed_mesh(m, &palette), Some(0)))
                        .collect();
                    Row {
                        kind: Kind::Corpse,
                        placed: false,
                        tiers,
                        card: None,
                    }
                }
                Shape::Skinned(tiers) => Row {
                    kind: Kind::Soldier,
                    placed: false,
                    card: match tiers.last() {
                        Some(m) => Some(single(&posed_mesh(m, &rest()?), Some(0))),
                        None => None,
                    },
                    tiers: tiers.iter().map(|m| single(m, None)).collect(),
                },
                Shape::Articulated(nodes) => {
                    let count = nodes.first().map_or(0, |n| n.tiers.len());
                    let tiers = (0..count)
                        .map(|t| {
                            let meshes: Vec<(&Mesh, Vec<u8>)> = nodes
                                .iter()
                                .enumerate()
                                .map(|(i, n)| {
                                    let track = match n.name.as_str() {
                                        "track_L" => 1,
                                        "track_R" => 2,
                                        _ => 0,
                                    };
                                    (
                                        &n.tiers[t],
                                        pack_vertices(&n.tiers[t], *base, Some(i as u8), track),
                                    )
                                })
                                .collect();
                            drawable(device, &label, bundle, &meshes)
                        })
                        .collect();
                    Row {
                        kind: Kind::Vehicle,
                        placed: false,
                        tiers,
                        card: None,
                    }
                }
                Shape::Static(states) => {
                    // A piece the bundle lacks (a wreck with no turret to
                    // throw) is a row nothing draws, as in the browser.
                    let found = states
                        .iter()
                        .find(|s| s.name == state)
                        .or_else(|| state.is_empty().then(|| states.first()).flatten());
                    let Some(s) = found else {
                        rows.push(None);
                        continue;
                    };
                    Row {
                        kind: Kind::Piece,
                        placed: a.placed != 0,
                        tiers: s.tiers.iter().map(|m| single(m, Some(0))).collect(),
                        card: None,
                    }
                }
            };
            rows.push(Some(row));
        }
        if materials.is_empty() {
            materials.resize(MATERIAL_ROWS * 4, 0.0);
        }
        let materials = upload(
            device,
            "model-materials",
            BufferUsages::STORAGE,
            bytemuck::cast_slice(&materials),
        );
        let albedo = texture_array(
            device,
            queue,
            "model-albedo",
            TextureFormat::Rgba8UnormSrgb,
            &layers.albedo,
        );
        let surface = texture_array(
            device,
            queue,
            "model-surface",
            TextureFormat::Rgba8Unorm,
            &layers.surface,
        );
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("model-textures"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            anisotropy_clamp: 8,
            ..Default::default()
        });
        let layout = pipelines.layout("modelLayout").clone();
        let palette = growable(device, "model-palette", BufferUsages::STORAGE, 0);
        let instances = growable(device, "model-instances", BufferUsages::VERTEX, 0);
        let group = Self::bind_group(
            device, &layout, &palette, &materials, &albedo, &surface, &sampler,
        );
        Ok(Self {
            group,
            rows,
            layout,
            materials,
            albedo,
            surface,
            sampler,
            palette,
            instances,
            runs: Vec::new(),
            drawn: Drawn::default(),
        })
    }

    fn bind_group(
        device: &Device,
        layout: &BindGroupLayout,
        palette: &Buffer,
        materials: &Buffer,
        albedo: &TextureView,
        surface: &TextureView,
        sampler: &Sampler,
    ) -> BindGroup {
        bind(
            device,
            layout,
            "models",
            &[
                palette.as_entire_binding(),
                materials.as_entire_binding(),
                BindingResource::TextureView(albedo),
                BindingResource::TextureView(surface),
                BindingResource::Sampler(sampler),
            ],
        )
    }

    /// Writes `frame`'s palettes and instances, and makes each batch a run.
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        frame: &presentation::presentation::Frame,
    ) {
        let palette: &[u8] = bytemuck::cast_slice(&frame.palettes);
        let instances: &[u8] = bytemuck::cast_slice(&frame.instances);
        if palette.len() as u64 > self.palette.size() {
            self.palette = growable(
                device,
                "model-palette",
                BufferUsages::STORAGE,
                palette.len() as u64 * 2,
            );
            self.group = Self::bind_group(
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
        for b in &frame.batches {
            if b.first != NONE && b.pass != CAST_PASS {
                self.drawn.listed += b.count;
            }
            let Some(Some(row)) = self.rows.get(b.appearance as usize) else {
                continue;
            };
            if b.first == NONE || b.count == 0 {
                continue;
            }
            let impostor = b.tier == IMPOSTOR as u32;
            let tier = b.tier as usize;
            let Some(drawable) = row.drawable(tier) else {
                continue;
            };
            let ghost = b.pass == GHOST_PASS;
            let caster = match (row.placed, b.pass) {
                (true, CAST_PASS) => Some(tier),
                (false, VIEW_PASS) if !impostor => {
                    Some((tier + CASTER_COARSER).min(row.tiers.len().saturating_sub(1)))
                }
                _ => None,
            };
            self.runs.push(Run {
                row: b.appearance as usize,
                tier,
                pass: b.pass,
                caster,
                first: b.first,
                count: b.count,
            });
            if b.pass == CAST_PASS {
                continue;
            }
            let d = &mut self.drawn;
            match (ghost, row.kind) {
                (true, _) => d.ghosts += b.count,
                (_, Kind::Soldier) => d.soldiers += b.count,
                (_, Kind::Vehicle) => d.vehicles += b.count,
                (_, Kind::Corpse) => d.corpses += b.count,
                (_, Kind::Piece) => d.pieces += b.count,
            }
            d.impostors += if impostor { b.count } else { 0 };
            d.triangles += u64::from(b.count * drawable.count / 3);
        }
    }

    /// What the last prepared frame draws.
    pub fn drawn(&self) -> Drawn {
        self.drawn
    }

    /// The layer's own group, by the layout it fills.
    pub fn group(&self) -> (&'static str, &BindGroup) {
        ("modelLayout", &self.group)
    }

    /// Every run `tier_of` gives a tier, through `range` of its drawable.
    fn each_run(
        &self,
        pass: &mut RenderPass,
        tier_of: impl Fn(&Run) -> Option<usize>,
        range: impl Fn(&Drawable) -> Part,
    ) {
        pass.set_vertex_buffer(1, self.instances.slice(..));
        for run in &self.runs {
            let Some(tier) = tier_of(run) else { continue };
            let Some(row) = &self.rows[run.row] else {
                continue;
            };
            let Some(d) = row.drawable(tier) else {
                continue;
            };
            let part = range(d);
            if part.count == 0 {
                continue;
            }
            pass.set_vertex_buffer(0, d.vertices.slice(..));
            pass.set_index_buffer(d.indices.slice(..), IndexFormat::Uint32);
            pass.draw_indexed(
                part.first..part.first + part.count,
                0,
                run.first..run.first + run.count,
            );
        }
    }

    /// One surface class of every run but the ghosts, into a pass whose
    /// pipeline and groups are set.
    pub fn draw(&self, pass: &mut RenderPass, surface: Surface) {
        let view = |r: &Run| (r.pass == VIEW_PASS).then_some(r.tier);
        self.each_run(pass, view, |d| d.part(surface));
    }

    /// One surface class of every caster into a cascade (`drawCasters`).
    pub fn draw_casters(&self, pass: &mut RenderPass, surface: Surface) {
        self.each_run(pass, |r| r.caster, |d| d.part(surface));
    }

    /// The placement ghosts: whole meshes, every surface.
    pub fn draw_ghosts(&self, pass: &mut RenderPass) {
        let ghost = |r: &Run| (r.pass == GHOST_PASS).then_some(r.tier);
        self.each_run(pass, ghost, |d| Part {
            first: d.parts[CUTOUT].first,
            count: d.count,
        });
    }

    /// Whether any run is a ghost.
    pub fn has_ghosts(&self) -> bool {
        self.runs.iter().any(|r| r.pass == GHOST_PASS)
    }
}
