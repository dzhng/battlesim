//! A vehicle's loops: engine, running gear, turret and back-up beeper by load.
use serde::Serialize;

use super::lerp;
use crate::sound::tables::{profile, VehicleSound};

/// One of a vehicle's sounding loops.
#[derive(Serialize)]
pub struct VehicleLoop<'a> {
    pub slot: &'static str,
    pub distance: &'a str,
    pub sound: &'a str,
    pub gain: f64,
    pub rate: f64,
}

/// The loops a vehicle of `row` sounds: its engine from idle to full `load`
/// (a share of `full_speed_mps`), running gear rising with it, a turret's
/// whine by `traverse` (a share of `full_traverse_rps`) and, `reversing`, the
/// back-up beeper, steady whatever the speed, as an alarm is. A layer below
/// audibility is left out.
pub fn vehicle_loops(
    row: &VehicleSound,
    load: f64,
    traverse: f64,
    reversing: bool,
) -> Vec<VehicleLoop<'_>> {
    let mut loops = vec![VehicleLoop {
        slot: "engine",
        distance: &row.engine_distance,
        sound: &row.engine,
        gain: lerp(row.idle_gain, row.load_gain, load),
        rate: lerp(row.idle_rate, row.load_rate, load),
    }];
    if let Some(running) = row.running.as_deref().filter(|_| load > 0.02) {
        loops.push(VehicleLoop {
            slot: "running",
            distance: profile::SMALL_DETAILS,
            sound: running,
            gain: row.running_gain * load.sqrt(),
            rate: 0.5 + load,
        });
    }
    if let Some(turret) = row.turret.as_deref().filter(|_| traverse > 0.05) {
        loops.push(VehicleLoop {
            slot: "turret",
            distance: profile::SMALL_DETAILS,
            sound: turret,
            gain: row.turret_gain * traverse,
            rate: 0.9 + 0.2 * traverse,
        });
    }
    if let Some(reverse) = row.reverse.as_deref().filter(|_| reversing && load > 0.02) {
        loops.push(VehicleLoop {
            slot: "reverse",
            distance: profile::SMALL_DETAILS,
            sound: reverse,
            gain: row.reverse_gain,
            rate: 1.0,
        });
    }
    loops
}
