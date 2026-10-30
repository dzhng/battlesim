//! Native runtime fetching for the shared appearance bundle decoder.
use renderer_core::bundle::{Bundle, BundleSource};

use presentation::assets::Runtime;
use std::path::Path;
use std::sync::Arc;

/// Filesystem runtime catalog. Decoding belongs to the shared renderer.
pub struct Library {
    runtime: Runtime,
}

impl Library {
    pub fn open(dir: &Path) -> Result<Self, String> {
        Runtime::open(dir).map(|runtime| Self { runtime })
    }

    pub fn bundle(&self, name: &str) -> Result<Arc<Bundle>, String> {
        let hash = self.runtime.bundle_hash(name)?;
        let bytes = self.runtime.read(hash, "bundle.bin")?;
        let textures = self.runtime.catalog["textures"][hash]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .map(|id| {
                let id = id.as_str().ok_or("a texture id is not a string")?;
                Ok((id.to_owned(), self.runtime.read(id, "texture.bin")?))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let refs = textures
            .iter()
            .map(|(id, data)| (id.as_str(), data.as_slice()))
            .collect::<Vec<_>>();
        renderer_core::bundle::decode(name, &bytes, &refs).map(Arc::new)
    }
}

impl BundleSource for Library {
    fn bundle(&self, name: &str) -> Result<Arc<Bundle>, String> {
        self.bundle(name)
    }
}
