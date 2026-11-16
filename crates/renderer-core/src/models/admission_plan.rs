//! Complete generation planning and preflight, shared by warming and admission.
use super::row::RowInput;
use super::*;
use contract::identity::ContentId;
use std::collections::BTreeMap;

pub(super) struct AdmissionPlan {
    pub rows: Vec<Option<RowInput>>,
    pub bundles: BTreeMap<String, (Arc<Bundle>, ContentId)>,
    pub textures: BTreeMap<String, Arc<crate::bundle::Texture>>,
}
impl AdmissionPlan {
    pub fn new(
        library: &impl BundleSource,
        appearances: &[presentation::records::Appearance],
        strings: &str,
        rest_palette: impl Fn(u32) -> Option<Vec<Joint>>,
        track_side: impl Fn(u32, usize) -> Option<presentation::world::TrackSide>,
    ) -> Result<Self, String> {
        let text = |at: u32, len: u32| &strings[at as usize..(at + len) as usize];
        let mut bundles = std::collections::BTreeMap::new();
        for a in appearances {
            if a.draws != 0 && a.draws != DRAWS_PROP && a.draws != DRAWS_PROP_GROUND {
                continue;
            }
            let name = text(a.name, a.name_len);
            if bundles.contains_key(name) {
                continue;
            }
            let content = library.content_id(name)?;
            let bundle = library.bundle(name)?;
            bundles.insert(name.to_owned(), (bundle, content));
        }
        let mut wanted_textures = std::collections::BTreeMap::new();
        for (bundle, _) in bundles.values() {
            for material in &bundle.materials {
                for &index in material.textures.values() {
                    let texture = &bundle.textures[index];
                    wanted_textures.insert(texture.id.clone(), texture.clone());
                }
            }
        }
        let plans = super::plan::rows(
            appearances,
            strings,
            &bundles,
            &wanted_textures,
            &rest_palette,
            &track_side,
        )?;

        let rows = plans
            .into_iter()
            .zip(appearances)
            .map(|(plan, a)| {
                plan.map(|plan| {
                    let name = text(a.name, a.name_len).to_owned();
                    RowInput {
                        appearance: *a,
                        bundle: bundles[&name].0.clone(),
                        name,
                        plan,
                    }
                })
            })
            .collect();
        Ok(Self {
            rows,
            bundles,
            textures: wanted_textures,
        })
    }
    pub fn store(
        &self,
        store: SharedArtStore,
        layer_limit: u32,
        work: &mut crate::runtime::AdmissionWork,
    ) -> Result<SharedArtStore, String> {
        let wanted = self
            .bundles
            .values()
            .map(|(bundle, id)| (id.clone(), bundle.materials.len()))
            .collect();
        let wanted_cards = self
            .rows
            .iter()
            .flatten()
            .filter_map(|row| row.plan.card.clone())
            .collect();
        let (store, reason) =
            ArtStore::admit(store, &wanted, &self.textures, &wanted_cards, layer_limit)?;
        if reason.is_some() {
            work.rebuild_reason = reason;
        }

        Ok(store)
    }
}
