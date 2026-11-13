//! A vehicle standing idle on the ground with no battle around it: the loops
//! a stationary battle vehicle starts, its engine at idle and each sound
//! replaced as the battle replaces it, for a page that shows one unit at
//! rest. An aircraft there is parked, shut down, and plays nothing.
use serde::Serialize;

use super::frame::vehicle_loops;
use super::setup::admit;
use super::tables::{pick, SoundTables};
use crate::fielded;

/// One loop an idle vehicle plays, as the battle would start it beside the
/// listener.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct IdleLoop {
    pub slot: &'static str,
    pub sound: String,
    pub gain: f64,
    pub rate: f64,
}

/// The loops a vehicle of `vehicle_class` plays standing still on the
/// ground; none for a squad, which has no class, and none for a type that
/// `flies`: on the ground it is parked, its rotors still and engines off.
pub fn idle_loops(tables: &SoundTables, vehicle_class: Option<&str>, flies: bool) -> Vec<IdleLoop> {
    let Some(class) = vehicle_class.filter(|_| !flies) else {
        return Vec::new();
    };
    vehicle_loops(pick(&tables.audio.vehicles, class), 0.0, 0.0, false)
        .into_iter()
        .map(|l| IdleLoop {
            slot: l.slot,
            sound: tables.effect(l.sound).to_owned(),
            gain: l.gain,
            rate: l.rate,
        })
        .collect()
}

/// [`idle_loops`] for unit type `kind`, from a battle's sound setup JSON (the
/// one [`super::frame::Sound::from_json`] reads), its class and whether it
/// flies from the setup's unit catalog.
pub fn idle_loops_of(setup: &str, kind: &str) -> Result<Vec<IdleLoop>, String> {
    let admitted = admit(setup)?;
    let unit = admitted.unit_catalog["units"]
        .as_array()
        .and_then(|units| units.iter().find(|u| u["id"] == kind))
        .ok_or_else(|| format!("sound: {kind} is not in the unit catalog"))?;
    Ok(idle_loops(
        &admitted.tables,
        unit["vehicle_class"].as_str(),
        fielded::airborne(unit),
    ))
}
