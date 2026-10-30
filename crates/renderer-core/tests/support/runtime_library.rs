//! Runtime bundle and texture transport for asset-backed renderer checks.
use presentation::assets::{Runtime, RUNTIME};
use renderer_core::bundle::Library;
use std::path::Path;

pub fn runtime_library(names: &[impl AsRef<str>]) -> Result<Library, String> {
    let runtime = Runtime::open(Path::new(RUNTIME))?;
    let mut library = Library::default();
    for name in names {
        let name = name.as_ref();
        let hash = runtime.bundle_hash(name)?;
        let bytes = runtime.read(hash, "bundle.bin")?;
        let textures = runtime.catalog["textures"][hash]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .map(|id| {
                let id = id.as_str().ok_or("a texture id is not a string")?;
                Ok((id, runtime.read(id, "texture.bin")?))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let refs = textures
            .iter()
            .map(|(id, bytes)| (*id, bytes.as_slice()))
            .collect::<Vec<_>>();
        library.insert_bundle(name, &bytes, &refs)?;
    }
    Ok(library)
}
