//! The native filesystem adapter over the one map resolver
//! (`contract::maps::resolve`). A saved map is a folder of the catalogue,
//! `fixtures/maps/<id>/`: its physical `map.json`, the `SOURCES.json` that
//! pins what it is and where it came from, and its `encounters/<name>.json`.
//! `map.json` stores each building as its template and frame; the resolver
//! materializes it from the physical library `SOURCES.json` names.
//! The browser's adapter fetches the same documents and calls the same
//! resolver through WebAssembly, so both admit a map or refuse it alike.
use std::path::{Path, PathBuf};

use contract::maps::{
    resolve, MapAdmission, MapId, MapSources, ResolveCode, ResolveError, ResolvedMap,
};
use contract::scenario::EncounterDefinition;

/// A catalogue of saved maps and the physical template libraries they pin.
pub struct Catalogue {
    /// The directory of map folders.
    pub maps: PathBuf,
    /// The directory of physical template libraries: a map's `SOURCES.json`
    /// names its library's file there (`catalogue.library`).
    pub libraries: PathBuf,
}

impl Catalogue {
    /// The repository's catalogue, `fixtures/maps/`.
    pub fn shipped() -> Self {
        let fixtures = crate::fixtures::dir();
        Catalogue {
            maps: fixtures.join("maps"),
            libraries: fixtures,
        }
    }

    /// Every map's id, in order: the catalogue's folders.
    pub fn ids(&self) -> Vec<String> {
        names(&self.maps, |path| path.is_dir())
    }

    /// The map `id`, resolved: its physical definition and identity, or the
    /// refusal naming the document and field at fault. Nothing stands in for
    /// a map that does not resolve.
    pub fn load(&self, id: &str) -> Result<ResolvedMap, ResolveError> {
        self.resolve(&self.folder(id)?, id)
    }

    /// The saved map in `folder`, its refusals located under `id`.
    fn resolve(&self, folder: &Path, id: &str) -> Result<ResolvedMap, ResolveError> {
        let read = |name: &str| document(&folder.join(name), &format!("{id}/{name}"));
        let located = |error: ResolveError| ResolveError {
            location: format!("{id}/{}", error.location),
            ..error
        };
        let map = read("map.json")?;
        let sources = read("SOURCES.json")?;
        let library = MapSources::from_json(&sources)
            .map_err(located)?
            .catalogue
            .library;
        let library = document(
            &self.libraries.join(&library),
            &format!("{id}/SOURCES.json.catalogue.library"),
        )?;
        resolve(&map, &sources, &library, MapAdmission::CATALOGUE).map_err(located)
    }

    /// The names of the map's saved encounters, in order.
    pub fn encounters(&self, id: &str) -> Result<Vec<String>, ResolveError> {
        Ok(names(&self.folder(id)?.join("encounters"), |path| {
            path.extension().is_some_and(|e| e == "json")
        }))
    }

    /// The map's saved encounter `name`: the forces, events and scripted
    /// orders a scenario lays on it.
    pub fn encounter(&self, id: &str, name: &str) -> Result<EncounterDefinition, ResolveError> {
        let location = format!("{id}/encounters/{name}.json");
        address(name, &location)?;
        let text = document(
            &self
                .folder(id)?
                .join("encounters")
                .join(format!("{name}.json")),
            &location,
        )?;
        serde_json::from_str(&text).map_err(|e| ResolveError {
            code: ResolveCode::InvalidEncounter,
            location,
            message: e.to_string(),
        })
    }

    fn folder(&self, id: &str) -> Result<PathBuf, ResolveError> {
        address(id, id)?;
        Ok(self.maps.join(id))
    }
}

/// The shipped catalogue's map `id`, resolved.
pub fn load(id: &str) -> Result<ResolvedMap, ResolveError> {
    Catalogue::shipped().load(id)
}

/// A saved map's folder anywhere on disk, such as the one the `mapgen` CLI
/// writes, resolved against the shipped libraries.
pub fn load_folder(folder: &Path) -> Result<ResolvedMap, ResolveError> {
    Catalogue::shipped().resolve(folder, &folder.display().to_string())
}

/// The shipped catalogue's encounter `name` of map `id`.
pub fn encounter(id: &str, name: &str) -> Result<EncounterDefinition, ResolveError> {
    Catalogue::shipped().encounter(id, name)
}

/// A catalogue address is one directory name, never a path: checked before
/// anything is read.
fn address(name: &str, location: &str) -> Result<(), ResolveError> {
    MapId::new(name).map(drop).map_err(|message| ResolveError {
        code: ResolveCode::InvalidId,
        location: location.into(),
        message: message.into(),
    })
}

fn document(path: &Path, location: &str) -> Result<String, ResolveError> {
    std::fs::read_to_string(path).map_err(|e| ResolveError {
        code: ResolveCode::MissingDocument,
        location: location.into(),
        message: format!("reading {}: {e}", path.display()),
    })
}

/// The entries of `dir` that `keep` admits, by file stem, in order; none
/// when there is no such directory.
fn names(dir: &Path, keep: impl Fn(&Path) -> bool) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| keep(path))
        .filter_map(|path| Some(path.file_stem()?.to_str()?.to_owned()))
        .collect();
    names.sort();
    names
}
