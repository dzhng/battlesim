//! Fog of war's sight on the page's presentation (`presentation::fog::sight`):
//! the page says whether fog is drawn and with what sensor rules (and a
//! lab's stand-ins); the renderer uploads each frame's `FogUpload` as it is.
use crate::{error, Presentation};
use presentation::presentation::FogSetup;
use presentation::records::Camera;
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

    /// One frame's fog for the frame's `camera` record words (none: a
    /// probe's) over a `width`×`height` target, with `probes` points to
    /// probe; none without fog.
    pub fn prepare_fog(
        &mut self,
        camera: Option<Vec<f32>>,
        width: u32,
        height: u32,
        probes: u32,
    ) -> Result<Option<FogUpload>, JsError> {
        let camera = camera.map(|c| {
            bytemuck::pod_read_unaligned::<Camera>(
                &bytemuck::cast_slice(&c)[..std::mem::size_of::<Camera>()],
            )
        });
        let up = self
            .inner
            .prepare_fog(camera.as_ref(), width, height, probes)
            .map_err(error)?;
        Ok(up.map(FogUpload))
    }

    /// The renderer holds no fog: the next upload sends it all.
    pub fn resend_fog(&mut self) {
        self.inner.resend_fog();
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

/// One frame's fog uploads and passes (`presentation::fog::upload::FogUpload`).
#[wasm_bindgen]
pub struct FogUpload(presentation::fog::upload::FogUpload);

#[wasm_bindgen]
impl FogUpload {
    /// The canonical fog record as an owned JavaScript value for a renderer
    /// running in a separate WASM memory. Optional tables preserve absent
    /// deltas separately from explicitly empty tables.
    pub fn snapshot(&self) -> Result<JsValue, JsError> {
        serde_wasm_bindgen::to_value(&self.0).map_err(|error| JsError::new(&error.to_string()))
    }

    /// `FogParams`' words.
    pub fn params(&self) -> Vec<u32> {
        bytemuck::cast_slice(bytemuck::bytes_of(&self.0.params)).to_vec()
    }
    /// Whether fog is drawn: off, nothing runs.
    pub fn active(&self) -> bool {
        self.0.active
    }
    /// `FogEye` records, words.
    pub fn eyes(&self) -> Vec<u32> {
        bytemuck::cast_slice(&self.0.eyes).to_vec()
    }
    /// The maps' and terrain maps' sizes, words.
    pub fn map_words(&self) -> Vec<u32> {
        self.0.map_words.to_vec()
    }
    /// The rebuilt eyes' records; empty when none rebuild.
    pub fn rebuild(&self) -> Vec<u32> {
        self.0.rebuild.clone()
    }
    /// The terrain march's and merge's workgroups.
    pub fn rebuild_dispatch(&self) -> Vec<u32> {
        self.0.rebuild_dispatch.to_vec()
    }
    /// The foliage cells, when they changed.
    pub fn foliage(&self) -> Option<Vec<f32>> {
        self.0.foliage.clone()
    }
    /// `FogBox` records, words, when the side's knowledge changed.
    pub fn occluders(&self) -> Option<Vec<u32>> {
        self.0
            .occluders
            .as_ref()
            .map(|o| bytemuck::cast_slice(o).to_vec())
    }
    /// The `wholes` texture's words, when the knowledge changed.
    pub fn wholes(&self) -> Option<Vec<u32>> {
        self.0.wholes.clone()
    }
    /// The structures pass: count, workgroups x and y, flags row, flag rows;
    /// empty when it doesn't run.
    pub fn whole(&self) -> Vec<u32> {
        self.0.whole.as_ref().map_or_else(Vec::new, |w| {
            vec![
                w.count,
                w.dispatch[0],
                w.dispatch[1],
                w.flags_row,
                w.flag_rows,
            ]
        })
    }
    /// Its rows' words.
    pub fn whole_items(&self) -> Vec<u32> {
        self.0
            .whole
            .as_ref()
            .map_or_else(Vec::new, |w| w.items.clone())
    }
    /// Its uniform: `(whole_step_m, count, 0, 0)`.
    pub fn whole_uniform(&self) -> Vec<f32> {
        self.0
            .whole
            .as_ref()
            .map_or_else(Vec::new, |w| w.uniform.to_vec())
    }
    /// The cull: tiles across, down, and words per tile list.
    pub fn tiles(&self) -> Vec<u32> {
        self.0.tiles.to_vec()
    }
}
