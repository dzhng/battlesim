//! Physical poses and collision bodies for a tick of flight and movement.

use super::*;

impl Battle {
    /// Where every walking soldier and every vehicle track touches the ground,
    /// in unit order: the same list before and after movement.
    pub(super) fn treads(&self) -> Vec<(V2, Wear)> {
        let mut out = Vec::new();
        for unit in self
            .units
            .iter()
            .filter(|u| u.on_field() && !u.garrisoned() && !u.airborne())
        {
            match unit.hull {
                Some(half) => {
                    let side = v2(0.0, half.y * self.rules.ground.track_gauge).rotated(unit.yaw);
                    let c = unit.position.xy();
                    out.push((c + side, Wear::Tracks));
                    out.push((c - side, Wear::Tracks));
                }
                None => out.extend(unit.member_positions().map(|p| (p.xy(), Wear::Trampled))),
            }
        }
        out
    }

    /// Every unit's pose (ground contact centre and heading), and every
    /// soldier's position in unit then member order.
    pub(super) fn poses(&self) -> Poses {
        Poses {
            units: self
                .units
                .iter()
                .map(|u| Pose {
                    base: u.position,
                    yaw: u.yaw,
                })
                .collect(),
            soldiers: self
                .units
                .iter()
                .flat_map(|u| u.members.iter().map(|s| s.exposed(self.tick)))
                .collect(),
        }
    }

    /// Colliders for this tick: a capsule per living soldier, swept from
    /// where he stood to where he stands, and a box per vehicle.
    pub(super) fn bodies(&self, before: &Poses, after: &Poses) -> Vec<Body> {
        let b = &self.rules.physics;
        let mut bodies = Vec::new();
        let mut first = 0;
        for (i, unit) in self.units.iter().enumerate() {
            let soldiers = first..first + unit.members.len();
            first = soldiers.end;
            if !unit.on_field() {
                continue;
            }
            match unit.hull {
                Some(half) => bodies.push(Body {
                    id: BodyId(VEHICLE_BODY_BASE + unit.id.0),
                    unit: unit.id,
                    shape: Shape::Box { half },
                    from: before.units[i],
                    to: after.units[i],
                }),
                None => {
                    for (s, k) in unit.members.iter().zip(soldiers).filter(|(s, _)| s.alive()) {
                        let at = |p: &Poses, yaw: f64| Pose {
                            base: p.soldiers[k],
                            yaw,
                        };
                        bodies.push(Body {
                            id: BodyId(s.id),
                            unit: unit.id,
                            shape: Shape::Capsule {
                                radius: b.soldier_radius_m,
                                height: b.soldier_height_m,
                            },
                            from: at(before, before.units[i].yaw),
                            to: at(after, after.units[i].yaw),
                        });
                    }
                }
            }
        }
        bodies
    }
}
