//! Immutable content facts and the names routing to them in one catalog.
use super::{Asset, ContentId};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Default)]
pub(in crate::presentation) struct Facts {
    names: BTreeMap<String, ContentId>,
    contents: BTreeMap<ContentId, Arc<Asset>>,
}

impl Facts {
    pub(in crate::presentation) fn installed(assets: BTreeMap<String, (ContentId, Asset)>) -> Self {
        let mut facts = Self::default();
        for (name, (content, asset)) in assets {
            facts
                .contents
                .entry(content.clone())
                .or_insert_with(|| Arc::new(asset));
            facts.names.insert(name, content);
        }
        facts
    }

    pub(super) fn stage(
        &self,
        wanted: BTreeMap<String, ContentId>,
        mut supplied: BTreeMap<ContentId, Asset>,
    ) -> Result<Self, String> {
        let mut contents = BTreeMap::new();
        for (name, content) in &wanted {
            if contents.contains_key(content) {
                continue;
            }
            let asset = supplied
                .remove(content)
                .map(Arc::new)
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
