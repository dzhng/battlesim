//! Appearance bundles as the browser loads them (`scene-assets`' `gzip.ts`,
//! `codec.ts`, `texture.ts`): the runtime catalog names each appearance's
//! bundle by its art hash; the file travels gzipped under its transport hash,
//! as a `BGAT` container (a JSON header whose typed arrays are `{"$view": i}`,
//! then the arrays), and its textures travel as their own files. Only what
//! the model layer draws is read: meshes, materials and textures.
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

/// The runtime directory, beside the catalog.
pub const RUNTIME: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/runtime");
/// "BGAT", little-endian: a bundle without its textures' pixels.
const TRANSPORT_MAGIC: u32 = 0x5441_4742;
const FORMAT_VERSION: u32 = 4;

type Result<T> = std::result::Result<T, String>;

#[derive(Deserialize)]
struct Catalog {
    appearances: BTreeMap<String, CatalogEntry>,
    gzip: BTreeMap<String, Transport>,
    #[serde(default)]
    textures: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize)]
struct CatalogEntry {
    bundle: String,
}

#[derive(Deserialize)]
struct Transport {
    hash: String,
    bytes: usize,
    raw_bytes: usize,
}

/// The runtime catalog, reading bundles on request.
pub struct Library {
    dir: PathBuf,
    catalog: Catalog,
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
    pub base_color: [f32; 4],
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

pub struct Node {
    pub name: String,
    pub tiers: Vec<Mesh>,
}

pub enum Shape {
    /// A posed body's tiers.
    Skinned(Vec<Mesh>),
    /// Each node's tiers, in its own frame.
    Articulated(Vec<Node>),
    /// Each state's tiers.
    Static(Vec<Node>),
}

pub struct Bundle {
    pub shape: Shape,
    pub materials: Vec<Material>,
    pub textures: Vec<Texture>,
}

#[derive(Deserialize)]
struct Header {
    content: Value,
    views: Vec<(String, usize, usize)>,
}

/// A container's typed arrays, by view index.
struct Views<'a> {
    body: &'a [u8],
    views: Vec<(String, usize, usize)>,
}

impl Views<'_> {
    fn bytes(&self, v: &Value, ty: &str, size: usize) -> Result<&[u8]> {
        let i = v["$view"].as_u64().ok_or("expected a typed array")? as usize;
        let (t, offset, len) = self.views.get(i).ok_or(format!("missing view {i}"))?;
        if t != ty {
            return Err(format!("view {i} is {t}, expected {ty}"));
        }
        self.body
            .get(*offset..offset + len * size)
            .ok_or(format!("view {i} runs past the body"))
    }

    fn f32s(&self, v: &Value) -> Result<Vec<f32>> {
        let b = self.bytes(v, "f32", 4)?;
        Ok(b.as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c))
            .collect())
    }

    fn i16s(&self, v: &Value) -> Result<Vec<i16>> {
        let b = self.bytes(v, "i16", 2)?;
        Ok(b.as_chunks::<2>()
            .0
            .iter()
            .map(|c| i16::from_le_bytes(*c))
            .collect())
    }

    fn u16s(&self, v: &Value) -> Result<Vec<u16>> {
        let b = self.bytes(v, "u16", 2)?;
        Ok(b.as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect())
    }

    fn u8s(&self, v: &Value) -> Result<Vec<u8>> {
        Ok(self.bytes(v, "u8", 1)?.to_vec())
    }

    fn indices(&self, v: &Value) -> Result<Vec<u32>> {
        let i = v["$view"].as_u64().ok_or("expected indices")? as usize;
        match self.views.get(i).map(|v| v.0.as_str()) {
            Some("u16") => Ok(self.u16s(v)?.into_iter().map(u32::from).collect()),
            _ => {
                let b = self.bytes(v, "u32", 4)?;
                Ok(b.as_chunks::<4>()
                    .0
                    .iter()
                    .map(|c| u32::from_le_bytes(*c))
                    .collect())
            }
        }
    }

    fn mesh(&self, v: &Value) -> Result<Mesh> {
        let some = |key: &str| (!v[key].is_null()).then(|| &v[key]);
        let mesh = Mesh {
            positions: self.f32s(&v["positions"])?,
            normals: self.i16s(&v["normals"])?,
            tangents: some("tangents").map(|t| self.i16s(t)).transpose()?,
            uvs: self.f32s(&v["uvs"])?,
            colors: self.u8s(&v["colors"])?,
            joints: some("joints").map(|j| self.u8s(j)).transpose()?,
            weights: some("weights").map(|w| self.u16s(w)).transpose()?,
            indices: self.indices(&v["indices"])?,
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

    fn tiers(&self, v: &Value) -> Result<Vec<Mesh>> {
        v.as_array()
            .ok_or("no tiers")?
            .iter()
            .map(|m| self.mesh(m))
            .collect()
    }

    fn nodes(&self, list: &Value) -> Result<Vec<Node>> {
        list.as_array()
            .ok_or("no nodes")?
            .iter()
            .map(|n| {
                Ok(Node {
                    name: n["name"].as_str().ok_or("unnamed node")?.to_owned(),
                    tiers: self.tiers(&n["tiers"])?,
                })
            })
            .collect()
    }
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

impl Library {
    pub fn open(dir: &Path) -> Result<Self> {
        let path = dir.join("catalog.json");
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let catalog = serde_json::from_str(&text).map_err(|e| format!("catalog: {e}"))?;
        Ok(Self {
            dir: dir.to_owned(),
            catalog,
        })
    }

    /// The original content `hash`, from its gzip transport file `file`.
    fn read(&self, hash: &str, file: &str) -> Result<Vec<u8>> {
        let t = self
            .catalog
            .gzip
            .get(hash)
            .ok_or(format!("content {hash}: missing gzip transport"))?;
        let rel = format!("{}/{file}", t.hash);
        let bytes = std::fs::read(self.dir.join(&rel)).map_err(|e| format!("{rel}: {e}"))?;
        if bytes.starts_with(b"version https://git-lfs") {
            return Err(format!(
                "{rel} is a Git LFS pointer; run: git lfs pull --include assets/runtime/{rel}"
            ));
        }
        if bytes.len() != t.bytes {
            return Err(format!(
                "{rel}: {} bytes, expected {}",
                bytes.len(),
                t.bytes
            ));
        }
        let mut raw = Vec::with_capacity(t.raw_bytes);
        flate2::read::GzDecoder::new(&bytes[..])
            .read_to_end(&mut raw)
            .map_err(|e| format!("{rel}: {e}"))?;
        if raw.len() != t.raw_bytes {
            return Err(format!(
                "{rel}: inflates to {}, expected {}",
                raw.len(),
                t.raw_bytes
            ));
        }
        Ok(raw)
    }

    fn texture(&self, id: &str) -> Result<Texture> {
        let bytes = self.read(id, "texture.bin")?;
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
    pub fn bundle(&self, name: &str) -> Result<Bundle> {
        let hash = &self
            .catalog
            .appearances
            .get(name)
            .ok_or(format!("no appearance {name} in the catalog"))?
            .bundle;
        let bytes = self.read(hash, "bundle.bin")?;
        let what = format!("bundle {name}");
        if bytes.len() < 16 || u32_at(&bytes, 0) != TRANSPORT_MAGIC {
            return Err(format!("{what}: not a bundle"));
        }
        if u32_at(&bytes, 4) != FORMAT_VERSION {
            return Err(format!(
                "{what}: format {}, expected {FORMAT_VERSION}",
                u32_at(&bytes, 4)
            ));
        }
        let (head, body) = (u32_at(&bytes, 8) as usize, u32_at(&bytes, 12) as usize);
        if 16 + head + body != bytes.len() {
            return Err(format!("{what}: length does not match its header"));
        }
        let header: Header =
            serde_json::from_slice(&bytes[16..16 + head]).map_err(|e| format!("{what}: {e}"))?;
        let views = Views {
            body: &bytes[16 + head..],
            views: header.views,
        };
        let c = &header.content;
        let fail = |e: String| format!("{what}: {e}");
        let shape = match c["kind"].as_str() {
            Some("skinned") => Shape::Skinned(views.tiers(&c["tiers"]).map_err(fail)?),
            Some("articulated") => Shape::Articulated(views.nodes(&c["nodes"]).map_err(fail)?),
            Some("static") => Shape::Static(views.nodes(&c["states"]).map_err(fail)?),
            other => return Err(format!("{what}: kind {other:?}")),
        };
        let materials: Vec<Material> =
            serde_json::from_value(c["materials"].clone()).map_err(|e| fail(e.to_string()))?;
        let textures = self
            .catalog
            .textures
            .get(hash)
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .map(|id| self.texture(id))
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
