//! Authored appearance bundles decoded from transported runtime bytes.
//! Filesystem and network fetching belong to the platform shells.
use presentation::assets::{Container, View, BUNDLE};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Weak};

type Result<T> = std::result::Result<T, String>;

/// Source of decoded bundles for appearance admission.
pub trait BundleSource {
    fn bundle(&self, name: &str) -> Result<Arc<Bundle>>;
}

/// Fetched bundles retained by a platform shell across renderer rebuilds.
#[derive(Clone, Default)]
pub struct Library {
    bundles: BTreeMap<String, Arc<Bundle>>,
    textures: BTreeMap<String, Weak<Texture>>,
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
    /// before narrowing to GPU precision.
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
    pub textures: Vec<Arc<Texture>>,
}

/// A bundle's typed arrays, read from its content's `{"$view": i}`s.
type Views<'a> = Container<'a, Value>;

fn view(v: &Value) -> Result<View> {
    serde_json::from_value(v.clone()).map_err(|_| "expected a typed array".to_string())
}

fn mesh(views: &Views, v: &Value, materials: usize) -> Result<Mesh> {
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
    for (index, draw) in mesh.draws.iter().enumerate() {
        if draw
            .first
            .checked_add(draw.count)
            .is_none_or(|end| end > mesh.indices.len())
        {
            return Err(format!("draw {index} exceeds mesh indices"));
        }
        if draw.material >= materials {
            return Err(format!(
                "draw {index} names missing material {}",
                draw.material
            ));
        }
    }
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
        || mesh
            .draws
            .iter()
            .try_fold(0usize, |sum, d| sum.checked_add(d.count))
            != Some(mesh.indices.len())
    {
        return Err("malformed mesh".into());
    }
    Ok(mesh)
}

fn tiers(views: &Views, v: &Value, materials: usize) -> Result<Vec<Mesh>> {
    v.as_array()
        .ok_or("no tiers")?
        .iter()
        .map(|m| mesh(views, m, materials))
        .collect()
}

fn nodes(views: &Views, list: &Value, materials: usize) -> Result<Vec<Node>> {
    list.as_array()
        .ok_or("no nodes")?
        .iter()
        .map(|n| {
            Ok(Node {
                name: n["name"].as_str().ok_or("unnamed node")?.to_owned(),
                tiers: tiers(views, &n["tiers"], materials)?,
                top: n["bounds"]["max"][2].as_f64(),
            })
        })
        .collect()
}

/// Decode a bundle and its ordered texture payloads once, through the same
/// reader for native runtime files and browser fetches.
pub fn decode(name: &str, bundle_bytes: &[u8], textures: &[(&str, &[u8])]) -> Result<Bundle> {
    decode_with(name, bundle_bytes, textures, |id, bytes| {
        texture_bytes(id, bytes).map(Arc::new)
    })
}

fn decode_with(
    name: &str,
    bundle_bytes: &[u8],
    textures: &[(&str, &[u8])],
    mut texture: impl FnMut(&str, &[u8]) -> Result<Arc<Texture>>,
) -> Result<Bundle> {
    let what = format!("bundle {name}");
    let views = Views::parse(bundle_bytes, BUNDLE, &what)?;
    let c = &views.content;
    let fail = |e: String| format!("{what}: {e}");
    let materials: Vec<Material> =
        serde_json::from_value(c["materials"].clone()).map_err(|e| fail(e.to_string()))?;
    let shape = match c["kind"].as_str() {
        Some("skinned") => {
            Shape::Skinned(tiers(&views, &c["tiers"], materials.len()).map_err(fail)?)
        }
        Some("articulated") => {
            Shape::Articulated(nodes(&views, &c["nodes"], materials.len()).map_err(fail)?)
        }
        Some("static") => {
            Shape::Static(nodes(&views, &c["states"], materials.len()).map_err(fail)?)
        }
        other => return Err(format!("{what}: kind {other:?}")),
    };
    let textures = textures
        .iter()
        .map(|(id, bytes)| texture(id, bytes))
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

impl Library {
    /// A source draft shares immutable texture payloads without carrying stale
    /// bundle replacements over a catalog committed while the draft is filled.
    pub fn empty_generation(&self) -> Self {
        Self {
            bundles: BTreeMap::new(),
            textures: self.textures.clone(),
        }
    }

    /// Retained immutable sources share payload ownership while a replacement
    /// is staged and while admission reads them.
    pub fn replace_with(&mut self, replacements: Self) {
        self.bundles.extend(replacements.bundles);
        self.textures.extend(replacements.textures);
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&str) -> bool) {
        self.bundles.retain(|name, _| keep(name));
        self.textures
            .retain(|_, texture| texture.strong_count() > 0);
    }

    pub fn from_bundle(name: &str, bytes: &[u8], textures: &[(&str, &[u8])]) -> Result<Self> {
        let mut library = Self::default();
        library.insert_bundle(name, bytes, textures)?;
        Ok(library)
    }

    pub fn insert_bundle(
        &mut self,
        name: &str,
        bytes: &[u8],
        textures: &[(&str, &[u8])],
    ) -> Result<()> {
        let cache = &mut self.textures;
        let bundle = decode_with(name, bytes, textures, |id, bytes| {
            if let Some(texture) = cache.get(id).and_then(Weak::upgrade) {
                return Ok(texture);
            }
            let texture = Arc::new(texture_bytes(id, bytes)?);
            cache.insert(id.to_owned(), Arc::downgrade(&texture));
            Ok(texture)
        })?;
        self.bundles.insert(name.to_owned(), Arc::new(bundle));
        Ok(())
    }

    pub fn bundle(&self, name: &str) -> Result<Arc<Bundle>> {
        self.bundles
            .get(name)
            .cloned()
            .ok_or_else(|| format!("bundle not loaded: {name}"))
    }
}

impl BundleSource for Library {
    fn bundle(&self, name: &str) -> Result<Arc<Bundle>> {
        self.bundle(name)
    }
}

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

impl Library {
    /// Decoded mesh and texture array payloads retained for device recreation.
    /// Excludes allocator, container and string metadata overhead.
    pub fn payload_bytes(&self) -> usize {
        let mut textures = HashSet::new();
        fn mesh_bytes(mesh: &Mesh) -> usize {
            size_of_val(mesh.positions.as_slice())
                + size_of_val(mesh.normals.as_slice())
                + mesh
                    .tangents
                    .as_ref()
                    .map_or(0, |v| size_of_val(v.as_slice()))
                + size_of_val(mesh.uvs.as_slice())
                + size_of_val(mesh.colors.as_slice())
                + mesh
                    .joints
                    .as_ref()
                    .map_or(0, |v| size_of_val(v.as_slice()))
                + mesh
                    .weights
                    .as_ref()
                    .map_or(0, |v| size_of_val(v.as_slice()))
                + size_of_val(mesh.indices.as_slice())
                + size_of_val(mesh.draws.as_slice())
        }
        self.bundles
            .values()
            .map(|bundle| {
                let meshes: usize = match &bundle.shape {
                    Shape::Skinned(tiers) => tiers.iter().map(mesh_bytes).sum(),
                    Shape::Static(nodes) | Shape::Articulated(nodes) => nodes
                        .iter()
                        .flat_map(|node| node.tiers.iter())
                        .map(mesh_bytes)
                        .sum(),
                };
                meshes
                    + bundle
                        .textures
                        .iter()
                        .filter(|texture| textures.insert(Arc::as_ptr(texture)))
                        .flat_map(|texture| texture.levels.iter())
                        .map(|level| level.len())
                        .sum::<usize>()
            })
            .sum()
    }
}
