//! Native runtime fetching for the shared appearance bundle decoder.
use renderer_core::bundle::{Bundle, BundleSource};

use presentation::assets::Runtime;
use std::cell::RefCell;
use std::path::Path;
use std::sync::Arc;

/// Filesystem runtime catalog. Decoding belongs to the shared renderer.
pub struct Library {
    runtime: Runtime,
    decoded: RefCell<renderer_core::bundle::Library>,
}

impl Library {
    pub fn open(dir: &Path) -> Result<Self, String> {
        Runtime::open(dir).map(|runtime| Self {
            runtime,
            decoded: RefCell::new(renderer_core::bundle::Library::default()),
        })
    }

    pub fn bundle(&self, name: &str) -> Result<Arc<Bundle>, String> {
        if let Ok(bundle) = self.decoded.borrow().bundle(name) {
            return Ok(bundle);
        }
        let content = self.runtime.content_id(name)?;
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
        let mut decoded = self.decoded.borrow_mut();
        decoded.insert_bundle(name, content, &bytes, &refs)?;
        decoded.bundle(name)
    }
}

impl BundleSource for Library {
    fn bundle(&self, name: &str) -> Result<Arc<Bundle>, String> {
        self.bundle(name)
    }
}
