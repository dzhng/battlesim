//! Indirect-fire evidence and aim envelopes for the skirmish doctrine.

use super::*;

pub(super) fn artillery_in_range(
    unit: &contract::observation::OwnUnit,
    rules: &Rules,
    point: [f64; 3],
) -> bool {
    let range = ground_distance(unit.position, point);
    rules
        .catalog
        .mounts(unit.kind)
        .iter()
        .flat_map(|mount| &mount.def.weapons)
        .filter_map(|id| rules.weapons.get(id))
        .filter(|weapon| weapon.ballistics.trajectory == contract::ballistics::Trajectory::Indirect)
        .any(|weapon| {
            weapon
                .against(AltitudeLayer::Ground)
                .is_some_and(|fire| range >= weapon.min_range_m && range <= fire.range_m)
        })
}

/// Contacts have no emitter identity or motion. Retained sightings have a
/// side-scoped observed handle, so they can predict only their own last view.
pub(super) fn artillery_ground_guess(
    unit: &contract::observation::OwnUnit,
    frame: &ObservationFrame,
    rules: &Rules,
    tuning: &Tuning,
    sightings: &BTreeMap<ObservedTargetId, ArtillerySighting>,
) -> Option<[f64; 3]> {
    let horizon = Tuning::ticks(tuning.sighting_horizon_s, rules.tick_hz);
    let contacts = frame
        .contacts
        .iter()
        .filter(|contact| {
            contact.layer == AltitudeLayer::Ground
                && !contact.aloft
                && contact.evidence_tick <= frame.tick
                && frame.tick <= contact.expires_tick
                && frame.tick - contact.evidence_tick <= horizon
        })
        .filter_map(|contact| {
            let angle = f64::from(contact.id.0) * 2.399963229728653;
            let bias = contact.radius.min(tuning.aim_cap_m) * tuning.contact_aim_share;
            let point = [
                contact.center[0] + libm::cos(angle) * bias,
                contact.center[1] + libm::sin(angle) * bias,
                contact.center[2],
            ];
            // Reverse the uncertain displacement at a range edge; a report
            // cannot turn into an exact emitter fix merely to make it fire.
            let opposite = [
                contact.center[0] - libm::cos(angle) * bias,
                contact.center[1] - libm::sin(angle) * bias,
                contact.center[2],
            ];
            [point, opposite]
                .into_iter()
                .find(|point| artillery_in_range(unit, rules, *point))
                .map(|point| (point, contact.id.0))
        });
    let predicted = sightings
        .iter()
        .filter(|(id, seen)| {
            !frame.identified.iter().any(|target| target.id == **id)
                && seen.tick <= frame.tick
                && frame.tick - seen.tick <= horizon
        })
        .filter_map(|(id, seen)| {
            let seconds = (frame.tick - seen.tick) as f64 / f64::from(rules.tick_hz);
            let point = [
                seen.position[0] + seen.velocity[0] * seconds,
                seen.position[1] + seen.velocity[1] * seconds,
                seen.position[2],
            ];
            artillery_in_range(unit, rules, point).then_some((point, id.0))
        });
    contacts
        .chain(predicted)
        .min_by(|(a, ai), (b, bi)| {
            ground_distance(unit.position, *a)
                .total_cmp(&ground_distance(unit.position, *b))
                .then(ai.cmp(bi))
        })
        .map(|(point, _)| point)
}

/// Public enemy-held or contested objectives support a fallible route guess.
/// Neutral empty flags alone reveal no enemy presence and do not justify fire.
pub(super) fn artillery_objective_guess(
    unit: &contract::observation::OwnUnit,
    side: Side,
    entry: &EntrySite,
    frame: &ObservationFrame,
    rules: &Rules,
    tuning: &Tuning,
    route_time: &mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
) -> Option<[f64; 3]> {
    frame
        .skirmish
        .as_ref()?
        .objectives
        .iter()
        .filter(|objective| {
            objective.owner.is_some_and(|owner| owner != side)
                || objective.contested
                || objective.capturing.is_some_and(|owner| owner != side)
        })
        .filter_map(|objective| {
            // Aim at the far approach to the objective from our public entry,
            // never at a hidden occupant or a guaranteed centre-point hit.
            let dx = objective.center[0] - entry.center[0];
            let dy = objective.center[1] - entry.center[1];
            let length = libm::hypot(dx, dy);
            if length <= 0.0 {
                return None;
            }
            let offset = objective.radius_m.min(tuning.aim_cap_m) * tuning.objective_aim_share;
            let point = [
                objective.center[0] + dx / length * offset,
                objective.center[1] + dy / length * offset,
                unit.position[2],
            ];
            if !artillery_in_range(unit, rules, point) {
                return None;
            }
            route_time(unit.id, [point[0], point[1]])
                .filter(|time| time.is_finite() && *time >= 0.0)
                .map(|time| (point, time, &objective.id))
        })
        .min_by(|(_, at, ai), (_, bt, bi)| at.total_cmp(bt).then(ai.cmp(bi)))
        .map(|(point, _, _)| point)
}
