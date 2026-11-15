//! Desired name routes and newly supplied content, isolated from live ownership.
use super::{ContentId, Library};
use crate::runtime::AdmissionWork;
use presentation::presentation::catalog::{CatalogDraft, Placement};
use presentation::presentation::lab::Asset;
use presentation::presentation::Presentation;
use presentation::world::Detail;
use std::collections::{BTreeMap, HashSet};

/// A source draft retains identities, never old decoded ownership. Preparation
/// resolves held content against the current library and presentation again.
pub struct SourceDraft {
    library: Library,
    decoded: HashSet<ContentId>,
    held: HashSet<ContentId>,
    wanted: BTreeMap<String, ContentId>,
    facts: BTreeMap<ContentId, Asset>,
}

impl SourceDraft {
    pub fn new(library: &Library, presentation: &Presentation) -> Self {
        let decoded: Vec<_> = library.bundles.keys().cloned().collect();
        let missing: HashSet<_> = library
            .missing_content(presentation, &decoded)
            .into_iter()
            .collect();
        Self {
            library: library.empty_generation(),
            held: decoded
                .iter()
                .filter(|id| !missing.contains(*id))
                .cloned()
                .collect(),
            decoded: decoded.into_iter().collect(),
            wanted: BTreeMap::new(),
            facts: BTreeMap::new(),
        }
    }

    pub fn declare(&mut self, name: String, content: ContentId) {
        self.wanted.insert(name, content);
    }

    pub fn add_bundle(
        &mut self,
        content: ContentId,
        bytes: &[u8],
        textures: &[(&str, &[u8])],
        skeleton: Option<&[u8]>,
    ) -> Result<(), String> {
        if self.held.contains(&content) || self.facts.contains_key(&content) {
            return Ok(());
        }
        // Decode facts whenever presentation lacks them, even when the mesh
        // decoder already holds this identity. Supplied facts are never dropped
        // on the strength of a different owner's snapshot.
        // Both decoders finish before either draft owner changes.
        let facts = Asset::from_bundle(bytes, skeleton)
            .map_err(|error| format!("content {content:?}: {error}"))?;
        if !self.decoded.contains(&content) {
            self.library
                .insert_content(content.clone(), bytes, textures)?;
        }
        self.facts.insert(content, facts);
        Ok(())
    }

    pub fn stage(
        mut self,
        live: &Library,
        presentation: &Presentation,
        detail: Detail,
        placements: BTreeMap<String, Placement>,
    ) -> Result<(Library, CatalogDraft, AdmissionWork), String> {
        // A commit between declaration and preparation may retire a held
        // identity. Resolve it now, before creating any prospective GPU work.
        for (name, content) in &self.wanted {
            if !self.library.has_content(content) && !live.has_content(content) {
                return Err(format!(
                    "{name}: source content retired or not supplied: {content:?}"
                ));
            }
        }
        let catalog =
            presentation.stage_catalog(self.wanted.clone(), self.facts, detail, placements)?;
        let work = self.library.take_admission_work();
        let mut library = self.library;
        for (name, content) in &self.wanted {
            if !library.has_content(content) {
                library
                    .bundles
                    .insert(content.clone(), live.bundles[content].clone());
            }
            library.declare(name.clone(), content.clone());
        }
        library.retain(|name| self.wanted.contains_key(name));
        Ok((library, catalog, work))
    }
}
