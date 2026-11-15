//! What the UI hit-tests against: the drawn boxes of every soldier and
//! vehicle the side sees.
use super::*;

/// The boxes of every drawn soldier and vehicle, for the UI's hit tests:
/// a hull's box, else each soldier's body box. The same body-size table feeds
/// UnitArt, so a stand-in's visible block and click box cannot diverge.
pub(super) fn picks(
    sample: &FrameSample,
    units: &UnitsSection,
    body: &BodyBoxes<'_>,
    f: &mut Frame,
) {
    let o = &sample.observation;
    let kinds = o
        .own
        .iter()
        .map(|u| (u.id, &u.kind, 1))
        .chain(o.identified.iter().map(|e| (e.id, &e.kind, 2)));
    let by_id: HashMap<(u64, u32), &Arc<str>> = kinds
        .map(|(id, kind, flag)| ((id.to_bits(), flag), kind))
        .collect();
    for (pose, flag) in sample
        .own
        .iter()
        .map(|p| (p, 1))
        .chain(sample.identified.iter().map(|p| (p, 2)))
    {
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
        if let Some(hull) = units.body_half.get(&***kind).copied().flatten() {
            f.picks.push(pick(NONE, pose.position, hull));
        } else {
            f.picks.push(pick(NONE, pose.position, [0.0; 3]));
            let half = body.half(kind);
            for (at, id) in pose.members.iter().zip(&pose.member_ids) {
                f.picks
                    .push(pick(id.map_or(NONE, |id| id as u32), *at, half));
            }
        }
    }
}
