//! What the UI hit-tests against: the drawn boxes of every soldier and
//! vehicle the side sees.
use super::*;

/// The boxes of every drawn soldier and vehicle, for the UI's hit tests:
/// a hull's box, else each soldier's (`bodyBox`), and each unit's drawn
/// position as a box of no size.
pub(super) fn picks(
    sample: &FrameSample,
    units: &UnitsSection,
    input: &BattleInput,
    f: &mut Frame,
) {
    let o = &sample.observation;
    let kinds = o
        .own
        .iter()
        .map(|u| (u.id, &u.kind, 1))
        .chain(o.identified.iter().map(|e| (e.id, &e.kind, 2)));
    let poses = sample.own.iter().chain(&sample.identified);
    let by_id: HashMap<(u64, u32), &Arc<str>> = kinds
        .map(|(id, kind, flag)| ((id.to_bits(), flag), kind))
        .collect();
    let soldier = input.soldier;
    for (pose, flag) in sample
        .own
        .iter()
        .map(|p| (p, 1))
        .chain(sample.identified.iter().map(|p| (p, 2)))
    {
        let _ = &poses;
        let Some(kind) = by_id.get(&(pose.id.to_bits(), flag)) else {
            continue;
        };
        let aloft = units.types.get(&***kind).is_some_and(|t| t.airborne);
        let flags = flag | if aloft { 4 } else { 0 };
        let pick = |member: u32, at: Vec3, half: Vec3| Pick {
            unit: pose.id as u32,
            member,
            flags,
            pad: 0,
            centre: [
                at[0] as f32,
                at[1] as f32,
                (at[2] + half[2]) as f32,
                pose.yaw as f32,
            ],
            half: [half[0] as f32, half[1] as f32, half[2] as f32, 0.0],
        };
        match input.effects.types.get(&***kind).and_then(|t| t.half) {
            Some(hull) => f.picks.push(pick(NONE, pose.position, hull)),
            None => {
                f.picks.push(pick(NONE, pose.position, [0.0; 3]));
                let half = [soldier.radius_m, soldier.radius_m, soldier.height_m / 2.0];
                for (at, id) in pose.members.iter().zip(&pose.member_ids) {
                    f.picks
                        .push(pick(id.map_or(NONE, |id| id as u32), *at, half));
                }
            }
        }
    }
}
