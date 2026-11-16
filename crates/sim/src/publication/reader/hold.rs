//! An own unit's hold, decoded by the writer's field names: its host (a
//! building's footprint and seat total, or a carrier's id), phase and
//! progress.
use super::cells::host;
use super::{at, Hold, Host, Reader, Result};
use crate::publication::OWN_FIELDS;

impl Reader {
    /// The hold an own row names; none without a phase.
    pub(super) fn hold(&self, f: &[f32]) -> Result<Option<Hold>> {
        const FIELDS: &[&str] = &OWN_FIELDS;
        self.optional("holdPhases", v!(f, "holdPhase"))?
            .map(|phase| -> Result<Hold> {
                let (kind, id) = host!(self, f)?.ok_or("a hold names no host")?;
                let host = if &*kind == "building" {
                    Host::Building {
                        id,
                        center: [v!(f, "hostX"), v!(f, "hostY")],
                        half: [v!(f, "hostHalfX"), v!(f, "hostHalfY")],
                        seats: v!(f, "hostSeats"),
                    }
                } else {
                    Host::Carrier { id }
                };
                Ok(Hold {
                    host,
                    phase,
                    progress: v!(f, "holdProgress"),
                })
            })
            .transpose()
    }
}
