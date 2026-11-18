//! What sound hears of the canonical presentation: launches, cues and motion.
use super::{Frame, Presentation};
use crate::effects::launches::Launch;
use crate::effects::publication::EffectPublication;
use crate::records::{Emitter, NONE};
use crate::sound::frame::{Cue, Heard, Motion, Shout};
use crate::units::pose::PoseFrame;
use crate::world::UnitsSection;
use serde::Serialize;
use sim::publication::reader::Observation;
use std::sync::Arc;

impl Presentation {
    /// Each of a scripted run's publications' launches, JSON (`[[Shot]]`):
    /// what a lab's sound hears without a battle, by the effects' own
    /// derivation.
    pub fn launches_json(publications: &str) -> Result<String, String> {
        let publications: Vec<EffectPublication> =
            serde_json::from_str(publications).map_err(|e| e.to_string())?;
        let mut tracker = crate::effects::launches::LaunchTracker::default();
        let shots: Vec<Vec<Shot>> = publications
            .iter()
            .map(|p| tracker.note(p, false, None).iter().map(Shot::of).collect())
            .collect();
        serde_json::to_string(&shots).map_err(|e| e.to_string())
    }

    /// The last publication as sound hears it: what the effects took of it,
    /// the launches it shows, the side's hearing cues and the shouts it
    /// hears in the open; none before one.
    pub fn heard(&self) -> Option<Heard<'_>> {
        let (effects, launches) = self.loaded.as_ref()?.sound.as_ref()?;
        let latest = self.latest.as_ref()?;
        Some(Heard {
            effects,
            launches,
            audible: latest.audible.iter().map(Cue::from).collect(),
            shouts: latest.shouts.iter().map(Shout::from).collect(),
        })
    }

    /// What the last frame draws moving, as sound hears it: its emitters,
    /// each vehicle as its type's vehicle class.
    pub fn motion(&self) -> Motion {
        let types = self.loaded.as_ref().map(|l| &l.units.types);
        Motion::of(&self.frame.emitters, |class| {
            types?
                .values()
                .nth(class as usize)?
                .vehicle_class
                .as_deref()
        })
    }
}

pub(super) fn emitters(
    o: &Observation,
    poses: &PoseFrame,
    side: &str,
    units: &UnitsSection,
    f: &mut Frame,
) {
    let reversing = |unit: f64, own: bool| {
        if own {
            o.own.iter().any(|u| u.id == unit && u.reversing)
        } else {
            o.identified.iter().any(|e| e.id == unit && e.reversing)
        }
    };
    for v in &poses.vehicles {
        let own = &*v.side == side;
        let a = v.articulation;
        f.emitters.push(Emitter {
            unit: v.unit as u32,
            member: NONE,
            class: units
                .types
                .keys()
                .position(|k| **k == *v.kind)
                .map_or(NONE, |i| i as u32),
            flags: u32::from(!own) | if reversing(v.unit, own) { 2 } else { 0 },
            position: [
                v.position[0] as f32,
                v.position[1] as f32,
                v.position[2] as f32,
                0.0,
            ],
            motion: [
                a.travel_l as f32,
                a.travel_r as f32,
                a.turret_yaw as f32,
                if units
                    .types
                    .get(v.kind.as_ref())
                    .is_some_and(|t| t.has_rotor())
                {
                    a.rotor_speed as f32
                } else {
                    -1.0
                },
            ],
        });
    }
    for s in &poses.soldiers {
        f.emitters.push(Emitter {
            unit: s.unit as u32,
            member: s.soldier as u32,
            class: NONE,
            flags: u32::from(&*s.side != side),
            position: [
                s.position[0] as f32,
                s.position[1] as f32,
                s.position[2] as f32,
                0.0,
            ],
            motion: [0.0; 4],
        });
    }
}

/// A launch as sound reads it.
#[derive(Serialize)]
struct Shot {
    shooter: f64,
    mount: usize,
    soldier: Option<f64>,
    x: f64,
    y: f64,
    z: f64,
    kind: Arc<str>,
}

impl Shot {
    fn of(l: &Launch) -> Self {
        Self {
            shooter: l.shooter,
            mount: l.mount,
            soldier: l.soldier,
            x: l.at[0],
            y: l.at[1],
            z: l.at[2],
            kind: l.kind.clone(),
        }
    }
}
