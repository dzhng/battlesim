//! The scenery: the trees, the tree lines' shrubs, the scenery past the map,
//! the forest floor's dressing, felled trees and their stumps, as the frame's
//! static batches list them. The walk is the presentation crate's; this holds
//! each scenery row's meshes and draws its batches through the scenery
//! pipelines, routed by `Appearance.draws`:
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
use presentation::presentation::catalog::CatalogView;
use presentation::records::{CAST_PASS, VIEW_PASS};
use presentation::static_world::ground::mesh::VERTEX_FLOATS;
use presentation::static_world::ground::walk::{Draws, DRAWS};
use presentation::view::TIERS;
use wgpu::*;

use crate::buffer;
use crate::bundle::{Bundle, BundleSource, Shape};
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
    cascade_mask: u32,
    key: u32,
    count: u32,
}

impl Run {
    fn draws_in(&self, pass: u32, cascade: Option<usize>) -> bool {
        self.pass == pass && cascade.is_none_or(|c| self.cascade_mask & (1 << c) != 0)
    }
}

#[cfg(test)]
#[path = "scenery_cascade_tests.rs"]
mod cascade_tests;

pub struct Scenery {
    pub submissions: crate::submitted::Submissions,
    trees_shown: bool,
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
        queue: &Queue,
        pipelines: &Pipelines,
        library: &impl BundleSource,
        catalog: CatalogView<'_>,
    ) -> Result<Self, String> {
        let world = catalog.world();
        let kinds = catalog
            .ground()
            .map_or_else(Vec::new, |g| g.scenery_kinds());
        Self::from_world(
            device,
            queue,
            pipelines,
            library,
            &world.appearances,
            &world.strings,
            &kinds,
            |row| catalog.stump_mesh(row),
        )
    }

    /// Installs the exact appearance publication and ground kind order. A shell
    /// supplies presentation-owned stump meshes by appearance row, so admission
    /// does not depend on sharing the presentation's native or WASM memory.
    #[allow(clippy::too_many_arguments)]
    pub fn from_world<'a>(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        library: &impl BundleSource,
        appearances: &[presentation::records::Appearance],
        strings: &str,
        kinds: &[String],
        stump_mesh: impl Fn(u32) -> Option<&'a [f32]>,
    ) -> Result<Self, String> {
        let text = |at: u32, len: u32| &strings[at as usize..(at + len) as usize];
        let upload = |name: &str, mesh: &[f32]| -> Mesh {
            let bytes = bytemuck::cast_slice(mesh);
            let vertices = (mesh.len() / VERTEX_FLOATS) as u32;
            (
                buffer(device, queue, name, BufferUsages::VERTEX, bytes),
                vertices,
            )
        };
        let mut tiers: Vec<Vec<Mesh>> = kinds.iter().map(|_| Vec::new()).collect();
        let mut rows = Vec::with_capacity(appearances.len());
        for (r, a) in appearances.iter().enumerate() {
            let name = text(a.name, a.name_len);
            let kind = kinds.iter().position(|k| k == name);
            let (Some(kind), true) = (kind, a.draws != 0) else {
                rows.push(None);
                continue;
            };
            let stump = if a.draws == Draws::Stump as u32 {
                // Only a tree leaves a stump; a shrub's stump row draws nothing.
                let Some(mesh) = stump_mesh(r as u32) else {
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
            submissions: Default::default(),
            trees_shown: true,
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
                cascade_mask: b.cascade_mask,
                key: b.key,
                count: b.count,
            });
        }
    }

    pub fn set_trees_shown(&mut self, shown: bool) {
        self.trees_shown = shown;
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
        cascade: Option<usize>,
        bound: impl Fn(Draws) -> &'a Bound,
        groups: &[(&str, &BindGroup)],
        statics: &Statics,
    ) -> crate::submitted::SubmittedDraws {
        let mut submitted = crate::submitted::SubmittedDraws::default();
        for run in self.runs.iter().filter(|r| r.draws_in(which, cascade)) {
            let row = self.rows[run.row].as_ref().expect("a scenery row");
            let draws = DRAWS[row.draws as usize - 1];
            if !self.trees_shown && draws != Draws::Dressing {
                continue;
            }
            bound(draws).set(pass, groups);
            let (mesh, vertices) = row
                .stump
                .as_ref()
                .unwrap_or(&self.tiers[row.kind][run.tier]);
            pass.set_vertex_buffer(0, mesh.slice(..));
            pass.set_vertex_buffer(1, statics.buffer(run.key).slice(..));
            let count = self.instances(row, run.count);
            pass.draw(0..*vertices, 0..count);
            submitted.record(*vertices, count);
        }
        submitted
    }

    /// The listed casters into one cascade.
    pub fn draw_casters(
        &self,
        pass: &mut RenderPass,
        camera: (&str, &BindGroup),
        statics: &Statics,
        cascade: usize,
    ) {
        let bound = |d| match d {
            Draws::Felled => &self.felled_caster,
            _ => &self.caster,
        };
        let work = self.draw_runs(pass, CAST_PASS, Some(cascade), bound, &[camera], statics);
        self.submissions
            .record(crate::submitted::Pass::Cascade(cascade), |stats| {
                stats.scenery.add(work)
            });
    }

    /// The scenery in view into the depth prepass.
    pub fn draw_depth(&self, pass: &mut RenderPass, camera: (&str, &BindGroup), statics: &Statics) {
        let bound = |d| match d {
            Draws::Felled => &self.felled_prepass,
            Draws::Dressing => &self.dressing_prepass,
            _ => &self.prepass,
        };
        let work = self.draw_runs(pass, VIEW_PASS, None, bound, &[camera], statics);
        self.submissions
            .record(crate::submitted::Pass::Depth, |stats| {
                stats.scenery.add(work)
            });
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
        let work = self.draw_runs(pass, VIEW_PASS, None, bound, groups, statics);
        self.submissions
            .record(crate::submitted::Pass::Color, |stats| {
                stats.scenery.add(work)
            });
    }
}

impl Scenery {
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        for tiers in &self.tiers {
            for (buffer, _) in tiers {
                memory.buffer(buffer);
            }
        }
        for row in self.rows.iter().flatten() {
            if let Some((buffer, _)) = &row.stump {
                memory.buffer(buffer);
            }
        }
    }
}

impl Scenery {
    pub fn diagnostics(&self, walk: [usize; 8], pending: bool, falling: bool) -> serde_json::Value {
        let population = |draws: Draws, placed: usize| {
            let mut tiers = [0u64; TIERS];
            let (mut triangles, mut casters) = (0u64, 0u64);
            for run in &self.runs {
                let row = self.rows[run.row].as_ref().expect("prepared scenery row");
                if row.draws != draws as u32 {
                    continue;
                }
                let count = self.instances(row, run.count);
                if run.pass == CAST_PASS {
                    casters += u64::from(count);
                } else {
                    tiers[run.tier] += u64::from(count);
                    let (_, vertices) = row
                        .stump
                        .as_ref()
                        .unwrap_or(&self.tiers[row.kind][run.tier]);
                    triangles += u64::from(*vertices / 3) * u64::from(count);
                }
            }
            serde_json::json!({"placed": placed, "tiers": tiers, "triangles": triangles, "casters": casters})
        };
        let mut dressing = population(Draws::Dressing, 0);
        let placed: u64 = dressing["tiers"]
            .as_array()
            .expect("tier array")
            .iter()
            .map(|v| v.as_u64().expect("tier count"))
            .sum();
        dressing["placed"] = placed.into();
        dressing["cells"] = walk[3].into();
        dressing["bytes"] = (placed * size_of::<presentation::records::Instance>() as u64).into();
        dressing["pending"] = pending.into();
        let felled = population(Draws::Felled, 0);
        let stumps = population(Draws::Stump, 0);
        let count = |v: &serde_json::Value| {
            v["tiers"]
                .as_array()
                .expect("tier array")
                .iter()
                .map(|n| n.as_u64().expect("tier count"))
                .sum::<u64>()
        };
        let submitted = self.submissions.snapshot();
        serde_json::json!({
            "kinds": self.tiers.iter().filter(|tiers| !tiers.is_empty()).count(),
            "visitedBounds": walk[4],
            "forest": population(Draws::Forest, walk[0]),
            "understorey": population(Draws::Understorey, walk[1]),
            "backdrop": population(Draws::Backdrop, walk[2]),
            "dressing": dressing,
            "felled": { "trees": count(&felled), "stumps": count(&stumps), "falling": falling,
                "work": {"envelopeRowsTotal": walk[5], "snapshotRowsTotal": walk[6], "membershipChecks": walk[7]} },
            "draws": submitted.depth.scenery.draws + submitted.color.scenery.draws + submitted.cascades.iter().map(|c| c.scenery.draws).sum::<u64>(),
        })
    }
}

impl Scenery {
    pub fn drawn_instances(&self) -> serde_json::Value {
        let mut populations: [Vec<u64>; 6] =
            std::array::from_fn(|i| vec![0; self.tiers.len() * if i == 5 { 1 } else { TIERS }]);
        for run in self.runs.iter().filter(|run| run.pass == VIEW_PASS) {
            let row = self.rows[run.row].as_ref().expect("prepared scenery row");
            if !self.trees_shown && row.draws != Draws::Dressing as u32 {
                continue;
            }
            let index = if row.draws == Draws::Stump as u32 {
                row.kind
            } else {
                row.kind * TIERS + run.tier
            };
            populations[row.draws as usize - 1][index] += u64::from(self.instances(row, run.count));
        }
        let [forest, understorey, backdrop, dressing, felled, stumps] = populations;
        serde_json::json!({"forest": forest, "understorey": understorey, "backdrop": backdrop,
            "dressing": dressing, "felled": felled, "stumps": stumps})
    }
}
