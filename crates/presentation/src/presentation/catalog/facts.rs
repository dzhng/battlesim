//! Immutable content facts and the names routing to them in one catalog.
use super::{Asset, ContentId};
use crate::units::drawn::ClipTables;
use crate::world::Appearance;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Default)]
pub(in crate::presentation) struct Facts {
    names: BTreeMap<String, ContentId>,
    contents: BTreeMap<ContentId, Arc<Asset>>,
}

impl Facts {
    pub(in crate::presentation) fn installed(
        assets: BTreeMap<String, (ContentId, Asset)>,
        tables: Option<&ClipTables>,
    ) -> Self {
        let mut facts = Self::default();
        for (name, (content, asset)) in assets {
            if !facts.contents.contains_key(&content) {
                let asset = share_table(asset, tables, facts.contents.values());
                facts.contents.insert(content.clone(), Arc::new(asset));
            }
            facts.names.insert(name, content);
        }
        facts
    }

    pub(super) fn stage(
        &self,
        wanted: BTreeMap<String, ContentId>,
        mut supplied: BTreeMap<ContentId, Asset>,
        tables: Option<&ClipTables>,
    ) -> Result<Self, String> {
        let mut contents = BTreeMap::new();
        for (name, content) in &wanted {
            if contents.contains_key(content) {
                continue;
            }
            let asset = supplied
                .remove(content)
                .map(|asset| {
                    Arc::new(share_table(
                        asset,
                        tables,
                        self.contents.values().chain(contents.values()),
                    ))
                })
                .or_else(|| self.contents.get(content).cloned())
                .ok_or_else(|| format!("{name}: catalog facts missing for content {content:?}"))?;
            contents.insert(content.clone(), asset);
        }
        Ok(Self {
            names: wanted,
            contents,
        })
    }

    pub(in crate::presentation) fn contains_content(&self, content: &ContentId) -> bool {
        self.contents.contains_key(content)
    }

    pub(in crate::presentation) fn get(&self, name: &str) -> Option<&Asset> {
        self.contents.get(self.names.get(name)?).map(Arc::as_ref)
    }

    pub(in crate::presentation) fn iter(&self) -> impl Iterator<Item = (&String, &Asset)> {
        self.names
            .iter()
            .map(|(name, content)| (name, self.contents[content].as_ref()))
    }

    pub(super) fn keys(&self) -> impl Iterator<Item = &String> {
        self.names.keys()
    }
}

/// A scene holds one clip allocation per table content, including facts just
/// fielded in Rust. New model content can reuse a resident rig's clip data.
fn share_table<'a>(
    mut asset: Asset,
    tables: Option<&ClipTables>,
    mut held: impl Iterator<Item = &'a Arc<Asset>>,
) -> Asset {
    if let Asset::Animated {
        appearance, table, ..
    } = &mut asset
    {
        if let Appearance::Skinned { table: key, .. } = appearance.as_ref() {
            let shared = tables.and_then(|t| t.get(key)).cloned().or_else(|| {
                held.find_map(|asset| match asset.as_ref() {
                    Asset::Animated { appearance, table, .. }
                        if matches!(appearance.as_ref(), Appearance::Skinned { table: other, .. } if other == key) => Some(table.clone()),
                    _ => None,
                })
            });
            if let Some(shared) = shared {
                *table = shared;
            }
        }
    }
    asset
}
