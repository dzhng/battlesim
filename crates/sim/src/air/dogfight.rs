//! Fixed-wing close air combat. This module owns every write to a jet's
//! `dogfight_target` and `come_round`; movement only observes the resulting
//! link.

use contract::ids::UnitId;
use contract::random::{mix, Rng};
use contract::scenario::FixedWingFlight;

use crate::knowledge::Track;
use crate::math::{v2, V2, V3};
use crate::units::{AirState, Motion, Unit};

use super::fixed_wing::{FixedWingState, NoseAt};
use super::steer::Intent;

/// Whether two aircraft are past the break range in 3D: beyond it a fight
/// goes on only while they close or come round for each other (`broken`).
pub fn breaks(distance_3d: f64, air: &FixedWingFlight) -> bool {
    distance_3d > air.dogfight_break_m
}

/// Whether `unit`'s dogfight against `target` is over: either can no longer
/// fight, or they are past the break range and not closing on each other.
/// While `unit` comes round for another pass at a target that fights it
/// back, the fight holds out to the farthest a come-round carries the pair
/// past the break (`come_round_spread`, at the faster jet's speed), so a
/// wide turn back is not a fight given up; a target that has left the fight
/// to fly elsewhere is let go at the break.
pub(crate) fn broken(unit: &Unit, target: &Unit, air: &FixedWingFlight) -> bool {
    if !can_dogfight(unit, target) {
        return true;
    }
    let distance = crate::math::body_distance(unit.position, target.position);
    if !breaks(distance, air) {
        return false;
    }
    let velocity = |u: &Unit| u.air.as_ref().map_or(V3::default(), |a| a.velocity);
    let closing = (velocity(target) - velocity(unit)).dot(target.position - unit.position) < 0.0;
    let coming_round = unit
        .air
        .as_ref()
        .and_then(|a| a.come_round)
        .is_some_and(|(foe, _)| foe == target.id);
    let fights_back = target
        .air
        .as_ref()
        .is_some_and(|a| a.dogfight_target == Some(unit.id));
    let Motion::Air(flight) = &unit.motion else {
        unreachable!("a jet that can dogfight flies")
    };
    let speed = velocity(unit)
        .xy()
        .length()
        .max(velocity(target).xy().length())
        .max(flight.cruise_mps);
    let turning_room =
        air.dogfight_break_m + come_round_spread(speed, flight.turn_rad_s, air.roll_in_s);
    !closing && !(coming_round && fights_back && distance < turning_room)
}

/// How much further apart two jets drift than where they began to come
/// round for each other: each flies on while it rolls in, then swings a
/// turn's radius further out before its turn brings it back.
fn come_round_spread(speed_mps: f64, turn_rad_s: f64, roll_in_s: f64) -> f64 {
    2.0 * speed_mps / turn_rad_s.max(f64::MIN_POSITIVE) + 2.0 * speed_mps * roll_in_s
}

/// Whether `pursuer` may fly a dogfight against `target` (D29c): both fly
/// on wings, and the target is alive in the pursuer's own layer. Read from
/// the layers rather than the current heights; against a rotor aircraft in
/// any layer, anything in another layer, or the ground, a winged aircraft
/// attacks in passes, and a rotor aircraft never dogfights.
pub(crate) fn can_dogfight(pursuer: &Unit, target: &Unit) -> bool {
    pursuer.fixed_wing()
        && target.fixed_wing()
        && target.alive()
        && target.layer() == pursuer.layer()
}

/// Link a firing winged aircraft to its winged target and let the target
/// answer it if unengaged.
pub(crate) fn on_fire(units: &mut [Unit], shooter: UnitId, target: UnitId) {
    let (Some(s), Some(t)) = (units.get(shooter.0 as usize), units.get(target.0 as usize)) else {
        return;
    };
    if shooter == target || !s.alive() || !can_dogfight(s, t) {
        return;
    }
    let answers = can_dogfight(t, s);
    units[shooter.0 as usize]
        .air
        .as_mut()
        .expect("an aircraft's flight state")
        .dogfight_target = Some(target);
    if answers
        && units[target.0 as usize]
            .air
            .as_ref()
            .is_some_and(|air| air.dogfight_target.is_none())
    {
        units[target.0 as usize]
            .air
            .as_mut()
            .expect("an aircraft's flight state")
            .dogfight_target = Some(shooter);
    }
}

/// Install a target an explicit air order selected; its caller has judged
/// the pair (`can_dogfight`).
pub(crate) fn set_target(unit: &mut Unit, target: UnitId) {
    if unit.alive() && unit.fixed_wing() {
        unit.air
            .as_mut()
            .expect("an aircraft's flight state")
            .dogfight_target = Some(target);
    }
}

/// Forget the side a jet came round toward, on a tick it flies no dogfight,
/// so a fight it takes up again chooses afresh.
pub(crate) fn not_flown(unit: &mut Unit) {
    if let Some(air) = unit.air.as_mut() {
        air.come_round = None;
    }
}

/// Clear a link through the single dogfight owner.
pub(crate) fn clear(unit: &mut Unit) {
    if let Some(air) = unit.air.as_mut() {
        air.dogfight_target = None;
        air.come_round = None;
    }
}

/// A jet's dogfight intent this tick against `target`, as its side tracks
/// it: none, and no come-round side kept, once the side no longer does. The
/// side a jet swings out to as it comes round is kept per opponent, so a jet
/// that changes opponents chooses afresh.
pub(crate) fn link_intent(
    air: &mut AirState,
    target: UnitId,
    track: Option<&Track>,
    own: &FixedWingState,
    seed: u64,
    flight: &FixedWingFlight,
    geometry: &DogfightGeometry,
) -> Option<Intent> {
    let Some(track) = track else {
        air.come_round = None;
        return None;
    };
    let mut side = air
        .come_round
        .filter(|(id, _)| *id == target)
        .map(|(_, side)| side);
    let intent = intent(own, track, seed, flight, geometry, &mut side);
    air.come_round = side.map(|side| (target, side));
    Some(intent)
}

/// Build one tick of a linked jet's dogfight: a run of head-on passes, as
/// a film shows one. While the target lies ahead it flies at it nose-on, so
/// its guns and missiles bear on the way in; closing on it, it aims
/// `separation_m` to the target's right from as far out as it needs to roll
/// in and slide that far at the speed they close (an overtaking chaser
/// too), so the two flash past beside each other rather than through. Past
/// it, while both fly apart, it extends until they are as far apart as its
/// weapons reach (less where coming round would carry the other out of its
/// sight); then it comes round toward one side of the line between them
/// that both jets pick alike (the side toward the middle of the field from
/// the pair's midpoint), held until it closes on the other again, so one
/// turns left and the other right and they meet head-on again instead of
/// circling apart, and the fight keeps to the middle. A target that runs
/// gets a nose-on chase from behind; one on its tail makes it come round at
/// once.
///
/// The seed is already separated by jet id by the caller, so the sampled
/// height is stable for that jet across ticks while remaining battle seeded.
pub fn intent(
    own: &FixedWingState,
    target_track: &Track,
    seed: u64,
    flight: &FixedWingFlight,
    geometry: &DogfightGeometry,
    come_round: &mut Option<V2>,
) -> Intent {
    let DogfightGeometry {
        separation_m,
        turn_rad_s,
        centre,
        reach_m,
        seen_behind_m,
        seen_abeam_m,
    } = *geometry;
    let mut rng = Rng::new(mix(seed));
    let band = flight.dogfight_band_agl_m;
    let agl = band[0] + rng.unit() * (band[1] - band[0]);
    let own_forward = v2(libm::cos(own.heading), libm::sin(own.heading));
    let target_forward = v2(libm::cos(target_track.yaw), libm::sin(target_track.yaw));
    let target = target_track.position.xy();
    let offset = target - own.pos.xy();
    let distance = offset.length();
    let to_target = if distance > f64::EPSILON {
        offset * (1.0 / distance)
    } else {
        own_forward
    };
    let right = v2(to_target.y, -to_target.x);
    let turn_rad_s = turn_rad_s.max(f64::MIN_POSITIVE);
    let closing_on = (target_track.velocity - own_forward * own.speed_mps).dot(offset) < 0.0;
    if own_forward.dot(to_target) > 0.0 {
        // The come-round is over once it closes on the other jet again.
        if closing_on {
            *come_round = None;
        }
        // It slides aside from as far out as rolling in and slipping one
        // separation sideways takes at the speed they close: early when
        // head-on, late when overtaking, never in an even chase.
        let closing = (own_forward * own.speed_mps - target_track.velocity).dot(to_target);
        let slide_s =
            flight.roll_in_s + (2.0 * separation_m / (own.speed_mps.max(1.0) * turn_rad_s)).sqrt();
        let sidestep = distance < closing.max(0.0) * slide_s;
        let aside = if sidestep { separation_m } else { 0.0 };
        // Flying at it, it points its nose at the other, so its guns bear
        // once it is in reach, never below the band's floor.
        let nose_at = Some(NoseAt {
            point: target_track.position,
            lowest_agl_m: band[0],
        });
        return Intent::Pursue {
            point: target + right * aside,
            agl,
            nose_at,
        };
    }
    // It extends until they are as far apart as its weapons reach, so all
    // of them bear as it comes back nose-on; less where the spread of coming
    // round would carry the other past what it sees behind and beside it,
    // and never past the break.
    let turn_width = 2.0 * own.speed_mps / turn_rad_s;
    let rolling_in = 2.0 * own.speed_mps * flight.roll_in_s;
    let spread = come_round_spread(own.speed_mps, turn_rad_s, flight.roll_in_s);
    let extend_to = reach_m
        .min(flight.dogfight_break_m)
        .min(seen_behind_m - rolling_in)
        .min(seen_abeam_m - spread)
        .max(0.0);
    let flying_apart = target_forward.dot(to_target) > 0.0;
    if flying_apart && distance < extend_to && come_round.is_none() {
        return Intent::Pursue {
            point: own.pos.xy() + own_forward * turn_width,
            agl,
            nose_at: None,
        };
    }
    let middle = centre - (own.pos.xy() + target) * 0.5;
    let side = *come_round.get_or_insert_with(|| shared_side(right, middle, separation_m));
    Intent::Pursue {
        point: target + side * turn_width,
        agl,
        nose_at: None,
    }
}

/// What shapes a jet's dogfight passes: how far beside the other it flies
/// by (`AirRules::separation_m`), the rate it turns at by choice, the middle
/// of the field it keeps the fight to, the nearest reach of its weapons at
/// the other's layer, and how far it sees the other behind and beside it.
#[derive(Clone, Copy, Debug)]
pub struct DogfightGeometry {
    pub separation_m: f64,
    pub turn_rad_s: f64,
    pub centre: V2,
    pub reach_m: f64,
    pub seen_behind_m: f64,
    pub seen_abeam_m: f64,
}

/// Of the two sides of a line of sight, given one side's normal, the one
/// both jets on it pick alike as they start to come round: the side toward
/// `middle` (from their midpoint to the field's middle), so the fight keeps
/// to the middle; within `tie` of the middle, where the jets' sightings of
/// each other may put it on either side, the side toward a fixed world
/// direction. Each jet, seeing the other along the reversed line, picks the
/// same.
fn shared_side(normal: V2, middle: V2, tie: f64) -> V2 {
    let off_middle = normal.dot(middle);
    let toward = if off_middle.abs() > tie {
        off_middle
    } else if normal.y.abs() > 1e-9 {
        normal.y
    } else {
        normal.x
    };
    if toward >= 0.0 {
        normal
    } else {
        -normal
    }
}

#[cfg(test)]
mod tests {
    use super::{broken, can_dogfight, come_round_spread};
    use crate::math::{v2, v3};
    use crate::units::{Motion, Unit};
    use contract::catalog::AltitudeLayer;
    use contract::ids::UnitId;

    /// Two test jets in high air, as a battle spawns them.
    fn two_jets() -> (Unit, Unit, contract::scenario::FixedWingFlight) {
        let setup = serde_json::from_value(serde_json::json!({
            "map": { "size": [8000, 8000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
            "rules": crate::fixtures::test_rules(),
            "units": [
                { "side": "blue", "kind": "test_jet", "position": [500, 4000] },
                { "side": "red", "kind": "test_jet", "position": [900, 4000] }
            ],
            "events": [], "scripts": []
        }))
        .unwrap();
        let battle = crate::battle::Battle::new(&setup, 1);
        let unit = |id| battle.unit(UnitId(id)).unwrap().clone();
        let air = battle.rules().air.fixed_wing;
        (unit(0), unit(1), air)
    }

    /// Place `jet` at `x` along the field's middle line, flying `vx` along it.
    fn fly(jet: &mut Unit, x: f64, vx: f64) {
        jet.position = v3(x, 4000.0, 1000.0);
        jet.air.as_mut().unwrap().velocity = v3(vx, 0.0, 0.0);
    }

    /// A fight past the break range ends once the pair stop closing (an
    /// even chase or a runaway) but not while they close; a jet coming round
    /// for the other keeps it out to the room its turn back needs, and no
    /// further, and only while the other fights it back.
    #[test]
    fn past_the_break_a_fight_ends_unless_closing_or_coming_round() {
        let (mut blue, mut red, air) = two_jets();
        let Motion::Air(flight) = blue.motion else {
            unreachable!("a jet flies")
        };
        let speed = flight.cruise_mps;
        let room =
            air.dogfight_break_m + come_round_spread(speed, flight.turn_rad_s, air.roll_in_s);
        let past = air.dogfight_break_m + 100.0;
        fly(&mut blue, 0.0, speed);
        fly(&mut red, past, speed);
        assert!(broken(&blue, &red, &air), "an even chase past the break");
        fly(&mut red, past, 2.0 * speed);
        assert!(broken(&blue, &red, &air), "a runaway past the break");
        fly(&mut red, past, -speed);
        assert!(!broken(&blue, &red, &air), "closing head-on past the break");
        fly(&mut red, 400.0, 2.0 * speed);
        assert!(!broken(&blue, &red, &air), "a runaway inside the break");

        blue.air.as_mut().unwrap().come_round = Some((red.id, v2(0.0, 1.0)));
        fly(&mut blue, 0.0, -speed);
        fly(&mut red, room - 100.0, speed);
        assert!(
            broken(&blue, &red, &air),
            "coming round for a jet that has left the fight"
        );
        red.air.as_mut().unwrap().dogfight_target = Some(blue.id);
        assert!(!broken(&blue, &red, &air), "coming round inside its room");
        fly(&mut red, room + 100.0, speed);
        assert!(broken(&blue, &red, &air), "coming round beyond its room");
        fly(&mut red, room - 100.0, speed);
        // Its side loses sight of the other mid-turn: the turn is forgotten
        // with the track, and the fight with it.
        let own = super::FixedWingState {
            pos: blue.position,
            heading: std::f64::consts::PI,
            speed_mps: speed,
            burner: 0.0,
            turn_rate_rad_s: 0.0,
            turn_sign: 1,
            pitch: 0.0,
            pitch_rate_rad_s: 0.0,
        };
        let geometry = super::DogfightGeometry {
            separation_m: 14.0,
            turn_rad_s: flight.turn_rad_s,
            centre: v2(4000.0, 4000.0),
            reach_m: 900.0,
            seen_behind_m: 700.0,
            seen_abeam_m: 700.0,
        };
        let intent = super::link_intent(
            blue.air.as_mut().unwrap(),
            red.id,
            None,
            &own,
            7,
            &air,
            &geometry,
        );
        assert!(intent.is_none(), "no dogfight flown at a lost track");
        assert!(broken(&blue, &red, &air), "lost from sight mid-turn");
        blue.air.as_mut().unwrap().come_round = Some((UnitId(99), v2(0.0, 1.0)));
        assert!(
            broken(&blue, &red, &air),
            "coming round for another jet keeps no fight with this one"
        );
    }

    /// A dogfight forms only between two winged aircraft in one layer: the
    /// layer they share, not high air, decides, and a rotor target in any
    /// layer, the pursuer's own included, gets passes. Rows that fly no law
    /// yet (a winged aircraft in low air, a rotor in high air) stand in for
    /// the pairs the catalog doesn't field.
    #[test]
    fn only_two_winged_aircraft_in_one_layer_dogfight() {
        let setup = serde_json::from_value(serde_json::json!({
            "map": { "size": [2000, 2000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
            "rules": crate::fixtures::test_rules(),
            "units": [
                { "side": "blue", "kind": "test_jet", "position": [500, 500] },
                { "side": "red", "kind": "test_heli", "position": [900, 500] }
            ],
            "events": [], "scripts": []
        }))
        .unwrap();
        let battle = crate::battle::Battle::new(&setup, 1);
        let unit = |id| battle.unit(UnitId(id)).unwrap().clone();
        let (jet, heli) = (unit(0), unit(1));
        let mut low_jet = jet.clone();
        let Motion::Air(flight) = &mut low_jet.motion else {
            unreachable!("a jet flies")
        };
        flight.layer = AltitudeLayer::LowAir;
        let mut high_heli = heli.clone();
        let Motion::Air(flight) = &mut high_heli.motion else {
            unreachable!("a helicopter flies")
        };
        flight.layer = AltitudeLayer::HighAir;
        let pairs: [(&str, &Unit, &Unit, bool); 8] = [
            ("jet on jet", &jet, &jet, true),
            ("low-air jet on low-air jet", &low_jet, &low_jet, true),
            ("jet on helicopter", &jet, &heli, false),
            ("low-air jet on helicopter", &low_jet, &heli, false),
            (
                "jet on a rotor aircraft in high air",
                &jet,
                &high_heli,
                false,
            ),
            ("low-air jet on jet", &low_jet, &jet, false),
            ("helicopter on helicopter", &heli, &heli, false),
            ("helicopter on low-air jet", &heli, &low_jet, false),
        ];
        for (what, pursuer, target, expected) in pairs {
            assert_eq!(can_dogfight(pursuer, target), expected, "{what}");
        }
    }
}
