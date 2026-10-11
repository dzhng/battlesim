//! The browser's presentation module (`specs/native-rendering-parity/slices/18b-presentation-api.md`):
//! one side's publications in, the game UI's observation out. It links the
//! publication reader, not the rules, and runs on the page's main thread.
use std::sync::Arc;

use serde::Serialize;
use sim::publication::reader::{Corpse, FallenBody, Fog, KnownProp, Observation, Reader, Shared};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Presentation {
    reader: Reader,
    latest: Option<Observation>,
    /// The static groups and the fog the page last took, kept to compare by pointer.
    taken: Option<(Shared<Corpse>, Shared<KnownProp>, Shared<FallenBody>)>,
    taken_fog: Option<Arc<Fog>>,
}

fn error(e: String) -> JsError {
    JsError::new(&e)
}

/// Appends `"name":<group>` to an object's JSON when `group` isn't the one
/// the page already holds.
fn splice<T: Serialize>(json: &mut String, name: &str, group: &Arc<T>, taken: Option<&Arc<T>>) {
    if taken.is_none_or(|t| !Arc::ptr_eq(t, group)) {
        json.pop();
        json.push_str(&format!(",\"{name}\":"));
        json.push_str(&serde_json::to_string(&**group).unwrap());
        json.push('}');
    }
}

#[wasm_bindgen]
impl Presentation {
    /// A presentation of the stream `layout` (the battle's `observation_layout`) names.
    #[wasm_bindgen(constructor)]
    pub fn new(layout: &str) -> Result<Presentation, JsError> {
        Ok(Presentation {
            reader: Reader::new(layout).map_err(error)?,
            latest: None,
            taken: None,
            taken_fog: None,
        })
    }

    /// Reads the next publication's raw words. False for a stale record (one
    /// from before [`invalidate`](Presentation::invalidate)); a malformed one throws.
    pub fn push_publication(&mut self, words: &[u32], _arrived_s: f64) -> Result<bool, JsError> {
        let Some(observation) = self.reader.decode(words).map_err(error)? else {
            return Ok(false);
        };
        self.latest = Some(observation);
        Ok(true)
    }

    /// The latest publication's observation as JSON, keyed as `ObservationView`,
    /// without `fog.bits` and `groundPatch.runs`. A static group is present
    /// only when it changed since the previous call.
    pub fn observation(&mut self) -> String {
        let latest = self.latest.as_ref().expect("a publication was pushed");
        let taken = self.taken.as_ref();
        let mut json = serde_json::to_string(latest).unwrap();
        splice(&mut json, "corpses", &latest.corpses, taken.map(|t| &t.0));
        splice(
            &mut json,
            "knownProps",
            &latest.known_props,
            taken.map(|t| &t.1),
        );
        splice(
            &mut json,
            "fallenBodies",
            &latest.fallen_bodies,
            taken.map(|t| &t.2),
        );
        self.taken = Some((
            latest.corpses.clone(),
            latest.known_props.clone(),
            latest.fallen_bodies.clone(),
        ));
        json
    }

    /// The latest fog bits when they changed since the previous call.
    pub fn fog_bits(&mut self) -> Option<Vec<u32>> {
        let fog = &self.latest.as_ref()?.fog;
        let changed = self.taken_fog.as_ref().is_none_or(|t| !Arc::ptr_eq(t, fog));
        self.taken_fog = Some(fog.clone());
        changed.then(|| fog.bits.clone())
    }

    /// The latest publication's ground runs.
    pub fn ground_runs(&self) -> Vec<f32> {
        self.latest
            .as_ref()
            .map_or_else(Vec::new, |o| o.ground_patch.runs.clone())
    }

    /// Records still in flight are stale; the next must open a new epoch,
    /// for `side` when named.
    pub fn invalidate(&mut self, side: Option<String>) -> Result<(), JsError> {
        self.reader.invalidate(side.as_deref()).map_err(error)?;
        (self.latest, self.taken, self.taken_fog) = (None, None, None);
        Ok(())
    }
}
