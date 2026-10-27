//! Appearance bundles as the browser loads them (`scene-assets`' `codec.ts`,
//! `texture.ts`), from the runtime directory the presentation crate reads
//! ([`Runtime`]): the catalog names each appearance's bundle, a `BGAT`
//! container, and its textures travel as their own files. Only what the
//! model layer draws is read: meshes, materials and textures.
use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use presentation::assets::Runtime;
use presentation::assets::{Container, View, BUNDLE};
use serde::Deserialize;
use serde_json::Value;

type Result<T> = std::result::Result<T, String>;

/// The runtime catalog, reading bundles on request.
pub struct Library {
    #[cfg(not(target_arch = "wasm32"))]
    runtime: Runtime,
    #[cfg(target_arch = "wasm32")]
    bundles: BTreeMap<String, Bundle>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Draw {
    pub material: usize,
    pub first: usize,
    pub count: usize,
}

/// One mesh tier (`MeshData`).
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub positions: Vec<f32>,
    /// snorm16 xyzw, w 0.
    pub normals: Vec<i16>,
    pub tangents: Option<Vec<i16>>,
    pub uvs: Vec<f32>,
    pub colors: Vec<u8>,
    /// Skinned only: four joints and unorm16 weights a vertex.
    pub joints: Option<Vec<u8>>,
    pub weights: Option<Vec<u16>>,
    pub indices: Vec<u32>,
    pub draws: Vec<Draw>,
}

impl Mesh {
    pub fn vertices(&self) -> usize {
        self.positions.len() / 3
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Coverage {
    Opaque,
    Cutout { cutoff: f32 },
    Blended,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Material {
    pub name: String,
    /// As authored (`f64`): the scenery multiplies it into its vertices
    /// before narrowing, as the browser does.
    pub base_color: [f64; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub tint: f32,
    #[serde(default)]
    pub textures: BTreeMap<String, usize>,
    pub colour_scale: Option<f32>,
    pub wear: Option<[f32; 4]>,
    pub coverage: Coverage,
    pub interior: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Texture {
    pub id: String,
    /// sRGB colour (albedo), else linear (normal, ORM).
    pub srgb: bool,
    pub size: u32,
    /// Every mip level, finest first, tightly packed RGBA8.
    pub levels: Vec<Vec<u8>>,
}

#[derive(Clone)]
pub struct Node {
    pub name: String,
    pub tiers: Vec<Mesh>,
    /// A static state's highest point (`bounds.max[2]`), where it has bounds.
    pub top: Option<f64>,
}

#[derive(Clone)]
pub enum Shape {
    /// A posed body's tiers.
    Skinned(Vec<Mesh>),
    /// Each node's tiers, in its own frame.
    Articulated(Vec<Node>),
    /// Each state's tiers.
    Static(Vec<Node>),
}

#[derive(Clone)]
pub struct Bundle {
    pub shape: Shape,
    pub materials: Vec<Material>,
    pub textures: Vec<Texture>,
}

/// A bundle's typed arrays, read from its content's `{"$view": i}`s.
type Views<'a> = Container<'a, Value>;

fn view(v: &Value) -> Result<View> {
    serde_json::from_value(v.clone()).map_err(|_| "expected a typed array".to_string())
}

fn mesh(views: &Views, v: &Value) -> Result<Mesh> {
    let some = |key: &str| (!v[key].is_null()).then(|| &v[key]);
    let mesh = Mesh {
        positions: views.f32s(view(&v["positions"])?)?,
        normals: views.i16s(view(&v["normals"])?)?,
        tangents: some("tangents").map(|t| views.i16s(view(t)?)).transpose()?,
        uvs: views.f32s(view(&v["uvs"])?)?,
        colors: views.u8s(view(&v["colors"])?)?,
        joints: some("joints").map(|j| views.u8s(view(j)?)).transpose()?,
        weights: some("weights").map(|w| views.u16s(view(w)?)).transpose()?,
        indices: views.indices(view(&v["indices"])?)?,
        draws: serde_json::from_value(v["draws"].clone()).map_err(|e| e.to_string())?,
    };
    let n = mesh.vertices();
    let skinned_ok = match (&mesh.joints, &mesh.weights) {
        (Some(j), Some(w)) => j.len() == n * 4 && w.len() == n * 4,
        (None, None) => true,
        _ => false,
    };
    if mesh.normals.len() != n * 4
        || mesh.uvs.len() != n * 2
        || mesh.colors.len() != n * 4
        || mesh.tangents.as_ref().is_some_and(|t| t.len() != n * 4)
        || !skinned_ok
        || mesh.indices.iter().any(|&i| i as usize >= n)
        || mesh.draws.iter().map(|d| d.count).sum::<usize>() != mesh.indices.len()
    {
        return Err("malformed mesh".into());
    }
    Ok(mesh)
}

fn tiers(views: &Views, v: &Value) -> Result<Vec<Mesh>> {
    v.as_array()
        .ok_or("no tiers")?
        .iter()
        .map(|m| mesh(views, m))
        .collect()
}

fn nodes(views: &Views, list: &Value) -> Result<Vec<Node>> {
    list.as_array()
        .ok_or("no nodes")?
        .iter()
        .map(|n| {
            Ok(Node {
                name: n["name"].as_str().ok_or("unnamed node")?.to_owned(),
                tiers: tiers(views, &n["tiers"])?,
                top: n["bounds"]["max"][2].as_f64(),
            })
        })
        .collect()
}

impl Library {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open(dir: &Path) -> Result<Self> {
        Runtime::open(dir).map(|runtime| Self { runtime })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn texture(&self, id: &str) -> Result<Texture> {
        let bytes = self.runtime.read(id, "texture.bin")?;
        let end = bytes.iter().take(64).position(|&b| b == b'\n');
        let header = end.and_then(|e| std::str::from_utf8(&bytes[..e]).ok());
        let parsed = header.and_then(|h| {
            let (format, size) = h.split_once(' ')?;
            let (w, h) = size.split_once('x')?;
            let w: u32 = w.parse().ok()?;
            (w.to_string() == h && w.is_power_of_two()).then_some((format, w))
        });
        let Some((format, size)) = parsed else {
            return Err(format!("texture {id}: malformed header"));
        };
        let srgb = match format {
            "rgba8unorm-srgb" => true,
            "rgba8unorm" => false,
            other => return Err(format!("texture {id}: format {other}")),
        };
        let mut at = end.expect("a header line") + 1;
        let mut levels = Vec::new();
        let mut edge = size as usize;
        while edge >= 1 {
            let n = edge * edge * 4;
            levels.push(
                bytes
                    .get(at..at + n)
                    .ok_or(format!("texture {id}: truncated"))?
                    .to_vec(),
            );
            at += n;
            edge >>= 1;
        }
        if at != bytes.len() {
            return Err(format!("texture {id}: length does not match its header"));
        }
        Ok(Texture {
            id: id.to_owned(),
            srgb,
            size,
            levels,
        })
    }

    /// The appearance `name`'s bundle, its textures joined back.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn bundle(&self, name: &str) -> Result<Bundle> {
        let hash = self.runtime.bundle_hash(name)?;
        let bytes = self.runtime.read(hash, "bundle.bin")?;
        let what = format!("bundle {name}");
        let views = Views::parse(&bytes, BUNDLE, &what)?;
        let c = &views.content;
        let fail = |e: String| format!("{what}: {e}");
        let shape = match c["kind"].as_str() {
            Some("skinned") => Shape::Skinned(tiers(&views, &c["tiers"]).map_err(fail)?),
            Some("articulated") => Shape::Articulated(nodes(&views, &c["nodes"]).map_err(fail)?),
            Some("static") => Shape::Static(nodes(&views, &c["states"]).map_err(fail)?),
            other => return Err(format!("{what}: kind {other:?}")),
        };
        let materials: Vec<Material> =
            serde_json::from_value(c["materials"].clone()).map_err(|e| fail(e.to_string()))?;
        let textures = self.runtime.catalog["textures"][hash]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .map(|id| self.texture(id.as_str().ok_or("a texture id is not a string")?))
            .collect::<Result<Vec<_>>>()?;
        for m in &materials {
            if m.textures.values().any(|&t| t >= textures.len()) {
                return Err(format!("{what}: a material names a texture it lacks"));
            }
        }
        Ok(Bundle {
            shape,
            materials,
            textures,
        })
    }
}

#[cfg(target_arch = "wasm32")]
impl Library {
    /// Build a browser library from fetched bundle and texture payloads. The
    /// JS shell owns fetching; this shared parser owns the decoded records.
    pub fn from_bundle(
        name: &str,
        bundle_bytes: &[u8],
        textures: &[(&str, &[u8])],
    ) -> Result<Self> {
        let what = format!("bundle {name}");
        let views = Views::parse(bundle_bytes, BUNDLE, &what)?;
        let c = &views.content;
        let fail = |e: String| format!("{what}: {e}");
        let shape = match c["kind"].as_str() {
            Some("skinned") => Shape::Skinned(tiers(&views, &c["tiers"]).map_err(fail)?),
            Some("articulated") => Shape::Articulated(nodes(&views, &c["nodes"]).map_err(fail)?),
            Some("static") => Shape::Static(nodes(&views, &c["states"]).map_err(fail)?),
            other => return Err(format!("{what}: kind {other:?}")),
        };
        let materials: Vec<Material> =
            serde_json::from_value(c["materials"].clone()).map_err(|e| fail(e.to_string()))?;
        let textures = textures
            .iter()
            .map(|(id, bytes)| texture_bytes(id, bytes))
            .collect::<Result<Vec<_>>>()?;
        for m in &materials {
            if m.textures.values().any(|&t| t >= textures.len()) {
                return Err(format!("{what}: a material names a texture it lacks"));
            }
        }
        Ok(Self {
            bundles: [(
                name.to_owned(),
                Bundle {
                    shape,
                    materials,
                    textures,
                },
            )]
            .into_iter()
            .collect(),
        })
    }

    pub fn bundle(&self, name: &str) -> Result<Bundle> {
        self.bundles
            .get(name)
            .cloned()
            .ok_or_else(|| format!("browser bundle not loaded: {name}"))
    }
}

#[cfg(target_arch = "wasm32")]
fn texture_bytes(id: &str, bytes: &[u8]) -> Result<Texture> {
    let end = bytes.iter().take(64).position(|&b| b == b'\n');
    let header = end.and_then(|e| std::str::from_utf8(&bytes[..e]).ok());
    let parsed = header.and_then(|h| {
        let (format, size) = h.split_once(' ')?;
        let (w, h) = size.split_once('x')?;
        let w: u32 = w.parse().ok()?;
        (w.to_string() == h && w.is_power_of_two()).then_some((format, w))
    });
    let Some((format, size)) = parsed else {
        return Err(format!("texture {id}: malformed header"));
    };
    let srgb = match format {
        "rgba8unorm-srgb" => true,
        "rgba8unorm" => false,
        other => return Err(format!("texture {id}: format {other}")),
    };
    let mut at = end.expect("a header line") + 1;
    let mut levels = Vec::new();
    let mut edge = size as usize;
    while edge >= 1 {
        let n = edge * edge * 4;
        levels.push(
            bytes
                .get(at..at + n)
                .ok_or(format!("texture {id}: truncated"))?
                .to_vec(),
        );
        at += n;
        edge >>= 1;
    }
    Ok(Texture {
        id: id.to_owned(),
        srgb,
        size,
        levels,
    })
}
