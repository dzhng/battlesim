//! Where a soldier's round leaves from: where he stands, or out on his lean
//! round the body he takes cover behind, and when that body holds his round.
use contract::ids::UnitId;

use super::FireContext;
use crate::lean;
use crate::math::V3;
use crate::world::{Collider, PropId};

/// Where a soldier fires a round from.
#[derive(Clone, Copy)]
pub(super) struct FirePoint {
    pub(super) origin: V3,
    /// The body he leans round, which his rounds pass (the own-cover
    /// rule): a round grazing its edge does not strike it.
    pub(super) past: Option<PropId>,
    /// The friendly hull he leans round, which never withholds his shot.
    pub(super) hull: Option<UnitId>,
    /// Fired from his lean point.
    pub(super) leaning: bool,
}

/// Where a soldier fires a round at `point` from, when it reaches:
/// from his muzzle where he stands when the round reaches straight from
/// there; else from his claimed lean point (while he is at the place it was
/// claimed with) when it reaches from there. Either way his round passes
/// the body he leans round. `None`: it reaches from neither. The line test
/// is [`lean::reaches`], the one the cover search judges places by.
pub(super) fn fire_from(
    ctx: &FireContext,
    blockers: &[lean::Hull],
    soldier: &crate::units::Soldier,
    point: V3,
    offset: V3,
) -> Option<FirePoint> {
    let muzzle = |p: crate::math::V2| {
        p.with_z(ctx.world.height_at(p.x, p.y).unwrap_or(soldier.position.z)) + offset
    };
    let lean = soldier
        .lean
        .as_ref()
        .filter(|l| (l.from - soldier.position.xy()).length() <= lean::AT_PLACE_M);
    let past = lean.and_then(|l| l.past());
    let hull = lean.and_then(|l| match l.body {
        lean::Round::Hull(u) => Some(u),
        lean::Round::Prop(_) => None,
    });
    let standing = soldier.position + offset;
    if lean::reaches(ctx.world, blockers, standing, point, None) {
        return Some(FirePoint {
            origin: standing,
            past,
            hull,
            leaning: false,
        });
    }
    // Out on a burst, or tucked in long enough to lean out again.
    let l = lean.filter(|l| {
        (ctx.tick < soldier.leaning_until || ctx.tick >= soldier.tucked_until)
            && lean::reaches(ctx.world, blockers, muzzle(l.at), point, past)
    })?;
    Some(FirePoint {
        origin: muzzle(l.at),
        past,
        hull,
        leaning: true,
    })
}

/// Whether the first body on a soldier's straight line from `origin` to
/// `point` is one he takes cover behind from it (Q20's test, the one that
/// gives him a tier).
pub(super) fn hides_behind(
    ctx: &FireContext,
    soldier: &crate::units::Soldier,
    origin: V3,
    point: V3,
) -> bool {
    let to = point - origin;
    let len = to.length();
    if len < 1e-6 {
        return false;
    }
    let Some(hit) = ctx.world.raycast(origin, to * (1.0 / len), len) else {
        return false;
    };
    let Collider::Prop(id) = hit.collider else {
        return false;
    };
    let Some(prop) = ctx.world.prop(id) else {
        return false;
    };
    let (at, reach) = (soldier.position.xy(), ctx.rules.cover.reach_m);
    let radius = ctx.rules.physics.soldier_radius_m;
    crate::cover::covers(&prop.footprint(), at, point.xy(), reach, radius)
}
