//! A vehicle's pose each frame: its guns on their published bearings and
//! elevations, its gun's recoil, its running gear's travel or its rotors'
//! turn, and its whole-model motion (an aircraft's lean, a downed one's
//! fall, a hull rocking on its springs).
use super::attitude::{aircraft_attitude, tilt, Flight};
use super::*;

/// Its landing gear stowed (1) or down (0) for its flight state: a live or
/// falling fixed-wing aircraft flies gear up; everything else stands on it.
fn gear_up_for(ty: &UnitType) -> f64 {
    if ty.airborne && ty.flight == Some(FlightModel::FixedWing) {
        1.0
    } else {
        0.0
    }
}

pub(super) struct VehicleState {
    pose: VehiclePose,
    /// Each mount's shot counter as last seen, and when the gun's last shot was.
    shots: Vec<f64>,
    fired_at: f64,
    /// Its hull on its springs (`kick_hull`).
    kick: [f64; 4],
    pub(super) seen: u64,
    flight: Option<Flight>,
}

impl PoseDriver {
    pub(super) fn vehicle(
        &mut self,
        unit: &FeedUnit,
        ty: &UnitType,
        time: f64,
        dt: f64,
        generation: u64,
    ) {
        let role = |name: &str| ty.roles.iter().position(|r| r == name);
        let (gun, hmg) = (role("gun"), role("hmg"));
        let gun_mount = gun.and_then(|g| unit.mounts.get(g));
        let hmg_mount = hmg.and_then(|h| unit.mounts.get(h));
        // The published elevations; each model's rig stops its guns at its
        // own pitch limits (`articulate`).
        let gun_target = gun_mount.map_or(0.0, |m| m.elevation);
        let hmg_target = hmg_mount.map_or(0.0, |m| m.elevation);
        let state = self
            .vehicles
            .entry(side_key(unit.id, &*unit.side == "blue").to_bits())
            .or_insert_with(|| {
                // First seen: posed as published, with no shot to recoil from.
                VehicleState {
                    shots: unit.mounts.iter().map(|m| m.shots).collect(),
                    fired_at: f64::NEG_INFINITY,
                    kick: [0.0; 4],
                    pose: VehiclePose {
                        unit: unit.id,
                        kind: unit.kind.clone(),
                        side: unit.side.clone(),
                        position: unit.position,
                        yaw: unit.yaw,
                        airborne: ty.airborne,
                        downed: false,
                        // Its rotors as a camera's shutter sees them turn.
                        articulation: Articulation {
                            gun_pitch: gun_target,
                            hmg_pitch: hmg_target,
                            rotor_blur: if ty.airborne { 1.0 } else { 0.0 },
                            // A type flies gear up or stands on it for life.
                            gear_up: gear_up_for(ty),
                            ..Articulation::default()
                        },
                        motion: None,
                    },
                    seen: generation,
                    flight: None,
                }
            });
        state.seen = generation;
        let feel = &self.feel;
        let pose = &mut state.pose;
        let a = &mut pose.articulation;
        if pose.airborne {
            // In the air its rotors turn and nothing rolls.
            a.rotor += feel.rotor.tip_mps * dt;
        } else {
            // Ground covered along the hull, plus each side's share of the turn.
            let forward = (unit.position[0] - pose.position[0]) * libm::cos(unit.yaw)
                + (unit.position[1] - pose.position[1]) * libm::sin(unit.yaw);
            let turned = delta_angle(pose.yaw, unit.yaw);
            let half = ty.draws[unit.side.as_ref()].half_track;
            a.travel_l += forward - turned * half;
            a.travel_r += forward + turned * half;
        }
        let flying = if pose.airborne && unit.attitude.is_none() {
            let f = state.flight.get_or_insert_with(Flight::default);
            aircraft_attitude(f, &feel.attitude, (pose.position, pose.yaw), unit, ty, dt)
        } else {
            None
        };
        pose.position = unit.position;
        pose.yaw = unit.yaw;
        pose.downed = unit.attitude.is_some();
        let attitude = unit.attitude.or(flying);
        let a = &mut pose.articulation;
        a.deploy = unit.deployment.unwrap_or(0.0);

        // The turret on the cannon's bearing, the HMG relative to what it
        // rides (a mount, else the hull); elevations eased (they change
        // only on a shot); a new cannon round recoils the gun.
        let carrier = hmg
            .and_then(|h| ty.mounts.get(h)?.on)
            .and_then(|on| unit.mounts.get(on));
        let carrier_bearing = carrier.map_or(unit.yaw, |m| m.bearing);
        a.turret_yaw = gun_mount.map_or(0.0, |m| delta_angle(unit.yaw, m.bearing));
        a.gun_pitch = approach(a.gun_pitch, gun_target, feel.mount.gun_elevation_rad_s * dt);
        a.hmg_yaw = hmg_mount.map_or(0.0, |m| delta_angle(carrier_bearing, m.bearing));
        a.hmg_pitch = approach(a.hmg_pitch, hmg_target, feel.mount.hmg_elevation_rad_s * dt);
        let last = |i: usize| state.shots.get(i).copied();
        if gun_mount.is_some_and(|m| m.shots > gun.and_then(last).unwrap_or(m.shots)) {
            state.fired_at = time;
        }
        a.recoil = recoil_at(time - state.fired_at, &feel.mount);

        // An aircraft leans; a hull on the ground rocks on its springs.
        pose.motion = match (attitude, &feel.hull_kick) {
            (Some((pitch, roll)), _) => Some(tilt(pitch, roll)),
            (None, Some(kick)) if !pose.airborne => {
                kick_hull(&mut state.kick, &state.shots, unit, ty, kick, dt)
            }
            _ => None,
        };
        state.shots.clear();
        state.shots.extend(unit.mounts.iter().map(|m| m.shots));
        self.out.vehicles.push(pose.clone());
    }
}
