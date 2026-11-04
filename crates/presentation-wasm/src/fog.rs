//! Fog of war's sight on the page's presentation (`presentation::fog::sight`):
//! the page says whether fog is drawn and with what sensor rules (and a
//! lab's stand-ins). The shared runtime consumes incremental fog work.
use crate::{error, Presentation};
use presentation::presentation::FogSetup;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Presentation {
    /// Draws the side's fog of war (`setup` JSON: `sensors`, the rules'
    /// sensor numbers; a lab's stand-in `eyes`, or the `units` it looks
    /// through), or none; `foliage` is the side's foliage when it changed
    /// (the simulation's export less the ground it has seen cleared).
    pub fn set_fog(
        &mut self,
        setup: Option<String>,
        foliage: Option<Vec<f32>>,
    ) -> Result<(), JsError> {
        let setup = setup
            .map(|s| serde_json::from_str::<FogSetup>(&s))
            .transpose()
            .map_err(|e| JsError::new(&format!("fog: {e}")))?
            .map(|s| FogSetup { foliage, ..s });
        self.inner.set_fog(setup).map_err(error)
    }

    /// Every eye's map rebuilds on the next frame (a cost probe).
    pub fn rebuild_fog(&mut self) {
        self.inner.rebuild_fog();
    }

    /// The fog's `FogStats` as JSON; none without fog.
    pub fn fog_stats(&self) -> Option<String> {
        self.inner
            .fog_stats()
            .map(|s| serde_json::to_string(&s).unwrap())
    }

    /// The eyes fog sees from, JSON (`key`, `position`, `forward`, `shape`, `range`).
    pub fn fog_eyes(&self) -> String {
        serde_json::to_string(&self.inner.fog_eyes()).unwrap()
    }

    /// The occluders the side's fog knows, seven numbers each (x, y, yaw,
    /// hx, hy, base, top), in the order of the structures' seen flags.
    pub fn fog_occluders(&self) -> Vec<f64> {
        self.inner
            .fog_occluders()
            .iter()
            .flat_map(|b| [b.x, b.y, b.yaw, b.hx, b.hy, b.base, b.top])
            .collect()
    }
}
