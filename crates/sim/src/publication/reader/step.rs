//! An own unit's order step, decoded by the writer's field names: its kind,
//! its point (none when the side knows none), the target an attack names and
//! the host a step enters.
use super::cells::{host, target};
use super::{at, OrderStep, Reader, Result, StepHost};
use crate::publication::STEP_FIELDS;

impl Reader {
    pub(super) fn step(&self, p: &[f32; STEP_FIELDS.len()]) -> Result<OrderStep> {
        const FIELDS: &[&str] = &STEP_FIELDS;
        let host = host!(self, p)?.map(|(kind, id)| StepHost { kind, id });
        Ok(OrderStep {
            kind: self.name("stepKinds", v!(p, "kind"))?,
            point: (!v!(p, "x").is_nan()).then(|| [v!(p, "x"), v!(p, "y")]),
            attack_move: v!(p, "attackMove") == 1.0,
            target: target!(self, p)?,
            host,
        })
    }
}
