//! A falling airframe's row, decoded by the writer's field names.
use super::{at, present, Crash, Reader, Result, Row, Strike};
use crate::publication::CRASH_FIELDS;

impl Reader {
    pub(super) fn crash(&self, r: &Row) -> Result<Crash> {
        const FIELDS: &[&str] = &CRASH_FIELDS;
        let f = r.fields;
        Ok(Crash {
            id: v!(f, "id"),
            own: v!(f, "own") == 1.0,
            kind: self.name("unitKinds", v!(f, "kind"))?,
            position: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
            yaw: v!(f, "yaw"),
            pitch: v!(f, "pitch"),
            roll: v!(f, "roll"),
            strike: present(v!(f, "strikeMps")).map(|speed_mps| Strike {
                point: [v!(f, "strikeX"), v!(f, "strikeY"), v!(f, "strikeZ")],
                normal: [v!(f, "strikeNx"), v!(f, "strikeNy"), v!(f, "strikeNz")],
                speed_mps,
            }),
        })
    }
}
