//! The wave field's frame, as the renderer runs it (`Presentation::waves`).
use crate::Presentation;
use presentation::records::WaveSplat;
use wasm_bindgen::prelude::*;

/// Floats before the splats: steps, epoch, shift x and y.
const HEAD: usize = 4;

#[wasm_bindgen]
impl Presentation {
    /// The wave field's frame for the last frame drawn: its steps, its
    /// epoch and how many texels it shifted, then each splat's `WaveSplat`
    /// floats; empty before a battle.
    pub fn waves(&mut self) -> Vec<f32> {
        let Some(w) = self.inner.waves() else {
            return Vec::new();
        };
        let floats = std::mem::size_of::<WaveSplat>() / 4;
        let mut out = Vec::with_capacity(HEAD + w.splats.len() * floats);
        let s = w.step;
        out.extend([
            s.steps as f32,
            s.epoch as f32,
            s.shift[0] as f32,
            s.shift[1] as f32,
        ]);
        for p in &w.splats {
            out.extend(p.splat);
            out.extend([p.step, p.kind, 0.0, 0.0]);
        }
        out
    }
}
