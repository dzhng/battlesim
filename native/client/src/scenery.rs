//! The scenery (`frame/sceneryLayer.ts`): the trees, the tree lines'
//! shrubs, the scenery past the map, the forest floor's dressing, felled
//! trees and their stumps, as the frame's static batches list them. The walk
//! is the presentation crate's; this holds each scenery row's meshes and
//! draws its batches through the browser's scenery pipelines, routed by
//! `Appearance.draws` as the browser routes them:
//!
//! - the forest and the tree lines' shrubs, and the stumps, shadowed and
//!   fogged whole (`forestColour`); the scenery past the map, never shadowed
//!   (`backdropColour`); the dressing, shrunk as it nears a few pixels and
//!   drawn from both sides (`dressing*`); a felled tree, tipped and pressed by
//!   its second record (`felled*`);
//! - every one of them into the depth prepass, and whatever the walk lists
//!   as a caster into each of the sun's cascades.
//!
//! Its batches are held between frames with the rest of the static world's
//! (`statics.rs`).
use presentation::presentation::Presentation;
use presentation::records::{CAST_PASS, VIEW_PASS};
use presentation::static_world::ground::mesh::VERTEX_FLOATS;
use presentation::static_world::ground::walk::{Draws, DRAWS};
use presentation::view::TIERS;
use wgpu::*;

use crate::bundle::{Bundle, Library, Shape};
use crate::gpu::buffer;
use crate::pipelines::{Bound, Pipelines};
use crate::statics::Statics;

/// The state scenery draws (winter will be the next biome's).
const STATE: &str = "summer";
/// The material whose triangles are foliage: shaded with leaf clumps.
const FOLIAGE: &str = "leaves";
/// `tier` of a scenery appearance as triangles in the world vertex
/// (`tierMesh`): linear albedo (material colour times vertex colour) in rgb,
/// 1 in the last float for foliage.
pub fn tier_mesh(name: &str, bundle: &Bundle, tier: usize) -> Result<Vec<f32>, String> {
    let Shape::Static(states) = &bundle.shape else {
        return Err(format!("scenery {name} is not a static bundle"));
    };
    let state = states
        .iter()
        .find(|s| s.name == STATE)
        .ok_or(format!("scenery {name} has no {STATE} state"))?;
    let mesh = state
        .tiers
        .get(tier)
        .ok_or(format!("scenery {name} has no tier {tier}"))?;
    let mut out = Vec::with_capacity(mesh.indices.len() * VERTEX_FLOATS);
    for draw in &mesh.draws {
        let material = &bundle.materials[draw.material];
        let foliage = if material.name == FOLIAGE { 1.0 } else { 0.0 };
        for &v in &mesh.indices[draw.first..draw.first + draw.count] {
            let v = v as usize;
            out.extend_from_slice(&mesh.positions[v * 3..v * 3 + 3]);
            for c in 0..3 {
                out.push((f64::from(mesh.normals[v * 4 + c]) / 32767.0) as f32);
            }
            for c in 0..3 {
                let colour = f64::from(mesh.colors[v * 4 + c]) / 255.0;
                out.push((material.base_color[c] * colour) as f32);
            }
            out.push(foliage);
        }
    }
    Ok(out)
}

/// A mesh on the GPU and its vertices.
type Mesh = (Buffer, u32);

/// One scenery row: what draws it, and its kind (`Ground::scenery_kinds`
/// index), whose tiers it draws; a stump draws its own mesh.
struct Row {
    draws: u32,
    kind: usize,
    stump: Option<Mesh>,
}

/// One listed batch: its held records over one row's tier.
struct Run {
    row: usize,
    tier: usize,
    pass: u32,
    key: u32,
    count: u32,
}

/// The instances the last frame lists in view, per population, kind and
/// tier (`kind · TIERS + tier`), and the stumps per kind: what the
/// browser's capture counts (`SceneryCounts`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawn {
    /// Forest, understorey, backdrop, dressing and felled, in `DRAWS` order.
    pub populations: [Vec<u32>; 5],
    pub stumps: Vec<u32>,
}

pub struct Scenery {
    rows: Vec<Option<Row>>,
    /// Per kind, its tiers, shared by every row that draws the kind.
    tiers: Vec<Vec<Mesh>>,
    runs: Vec<Run>,
    prepass: Bound,
    caster: Bound,
    forest: Bound,
    backdrop: Bound,
    dressing_prepass: Bound,
    dressing: Bound,
    felled_prepass: Bound,
    felled_caster: Bound,
    felled: Bound,
}

impl Scenery {
    /// Uploads every scenery row of the presentation's world from
    /// `library`'s bundles; a stump is the presentation's mesh.
    pub fn new(
        device: &Device,
        pipelines: &Pipelines,
        library: &Library,
        presentation: &Presentation,
    ) -> Result<Self, String> {
        let world = presentation.world();
        let kinds = presentation
            .ground()
            .map_or_else(Vec::new, |g| g.scenery_kinds());
        let text = |at: u32, len: u32| &world.strings[at as usize..(at + len) as usize];
        let upload = |name: &str, mesh: &[f32]| -> Mesh {
            let bytes = bytemuck::cast_slice(mesh);
            let vertices = (mesh.len() / VERTEX_FLOATS) as u32;
            (buffer(device, name, BufferUsages::VERTEX, bytes), vertices)
        };
        let mut tiers: Vec<Vec<Mesh>> = kinds.iter().map(|_| Vec::new()).collect();
        let mut rows = Vec::with_capacity(world.appearances.len());
        for (r, a) in world.appearances.iter().enumerate() {
            let name = text(a.name, a.name_len);
            let kind = kinds.iter().position(|k| k == name);
            let (Some(kind), true) = (kind, a.draws != 0) else {
                rows.push(None);
                continue;
            };
            let stump = if a.draws == Draws::Stump as u32 {
                // Only a tree leaves a stump; a shrub's stump row draws nothing.
                let Some(mesh) = presentation.stump_mesh(r as u32) else {
                    rows.push(None);
                    continue;
                };
                Some(upload(name, mesh))
            } else {
                if tiers[kind].is_empty() {
                    let bundle = library.bundle(name)?;
                    tiers[kind] = (0..TIERS)
                        .map(|t| Ok(upload(name, &tier_mesh(name, &bundle, t)?)))
                        .collect::<Result<_, String>>()?;
                }
                None
            };
            rows.push(Some(Row {
                draws: a.draws,
                kind,
                stump,
            }));
        }
        Ok(Self {
            rows,
            tiers,
            runs: Vec::new(),
            prepass: pipelines.bound("sceneryPrepass"),
            caster: pipelines.bound("sceneryCaster"),
            forest: pipelines.bound("forestColour"),
            backdrop: pipelines.bound("backdropColour"),
            dressing_prepass: pipelines.bound("dressingPrepass"),
            dressing: pipelines.bound("dressingColour"),
            felled_prepass: pipelines.bound("felledPrepass"),
            felled_caster: pipelines.bound("felledCaster"),
            felled: pipelines.bound("felledColour"),
        })
    }

    /// Takes the scenery's batches of the static world's the frame lists.
    pub fn prepare(&mut self, statics: &Statics) {
        self.runs.clear();
        for b in statics.listed() {
            let Some(Some(row)) = self.rows.get(b.row as usize) else {
                continue;
            };
            self.runs.push(Run {
                row: b.row as usize,
                tier: if row.stump.is_some() {
                    0
                } else {
                    b.tier as usize
                },
                pass: b.pass,
                key: b.key,
                count: b.count,
            });
        }
    }

    /// What the last prepared frame lists in view.
    pub fn drawn(&self) -> Drawn {
        let kinds = self.tiers.len();
        let mut out = Drawn {
            populations: std::array::from_fn(|_| vec![0; kinds * TIERS]),
            stumps: vec![0; kinds],
        };
        for run in self.runs.iter().filter(|r| r.pass == VIEW_PASS) {
            let row = self.rows[run.row].as_ref().expect("a scenery row");
            let count = self.instances(row, run.count);
            if row.draws == Draws::Stump as u32 {
                out.stumps[row.kind] += count;
            } else {
                out.populations[row.draws as usize - 1][row.kind * TIERS + run.tier] += count;
            }
        }
        out
    }

    /// A felled tree is two records an instance.
    fn instances(&self, row: &Row, records: u32) -> u32 {
        if row.draws == Draws::Felled as u32 {
            records / 2
        } else {
            records
        }
    }

    /// Every run of `pass`, each through the pipeline `bound` routes its
    /// row's `draws` to, with `groups`, from its batch in `statics`.
    fn draw_runs<'a>(
        &'a self,
        pass: &mut RenderPass,
        which: u32,
        bound: impl Fn(Draws) -> &'a Bound,
        groups: &[(&str, &BindGroup)],
        statics: &Statics,
    ) {
        for run in self.runs.iter().filter(|r| r.pass == which) {
            let row = self.rows[run.row].as_ref().expect("a scenery row");
            let draws = DRAWS[row.draws as usize - 1];
            bound(draws).set(pass, groups);
            let (mesh, vertices) = row
                .stump
                .as_ref()
                .unwrap_or(&self.tiers[row.kind][run.tier]);
            pass.set_vertex_buffer(0, mesh.slice(..));
            pass.set_vertex_buffer(1, statics.buffer(run.key).slice(..));
            pass.draw(0..*vertices, 0..self.instances(row, run.count));
        }
    }

    /// The listed casters into one cascade.
    pub fn draw_casters(
        &self,
        pass: &mut RenderPass,
        camera: (&str, &BindGroup),
        statics: &Statics,
    ) {
        let bound = |d| match d {
            Draws::Felled => &self.felled_caster,
            _ => &self.caster,
        };
        self.draw_runs(pass, CAST_PASS, bound, &[camera], statics);
    }

    /// The scenery in view into the depth prepass.
    pub fn draw_depth(&self, pass: &mut RenderPass, camera: (&str, &BindGroup), statics: &Statics) {
        let bound = |d| match d {
            Draws::Felled => &self.felled_prepass,
            Draws::Dressing => &self.dressing_prepass,
            _ => &self.prepass,
        };
        self.draw_runs(pass, VIEW_PASS, bound, &[camera], statics);
    }

    /// The scenery in view, lit; `groups` the frame's (camera, environment,
    /// fog).
    pub fn draw(&self, pass: &mut RenderPass, groups: &[(&str, &BindGroup)], statics: &Statics) {
        let bound = |d| match d {
            Draws::Backdrop => &self.backdrop,
            Draws::Felled => &self.felled,
            Draws::Dressing => &self.dressing,
            _ => &self.forest,
        };
        self.draw_runs(pass, VIEW_PASS, bound, groups, statics);
    }
}
