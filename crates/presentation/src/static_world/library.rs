//! The template art library (`packages/scene-assets/src/templateLibrary.ts`):
//! per building template and state, a range of rows that place kit modules
//! on it. Rows are columns over the whole library. It and the kit bundles are
//! scene-assets containers (`codec.ts`): a 16-byte header (magic, version,
//! header and body lengths), canonical JSON whose typed arrays are
//! `{"$view": i}`, then the arrays.
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::HashMap;
use std::ops::Range;

const LIBRARY: (u32, u32) = (0x4c544742, 3); // "BGTL"
const BUNDLE: (u32, u32) = (0x54414742, 4); // "BGAT": a bundle without its textures' pixels

/// What a side can know a building as, in the order of every state table.
pub const STATES: [&str; 3] = ["intact", "ruin", "gutted"];
/// Floats per row transform: x, y, z, yaw (radians about +Z), then the scale
/// on the module's own x, y and z.
pub const ROW_FLOATS: usize = 7;

/// A module's mesh bounds in its own frame: min x, y, z, then max.
pub type Bounds = [f32; 6];

/// A container's header: its content, and per typed array its type, byte
/// offset into the body and length.
#[derive(Deserialize)]
struct Header<T> {
    content: T,
    views: Vec<(String, usize, usize)>,
}

#[derive(Deserialize)]
struct View {
    #[serde(rename = "$view")]
    at: usize,
}

/// A container's content and body, refused unless it is `kind` (magic, version).
fn container<'a, T: DeserializeOwned>(
    bytes: &'a [u8],
    kind: (u32, u32),
    what: &str,
) -> Result<(Header<T>, &'a [u8]), String> {
    let word = |at: usize| {
        bytes
            .get(at..at + 4)
            .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
    };
    match (word(0), word(4), word(8), word(12)) {
        (Some(m), Some(v), Some(h), Some(b))
            if (m, v) == kind && 16 + (h + b) as usize == bytes.len() =>
        {
            let header = serde_json::from_slice(&bytes[16..16 + h as usize])
                .map_err(|e| format!("{what}: {e}"))?;
            Ok((header, &bytes[16 + h as usize..]))
        }
        _ => Err(format!(
            "{what}: not a container of format {kind:x?}, or truncated; re-bake"
        )),
    }
}

impl<T> Header<T> {
    /// The bytes of `view`, of type `ty` (`size` bytes an element).
    fn view<'a>(
        &self,
        body: &'a [u8],
        view: &View,
        ty: &str,
        size: usize,
    ) -> Result<&'a [u8], String> {
        match self.views.get(view.at) {
            Some((t, offset, len)) if t == ty => body.get(*offset..offset + len * size),
            _ => None,
        }
        .ok_or(format!("view {} is not {ty} within the body", view.at))
    }
}

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
        let (header, body) = container::<Content>(bytes, LIBRARY, "template library")?;
        let rows = &header.content.rows;
        let words = |view, ty, size| header.view(body, view, ty, size);
        let f32s = words(&rows.transform, "f32", 4)?
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c));
        let u16s = words(&rows.module, "u16", 2)?
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c));
        let (tiers, tint) = (
            words(&rows.tiers, "u8", 1)?.to_vec(),
            words(&rows.tint, "u8", 1)?.to_vec(),
        );
        let (module, transform): (Vec<u16>, Vec<f32>) = (u16s.collect(), f32s.collect());
        let count = module.len();
        let kits = &header.content.kits;
        let modules = header.content.modules.iter().map(|m| {
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
        for t in &header.content.templates {
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

/// A kit bundle's module bounds by name, from the bundle's content as it
/// travels (`splitTextures`), inflated. The mesh views are not read.
pub fn kit_bounds(bytes: &[u8]) -> Result<HashMap<String, Bounds>, String> {
    let (header, _) = container::<Bundle>(bytes, BUNDLE, "bundle")?;
    let bounds = |b: &StateBounds| [b.min[0], b.min[1], b.min[2], b.max[0], b.max[1], b.max[2]];
    Ok(header
        .content
        .states
        .iter()
        .map(|s| (s.name.clone(), bounds(&s.bounds)))
        .collect())
}
