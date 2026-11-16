//! What a battle's units and effects draw from (`presentation::fielded`),
//! resolved for the page. The page reads the files and Rust decides: it
//! names the runtime contents it reads (`contents`), the page fetches and
//! inflates them as it does any bundle, and `resolve` builds `load_world`'s
//! `units` and `effects` sections and clip tables from their bytes.
use std::borrow::Cow;
use std::collections::HashMap;

use presentation::fielded::{self, Facts};
use serde_json::Value;
use wasm_bindgen::prelude::*;

use crate::error;

#[wasm_bindgen]
pub struct Fielding {
    presentation: Value,
    rules: Value,
    catalog: Value,
    runtime: Value,
    kinds: Vec<String>,
    factions: Option<[String; 2]>,
    facts: Facts,
}

impl Fielding {
    fn fielding(&self) -> fielded::Fielding<'_> {
        fielded::Fielding {
            presentation: &self.presentation,
            rules: &self.rules,
            catalog: &self.catalog,
            runtime: &self.runtime,
            kinds: &self.kinds,
            factions: self
                .factions
                .as_ref()
                .map(|[blue, red]| [blue.as_str(), red.as_str()]),
        }
    }
}

#[wasm_bindgen]
impl Fielding {
    /// A battle fielding `kinds` (a JSON list) of the session's resolved
    /// unit `catalog` (`Catalog::view`, JSON) under the battle's `rules`
    /// (its scenario's, JSON), each side fighting for the faction
    /// `factions` names (a JSON pair, blue then red; none: one look each),
    /// with visual settings (`fixtures/presentation.json`) and the runtime
    /// catalog `runtime` (`catalog.json`).
    #[wasm_bindgen(constructor)]
    pub fn new(
        presentation: &str,
        rules: &str,
        catalog: &str,
        runtime: &str,
        kinds: &str,
        factions: Option<String>,
    ) -> Result<Fielding, JsError> {
        let parse = |what: &str, text: &str| {
            serde_json::from_str::<Value>(text).map_err(|e| error(format!("{what}: {e}")))
        };
        Ok(Self {
            presentation: parse("presentation", presentation)?,
            rules: parse("rules", rules)?,
            catalog: parse("unit catalog", catalog)?,
            runtime: parse("runtime catalog", runtime)?,
            kinds: serde_json::from_str(kinds).map_err(|e| error(format!("kinds: {e}")))?,
            factions: factions
                .map(|f| serde_json::from_str(&f))
                .transpose()
                .map_err(|e| error(format!("factions: {e}")))?,
            facts: Facts::default(),
        })
    }

    pub fn set_kinds(&mut self, kinds: &str) -> Result<(), JsError> {
        self.kinds = serde_json::from_str(kinds).map_err(|e| error(format!("kinds: {e}")))?;
        Ok(())
    }

    /// The runtime contents `resolve` reads, by hash.
    pub fn contents(&self) -> Result<Vec<String>, JsError> {
        self.fielding().contents(&self.facts).map_err(error)
    }

    /// Content-sized facts missing from this fielding, including their source hashes.
    pub fn pending_contents(&self) -> Result<String, JsError> {
        serde_json::to_string(&self.fielding().pending(&self.facts).map_err(error)?).map_err(error)
    }

    /// Absorb one pending content; resolve afterwards reads no supplied bytes.
    pub fn absorb(&mut self, content: &str, bytes: &[u8], lengths: &[u32]) -> Result<(), JsError> {
        let content: fielded::Content = serde_json::from_str(content).map_err(error)?;
        let hashes = content.hashes();
        if hashes.len() != lengths.len() {
            return Err(error("content lengths differ from its sources"));
        }
        let mut at = 0;
        let mut source = HashMap::new();
        for (hash, &len) in hashes.iter().zip(lengths) {
            let end = at + len as usize;
            source.insert(
                hash.as_str(),
                bytes
                    .get(at..end)
                    .ok_or_else(|| error("content runs past its bytes"))?,
            );
            at = end;
        }
        fielded::Fielding {
            presentation: &self.presentation,
            rules: &self.rules,
            catalog: &self.catalog,
            runtime: &self.runtime,
            kinds: &self.kinds,
            factions: self
                .factions
                .as_ref()
                .map(|f| [f[0].as_str(), f[1].as_str()]),
        }
        .absorb(&mut self.facts, &content, &|hash| {
            source
                .get(hash)
                .map(|bytes| Cow::Borrowed(*bytes))
                .ok_or_else(|| format!("content {hash} was not supplied"))
        })
        .map_err(error)
    }

    /// Resolve into an immutable Rust result. Its descriptions and clip tables
    /// stay in this module; freeing the source owner does not invalidate it.
    pub fn resolve(&mut self, bytes: &[u8], lengths: &[u32]) -> Result<Fielded, JsError> {
        let hashes = self.contents()?;
        if hashes.len() != lengths.len() {
            return Err(error(format!(
                "{} contents read, {} named",
                lengths.len(),
                hashes.len()
            )));
        }
        let mut read = HashMap::new();
        let mut at = 0;
        for (hash, &len) in hashes.iter().zip(lengths) {
            let end = at + len as usize;
            read.insert(
                hash.as_str(),
                bytes
                    .get(at..end)
                    .ok_or_else(|| error("contents run past their bytes"))?,
            );
            at = end;
        }
        let fielded = fielded::Fielding {
            presentation: &self.presentation,
            rules: &self.rules,
            catalog: &self.catalog,
            runtime: &self.runtime,
            kinds: &self.kinds,
            factions: self
                .factions
                .as_ref()
                .map(|[blue, red]| [blue.as_str(), red.as_str()]),
        }
        .resolve(&mut self.facts, &|hash| {
            read.get(hash)
                .map(|b| Cow::Borrowed(*b))
                .ok_or(format!("content {hash} was not read"))
        })
        .map_err(error)?;
        Ok(Fielded {
            inner: std::sync::Arc::new(fielded),
        })
    }
}

/// A fielding snapshot: metadata queries cross the boundary, descriptions do not.
#[wasm_bindgen]
pub struct Fielded {
    pub(crate) inner: std::sync::Arc<fielded::Fielded>,
}

#[wasm_bindgen]
impl Fielded {
    /// Another handle to this snapshot, consumed when loading a world. Descriptions
    /// remain shared; a world reload never consumes its caller's retained result.
    pub fn share(&self) -> Fielded {
        Fielded {
            inner: self.inner.clone(),
        }
    }

    /// Whether this kind places a hull rather than squad members in a showcase.
    pub fn hull(&self, kind: &str) -> Result<bool, JsError> {
        self.inner
            .units
            .types
            .get(kind)
            .map(|t| t.hull)
            .ok_or_else(|| error(format!("no unit type {kind}")))
    }

    /// The description work this resolve did, for the admission diagnostics.
    pub fn work(&self) -> String {
        serde_json::to_string(&self.inner.work).expect("fielding work serializes")
    }
}
