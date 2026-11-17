//! What the UI hit-tests against: the drawn boxes of every soldier and
//! vehicle the side sees.
use super::*;
use crate::world::BodyBox;

/// The boxes of every drawn soldier and vehicle, for the UI's hit tests:
/// a hull's box, else each soldier's body box. The same body-size table feeds
/// UnitArt, so a stand-in's visible block and click box cannot diverge.
pub(super) fn picks(sample: &FrameSample, units: &UnitsSection, body: &BodyBox<'_>, f: &mut Frame) {
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
        let aloft = units.types.get(&***kind).is_some_and(|t| t.airborne) && !pose.grounded;
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
        if units.types.get(&***kind).is_some_and(|t| t.hull) {
            let Some(hull) = body.half(kind) else {
                continue;
            };
            f.picks.push(pick(NONE, pose.position, hull));
        } else {
            let Some(half) = body.half(kind) else {
                continue;
            };
            f.picks.push(pick(NONE, pose.position, [0.0; 3]));
            for (at, id) in pose.members.iter().zip(&pose.member_ids) {
                f.picks
                    .push(pick(id.map_or(NONE, |id| id as u32), *at, half));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::interpolate::TickInterpolator;
    use crate::world::SoldierBody;
    use contract::command::{CommandEnvelope, Order};
    use contract::ids::{Side, UnitId};
    use sim::publication::{self, reader::Reader, Publisher};

    #[test]
    fn a_helicopters_pick_is_aloft_only_until_touchdown_for_either_side() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        for side in [Side::Blue, Side::Red] {
            let rules = crate::static_world::files::rules(&root, "supply").unwrap();
            let scenario = sim::maps::scenario("supply", "heli-fire", rules).unwrap();
            let mut battle = sim::battle::Battle::new(&scenario, 13);
            let kinds = crate::fielded::encounter_kinds(&root, "supply", "heli-fire").unwrap();
            let fielded = crate::fielded::fielded(&root, battle.rules(), &kinds).unwrap();
            let body = BodyBox::new(
                &fielded.units,
                SoldierBody {
                    radius_m: 0.2,
                    height_m: 1.8,
                },
            );
            let mut publisher = Publisher::new();
            let mut reader = Reader::new(&publication::layout_json(&battle)).unwrap();
            let mut interpolation = TickInterpolator::new(battle.rules().tick_hz as f64);
            let mut flying = false;
            let mut landed = false;
            let mut packed = false;
            for _ in 0..900 {
                battle.step();
                let words = publisher
                    .publish(&battle, side)
                    .unwrap()
                    .iter()
                    .map(|f| f.to_bits())
                    .collect::<Vec<_>>();
                let observation = Arc::new(reader.decode(&words).unwrap().unwrap());
                let time = observation.tick / battle.rules().tick_hz as f64;
                interpolation.push(observation, time);
                let sample = interpolation.frame(time).unwrap();
                let heli_id = sample
                    .observation
                    .own
                    .iter()
                    .find(|u| &*u.kind == "test_supply_heli")
                    .map(|u| u.id)
                    .or_else(|| {
                        sample
                            .observation
                            .identified
                            .iter()
                            .find(|u| &*u.kind == "test_supply_heli")
                            .map(|u| u.id)
                    });
                let (poses, owner_flag) = if side == Side::Blue {
                    (&sample.own, 1)
                } else {
                    (&sample.identified, 2)
                };
                let Some(heli) = poses.iter().find(|u| Some(u.id) == heli_id) else {
                    continue;
                };
                let mut frame = Frame::default();
                picks(sample, &fielded.units, &body, &mut frame);
                let pick = frame
                    .picks
                    .iter()
                    .find(|p| p.unit == heli.id as u32 && p.flags & 3 == owner_flag)
                    .unwrap();
                assert_eq!(pick.flags & 4 != 0, !heli.grounded);
                flying |= !heli.grounded;
                landed |= heli.grounded;
                if flying && landed {
                    break;
                }
                // An enemy can first identify it after touchdown. Lift it
                // again so that viewer also observes the airborne case.
                if landed && !packed {
                    let ack = battle.accept(CommandEnvelope {
                        side: Side::Blue,
                        player: None,
                        seq: 1,
                        order: Order::SetDeployment {
                            units: vec![UnitId(0)],
                            deployed: false,
                        },
                        queued: false,
                    });
                    assert_eq!(ack.error, None);
                    packed = true;
                }
            }
            assert!(flying && landed, "both flight and touchdown are observed: {side:?}, flying={flying}, landed={landed}, packed={packed}");
        }
    }
}
