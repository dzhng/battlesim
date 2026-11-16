//! An own unit's hold, decoded by the writer's field names: its host (a
//! building's footprint and seat total, or a carrier's id), phase and
//! progress.
use super::cells::{footprint, host};
use super::{at, Footprint, Hold, Host, Reader, Result};
use crate::publication::OWN_FIELDS;

impl Reader {
    /// The hold an own row names; none without a phase.
    pub(super) fn hold(&self, f: &[f32]) -> Result<Option<Hold>> {
        const FIELDS: &[&str] = &OWN_FIELDS;
        self.optional("holdPhases", v!(f, "holdPhase"))?
            .map(|phase| -> Result<Hold> {
                let (kind, id) = host!(self, f)?.ok_or("a hold names no host")?;
                let host = match (&*kind, footprint!(f)) {
                    ("building", Some(footprint)) => Host::Building {
                        id,
                        center: footprint.center,
                        half: footprint.half,
                        seats: v!(f, "hostSeats"),
                    },
                    ("building", None) => return Err("a held building has no footprint".into()),
                    _ => Host::Carrier { id },
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
