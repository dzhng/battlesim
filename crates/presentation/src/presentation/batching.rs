//! Posed models into contiguous instance runs and their draw batches.
use super::{pack_colour, Frame};
use crate::records::{Batch, Instance, NONE};
use crate::view::batch_caster;
use crate::world::Vec3;

/// One model this frame may draw, before batching.
pub(super) struct Model {
    pub(super) row: u32,
    pub(super) tier: u32,
    pub(super) pass: u32,
    pub(super) at: Vec3,
    pub(super) yaw: f64,
    pub(super) tint: [f32; 3],
    pub(super) highlight: bool,
    pub(super) scale: [f32; 3],
    pub(super) colour: [f64; 4],
    /// First palette joint, or 0 (identity).
    pub(super) palette: u32,
    pub(super) scroll: [f64; 2],
    /// How fog takes it (`Batch.fog`).
    pub(super) fog: u32,
    pub(super) shadow: bool,
}

/// The models as batches: one per drawable, tier and pass, in that order.
/// A model in view casts a tier coarser; a card, a ghost and a high-air
/// model cast nothing (`batch_caster`). A live stand-in can opt out of
/// casting while authored models keep this policy.
pub(super) fn write(models: &mut [Model], f: &mut Frame) {
    let run_of = |m: &Model| (m.row, m.tier, m.pass, m.shadow);
    models.sort_by_key(run_of);
    for run in models.chunk_by(|a, b| run_of(a) == run_of(b)) {
        let m = &run[0];
        let caster = if m.shadow {
            batch_caster(m.tier, m.pass)
        } else {
            NONE
        };
        let cascade_mask = if caster == NONE {
            0
        } else {
            crate::shadow::ALL_CASCADES
        };
        f.batches.push(Batch {
            key: crate::view::batch_key(m.row, m.tier, m.pass, cascade_mask),
            appearance: m.row,
            tier: m.tier,
            pass: m.pass,
            caster,
            cascade_mask,
            fog: m.fog,
            version: f.index,
            first: f.instances.len() as u32,
            count: run.len() as u32,
            pad: [0; 2],
        });
        let fract = |v: f64| (v - v.floor()) as f32;
        f.instances.extend(run.iter().map(|m| Instance {
            placement: [m.at[0] as f32, m.at[1] as f32, m.at[2] as f32, m.yaw as f32],
            data: [
                m.palette as f32,
                fract(m.scroll[0]),
                fract(m.scroll[1]),
                pack_colour(m.colour),
            ],
            tint: [
                m.tint[0],
                m.tint[1],
                m.tint[2],
                if m.highlight { -2.0 } else { -1.0 },
            ],
            scale: [m.scale[0], m.scale[1], m.scale[2], m.colour[3] as f32],
        }));
    }
}
