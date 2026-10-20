//! The runtime assets as every Rust reader takes them: the scene-assets
//! container (`packages/scene-assets/src/codec.ts`) the template library and
//! every appearance bundle travel in, and, with the `files` feature, the
//! runtime directory (`assets/runtime`): its catalog, and each content's file
//! gzipped under its transport hash (`gzip.ts`).
//!
//! A container is a 16-byte header (magic, version, header and body lengths,
//! little-endian), canonical JSON whose typed arrays are `{"$view": i}`, then
//! the arrays. Each view is its type, byte offset into the body and length.

use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::io::Read;

/// What a container holds: its magic, its format version and its name.
#[derive(Clone, Copy, Debug)]
pub struct Kind(pub u32, pub u32, pub &'static str);

/// "BGTL": the template art library.
pub const LIBRARY: Kind = Kind(0x4c54_4742, 3, "template library");
/// "BGAT": an appearance bundle without its textures' pixels.
pub const BUNDLE: Kind = Kind(0x5441_4742, 4, "bundle");

/// A typed array in a container's content.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct View {
    #[serde(rename = "$view")]
    pub at: usize,
}

#[derive(Deserialize)]
struct Header<T> {
    content: T,
    views: Vec<(String, usize, usize)>,
}

/// A container's content, and its typed arrays.
pub struct Container<'a, T> {
    pub content: T,
    views: Vec<(String, usize, usize)>,
    body: &'a [u8],
}

fn refused(kind: Kind, what: &str) -> String {
    format!(
        "{what}: not a {} of format {:#x} v{}, or truncated; re-bake",
        kind.2, kind.0, kind.1
    )
}

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

impl<'a, T: DeserializeOwned> Container<'a, T> {
    /// `bytes` (inflated) as a container of `kind`, or the refusal naming `what`.
    pub fn parse(bytes: &'a [u8], kind: Kind, what: &str) -> Result<Self, String> {
        if bytes.len() < 16 || (word(bytes, 0), word(bytes, 4)) != (kind.0, kind.1) {
            return Err(refused(kind, what));
        }
        let (head, body) = (word(bytes, 8) as usize, word(bytes, 12) as usize);
        if 16 + head + body != bytes.len() {
            return Err(refused(kind, what));
        }
        let header: Header<T> =
            serde_json::from_slice(&bytes[16..16 + head]).map_err(|e| format!("{what}: {e}"))?;
        Ok(Self {
            content: header.content,
            views: header.views,
            body: &bytes[16 + head..],
        })
    }
}

/// A container's content alone, read from the head of `read` (inflating as
/// it goes): the arrays are never read.
pub fn content<T: DeserializeOwned>(
    mut read: impl Read,
    kind: Kind,
    what: &str,
) -> Result<T, String> {
    let fail = |e: std::io::Error| format!("{what}: {e}");
    let mut head = [0u8; 16];
    read.read_exact(&mut head).map_err(fail)?;
    if (word(&head, 0), word(&head, 4)) != (kind.0, kind.1) {
        return Err(refused(kind, what));
    }
    let mut json = vec![0u8; word(&head, 8) as usize];
    read.read_exact(&mut json).map_err(fail)?;
    let header: Header<T> = serde_json::from_slice(&json).map_err(|e| format!("{what}: {e}"))?;
    Ok(header.content)
}

impl<'a, T> Container<'a, T> {
    /// The type of `view`'s elements, if it has one.
    pub fn ty(&self, view: View) -> Option<&str> {
        self.views.get(view.at).map(|v| v.0.as_str())
    }

    /// The bytes of `view`, of type `ty` (`size` bytes an element).
    pub fn bytes(&self, view: View, ty: &str, size: usize) -> Result<&'a [u8], String> {
        match self.views.get(view.at) {
            Some((t, offset, len)) if t == ty => len
                .checked_mul(size)
                .and_then(|bytes| offset.checked_add(bytes))
                .and_then(|end| self.body.get(*offset..end)),
            _ => None,
        }
        .ok_or(format!("view {} is not {ty} within the body", view.at))
    }

    pub fn f32s(&self, view: View) -> Result<Vec<f32>, String> {
        let b = self.bytes(view, "f32", 4)?;
        Ok(b.as_chunks()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c))
            .collect())
    }

    pub fn u32s(&self, view: View) -> Result<Vec<u32>, String> {
        let b = self.bytes(view, "u32", 4)?;
        Ok(b.as_chunks()
            .0
            .iter()
            .map(|c| u32::from_le_bytes(*c))
            .collect())
    }

    pub fn i16s(&self, view: View) -> Result<Vec<i16>, String> {
        let b = self.bytes(view, "i16", 2)?;
        Ok(b.as_chunks()
            .0
            .iter()
            .map(|c| i16::from_le_bytes(*c))
            .collect())
    }

    pub fn u16s(&self, view: View) -> Result<Vec<u16>, String> {
        let b = self.bytes(view, "u16", 2)?;
        Ok(b.as_chunks()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect())
    }

    pub fn u8s(&self, view: View) -> Result<Vec<u8>, String> {
        Ok(self.bytes(view, "u8", 1)?.to_vec())
    }

    /// Indices stored as `u16` or `u32`, widened.
    pub fn indices(&self, view: View) -> Result<Vec<u32>, String> {
        match self.ty(view) {
            Some("u16") => Ok(self.u16s(view)?.into_iter().map(u32::from).collect()),
            _ => self.u32s(view),
        }
    }
}

#[cfg(feature = "files")]
pub use runtime::{Runtime, RUNTIME};

#[cfg(feature = "files")]
mod runtime {
    use flate2::read::GzDecoder;
    use serde_json::Value;
    use std::fs::File;
    use std::io::Read;
    use std::path::{Path, PathBuf};

    /// The repository's runtime directory.
    pub const RUNTIME: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/runtime");

    /// The runtime directory: its catalog, and the content each hash names.
    pub struct Runtime {
        dir: PathBuf,
        pub catalog: Value,
    }

    impl Runtime {
        pub fn open(dir: &Path) -> Result<Self, String> {
            let path = dir.join("catalog.json");
            let text = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let catalog = serde_json::from_slice(&text).map_err(|e| format!("catalog: {e}"))?;
            Ok(Self {
                dir: dir.to_owned(),
                catalog,
            })
        }

        /// The bundle the catalog names for appearance `name`.
        pub fn bundle_hash(&self, name: &str) -> Result<&str, String> {
            self.catalog["appearances"][name]["bundle"]
                .as_str()
                .ok_or(format!("no appearance {name} in the catalog"))
        }

        /// The gzipped file `file` content `hash` travels in, open, its path
        /// in the runtime directory, and the bytes it inflates to.
        fn transport(&self, hash: &str, file: &str) -> Result<(String, File, u64), String> {
            let t = &self.catalog["gzip"][hash];
            let (Some(transport), Some(bytes), Some(raw)) = (
                t["hash"].as_str(),
                t["bytes"].as_u64(),
                t["raw_bytes"].as_u64(),
            ) else {
                return Err(format!("content {hash}: missing gzip transport"));
            };
            let rel = format!("{transport}/{file}");
            let fail = |e: std::io::Error| format!("{rel}: {e}");
            let mut gz = File::open(self.dir.join(&rel)).map_err(fail)?;
            let len = gz.metadata().map_err(fail)?.len();
            if len != bytes {
                let mut head = [0u8; 23];
                if gz.read_exact(&mut head).is_ok() && &head == b"version https://git-lfs" {
                    return Err(format!(
                        "{rel} is a Git LFS pointer; run: git lfs pull --include assets/runtime/{rel}"
                    ));
                }
                return Err(format!("{rel}: {len} bytes, expected {bytes}"));
            }
            Ok((rel, gz, raw))
        }

        /// Content `hash`, from its transport file `file`, inflated.
        pub fn read(&self, hash: &str, file: &str) -> Result<Vec<u8>, String> {
            let (rel, gz, raw) = self.transport(hash, file)?;
            let mut out = Vec::with_capacity(raw as usize);
            GzDecoder::new(gz)
                .read_to_end(&mut out)
                .map_err(|e| format!("{rel}: {e}"))?;
            if out.len() as u64 != raw {
                return Err(format!("{rel}: inflates to {}, expected {raw}", out.len()));
            }
            Ok(out)
        }

        /// Content `hash` from its transport file `file`, read and inflated
        /// only as far as it is read.
        pub fn reader(&self, hash: &str, file: &str) -> Result<impl Read, String> {
            Ok(GzDecoder::new(self.transport(hash, file)?.1))
        }
    }
}
