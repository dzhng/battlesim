//! What a battle's units and effects draw from (`presentation::fielded`),
//! resolved for the page. The page reads the files and Rust decides: it
//! names the runtime contents it reads (`contents`), the page fetches and
//! inflates them as it does any bundle, and `resolve` builds `load_world`'s
//! `units` and `effects` sections and clip tables from their bytes.
use std::collections::HashMap;

use presentation::fielded::{self, Sections};
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

use crate::error;

#[wasm_bindgen]
pub struct Fielding {
    presentation: Value,
    weapons: Value,
    rules: Value,
    catalog: Value,
    runtime: Value,
    kinds: Vec<String>,
    factions: Option<[String; 2]>,
    tables: Vec<Vec<f32>>,
}

impl Fielding {
    fn fielding(&self) -> fielded::Fielding<'_> {
        fielded::Fielding {
            presentation: &self.presentation,
            weapons: &self.weapons,
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
    /// with visual settings (`fixtures/presentation.json`), authored weapon
    /// inheritance (`weapons`), and the runtime
    /// catalog `runtime` (`catalog.json`).
    #[wasm_bindgen(constructor)]
    pub fn new(
        presentation: &str,
        weapons: &str,
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
            weapons: parse("weapons", weapons)?,
            rules: parse("rules", rules)?,
            catalog: parse("unit catalog", catalog)?,
            runtime: parse("runtime catalog", runtime)?,
            kinds: serde_json::from_str(kinds).map_err(|e| error(format!("kinds: {e}")))?,
            factions: factions
                .map(|f| serde_json::from_str(&f))
                .transpose()
                .map_err(|e| error(format!("factions: {e}")))?,
            tables: Vec::new(),
        })
    }

    /// The runtime contents `resolve` reads, by hash.
    pub fn contents(&self) -> Result<Vec<String>, JsError> {
        self.fielding().contents().map_err(error)
    }

    /// Resolves the battle from each content `contents` named, inflated, in
    /// its order: `bytes` holds them end to end, `lengths` their lengths.
    /// Returns `{units, effects, tables}` JSON, `tables` naming each clip
    /// table `table` hands over by index.
    pub fn resolve(&mut self, bytes: &[u8], lengths: &[u32]) -> Result<String, JsError> {
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
        let Sections {
            units,
            effects,
            tables,
        } = self
            .fielding()
            .resolve(&|hash| {
                read.get(hash)
                    .map(|b| b.to_vec())
                    .ok_or(format!("content {hash} was not read"))
            })
            .map_err(error)?;
        let names: Vec<&String> = tables.keys().collect();
        let out = json!({ "units": units, "effects": effects, "tables": names }).to_string();
        self.tables = tables.into_values().collect();
        Ok(out)
    }

    /// Clip table `index` of the last `resolve`'s `tables`.
    pub fn table(&self, index: usize) -> Vec<f32> {
        self.tables[index].clone()
    }
}
