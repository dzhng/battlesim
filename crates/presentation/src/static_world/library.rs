//! The template art library (`packages/scene-assets/src/templateLibrary.ts`):
//! per building template and state, a range of rows that place kit modules
//! on it. Rows are columns over the whole library. It and the kit bundles
//! travel as scene-assets containers ([`crate::assets`]).
use crate::assets::{self, Container, View, BUNDLE, LIBRARY};
use crate::world::Corners;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Read;
use std::ops::Range;

/// What a side can know a building as, in the order of every state table.
pub const STATES: [&str; 3] = ["intact", "ruin", "gutted"];
/// Floats per row transform: x, y, z, yaw (radians about +Z), then the scale
/// on the module's own x, y and z.
pub const ROW_FLOATS: usize = 7;

/// A module's mesh bounds in its own frame: min x, y, z, then max.
pub type Bounds = [f32; 6];

#[derive(Deserialize)]
struct Content {
    kits: Vec<Kit>,
    modules: Vec<ModuleRef>,
    templates: Vec<Template>,
    rows: Rows,
}

#[derive(Deserialize)]
struct Kit {
    appearance: String,
}

#[derive(Deserialize)]
struct ModuleRef {
    kit: usize,
    module: String,
}

#[derive(Deserialize)]
struct Template {
    id: String,
    states: HashMap<String, RowRange>,
}

#[derive(Deserialize)]
struct RowRange {
    first: usize,
    count: usize,
}

#[derive(Deserialize)]
struct Rows {
    module: View,
    transform: View,
    tiers: View,
    tint: View,
}

pub struct Module {
    /// The kit appearance it is a state of, and its name there.
    pub kit: String,
    pub name: String,
}

pub struct Library {
    pub modules: Vec<Module>,
    /// Per template id: its rows in each state it has.
    templates: HashMap<String, [Option<Range<usize>>; 3]>,
    /// Per row: its module, transform, tier mask (bit t: drawn at tier t) and
    /// sRGB tint.
    pub module: Vec<u16>,
    pub transform: Vec<f32>,
    pub tiers: Vec<u8>,
    pub tint: Vec<u8>,
}

impl Library {
    /// A library file's bytes, inflated.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let library = Container::<Content>::parse(bytes, LIBRARY, "template library")?;
        let rows = &library.content.rows;
        let (module, transform) = (library.u16s(rows.module)?, library.f32s(rows.transform)?);
        let (tiers, tint) = (library.u8s(rows.tiers)?, library.u8s(rows.tint)?);
        let count = module.len();
        let kits = &library.content.kits;
        let modules = library.content.modules.iter().map(|m| {
            let kit = kits
                .get(m.kit)
                .ok_or("template library: a module names no kit")?;
            Ok(Module {
                kit: kit.appearance.clone(),
                name: m.module.clone(),
            })
        });
        let modules = modules.collect::<Result<Vec<_>, String>>()?;
        if transform.len() != count * ROW_FLOATS || tiers.len() != count || tint.len() != count * 3
        {
            return Err("template library: row columns differ in length".into());
        }
        if module.iter().any(|&m| m as usize >= modules.len()) {
            return Err("template library: a row names no module".into());
        }
        let mut templates = HashMap::new();
        for t in &library.content.templates {
            let states = STATES.map(|s| t.states.get(s).map(|r| r.first..r.first + r.count));
            if states.iter().flatten().any(|r| r.end > count) {
                return Err(format!(
                    "template library: template {} has rows past the end",
                    t.id
                ));
            }
            templates.insert(t.id.clone(), states);
        }
        Ok(Self {
            modules,
            templates,
            module,
            transform,
            tiers,
            tint,
        })
    }

    /// A template's rows in each state, or the refusal naming it.
    pub fn states(&self, id: &str) -> Result<&[Option<Range<usize>>; 3], String> {
        self.templates
            .get(id)
            .ok_or_else(|| format!("template.missing: template \"{id}\" has no art in the library"))
    }
}

#[derive(Deserialize)]
struct Bundle {
    states: Vec<State>,
}

#[derive(Deserialize)]
struct State {
    name: String,
    bounds: StateBounds,
}

#[derive(Deserialize)]
struct StateBounds {
    min: [f32; 3],
    max: [f32; 3],
}

#[derive(Deserialize)]
struct Cornered {
    states: Vec<CorneredState>,
}

#[derive(Deserialize)]
struct CorneredState {
    name: String,
    bounds: Corners,
}

/// A static bundle's states, each with its bounds, from the head of the
/// bundle as it travels, inflating as it is read (`read`): the header alone
/// is read, never the meshes.
pub fn state_corners(read: impl Read, what: &str) -> Result<Vec<(String, Corners)>, String> {
    let content: Cornered = assets::content(read, BUNDLE, what)?;
    Ok(content
        .states
        .into_iter()
        .map(|s| (s.name, s.bounds))
        .collect())
}

/// A kit bundle's module bounds by name, from the bundle's content as it
/// travels (`splitTextures`), inflated. The mesh views are not read.
pub fn kit_bounds(bytes: &[u8]) -> Result<HashMap<String, Bounds>, String> {
    let bundle = Container::<Bundle>::parse(bytes, BUNDLE, "bundle")?;
    let bounds = |b: &StateBounds| [b.min[0], b.min[1], b.min[2], b.max[0], b.max[1], b.max[2]];
    Ok(bundle
        .content
        .states
        .iter()
        .map(|s| (s.name.clone(), bounds(&s.bounds)))
        .collect())
}
