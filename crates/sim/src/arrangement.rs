//! Where soldiers stand. There is no formation (D1): a squad ends every move
//! in a fresh, seeded random arrangement around the point it was sent to,
//! and a soldier placed on his own (out of a collapsing building, or a
//! replacement joining his squad) takes the nearest free standing spot.
//!
//! Every spot is standing room for a soldier's disc, clear of the solids the
//! caller names (a side's known props for orders, the true world for
//! placements), and reachable on foot in a straight line from where the
//! soldier sets out, so no spot lies behind a wall from its squad.
use contract::scenario::InfantryMovementRules;

use crate::math::{v2, V2};
use crate::rng::Rng;
use crate::world::{Prop, WorldGeometry};

/// Draws per soldier before the arrangement gives up on spacing him.
const DRAWS: usize = 24;
/// Rings of the nearest-free search are this far apart.
const RING_STEP_M: f64 = 0.5;

/// The random draws for one arrangement: fixed by the battle's seed, the
/// squad and the tick, so a replay draws the same spots and every order
/// draws new ones.
pub fn rng(seed: u64, unit: u32, tick: u64) -> Rng {
    let mut mix = Rng::new(seed ^ (u64::from(unit) << 40) ^ tick.rotate_left(17));
    Rng::new(mix.next_u64())
}

/// The ground a soldier's disc of `radius` stands on at `p`: inside the map,
/// traversable, and clear of every prop `solid` names.
pub fn standing_room(
    world: &WorldGeometry,
    p: V2,
    radius: f64,
    solid: &impl Fn(&Prop) -> bool,
) -> bool {
    world.traversable_at(p.x, p.y)
        && !world
            .props_near(p, radius)
            .iter()
            .any(|q| solid(q) && q.footprint().contains(p, radius))
}

/// A soldier's disc can walk straight from `a` to `b`: no solid in the way
/// (one he already stands in aside, as he may always step out of it) and
/// traversable ground at every half metre.
pub fn reachable(
    world: &WorldGeometry,
    a: V2,
    b: V2,
    radius: f64,
    solid: &impl Fn(&Prop) -> bool,
) -> bool {
    let length = (b - a).length();
    let blocked = world
        .props_near((a + b) * 0.5, length / 2.0 + radius)
        .iter()
        .any(|q| {
            let box_ = q.footprint();
            solid(q) && box_.meets_segment(a, b, radius) && !box_.contains(a, radius)
        });
    if blocked {
        return false;
    }
    let n = (length / RING_STEP_M).ceil().max(1.0) as usize;
    (0..=n).all(|k| {
        let p = a + (b - a) * (k as f64 / n as f64);
        world.traversable_at(p.x, p.y)
    })
}

/// The spread's diameter for a squad of `count`: the same ground per soldier
/// as the rules' reference squad (Q7).
pub fn spread(rules: &InfantryMovementRules, count: usize) -> f64 {
    rules.spread_m * (count as f64 / rules.spread_squad_size.max(1) as f64).sqrt()
}

/// One spot per soldier around `centre`: drawn uniformly over the spread,
/// at least `spacing` apart, recentred so the squad's middle is `centre`,
/// then each spot that is not free standing room reachable from `centre` is
/// moved to the nearest one that is. `None` when some soldier finds no spot
/// within twice the spread.
#[allow(clippy::too_many_arguments)]
pub fn arrange(
    world: &WorldGeometry,
    centre: V2,
    count: usize,
    diameter: f64,
    spacing: f64,
    radius: f64,
    solid: &impl Fn(&Prop) -> bool,
    rng: &mut Rng,
) -> Option<Vec<V2>> {
    let r = diameter / 2.0;
    let mut offsets: Vec<V2> = Vec::with_capacity(count);
    for _ in 0..count {
        // The draw farthest from the others, if none keeps the spacing.
        let mut best = (f64::NEG_INFINITY, v2(0.0, 0.0));
        for _ in 0..DRAWS {
            let (u, a) = (rng.unit(), rng.unit() * std::f64::consts::TAU);
            let o = v2(a.cos(), a.sin()) * (r * u.sqrt());
            let gap = offsets
                .iter()
                .map(|p| (*p - o).length())
                .fold(f64::INFINITY, f64::min);
            if gap > best.0 {
                best = (gap, o);
            }
            if gap >= spacing {
                break;
            }
        }
        offsets.push(best.1);
    }
    let mean = offsets.iter().fold(v2(0.0, 0.0), |a, &o| a + o) * (1.0 / count.max(1) as f64);
    let mut spots: Vec<V2> = Vec::with_capacity(count);
    for o in offsets {
        let wanted = centre + o - mean;
        let fits = |p: V2, taken: &[V2]| {
            taken.iter().all(|t| (*t - p).length() >= spacing)
                && standing_room(world, p, radius, solid)
                && reachable(world, centre, p, radius, solid)
        };
        let spot = if fits(wanted, &spots) {
            wanted
        } else {
            nearest_free(wanted, 2.0 * r.max(spacing), |p| fits(p, &spots))?
        };
        spots.push(spot);
    }
    Some(spots)
}

/// The nearest point to `from`, on rings outward from it every half metre up
/// to `radius`, that `fits`.
pub fn nearest_free(from: V2, radius: f64, fits: impl Fn(V2) -> bool) -> Option<V2> {
    let rings = (radius / RING_STEP_M).floor() as usize;
    for ring in 0..=rings {
        let r = ring as f64 * RING_STEP_M;
        let n = ((std::f64::consts::TAU * r / RING_STEP_M).ceil() as usize).max(1);
        for k in 0..n {
            let a = std::f64::consts::TAU * k as f64 / n as f64;
            let p = from + v2(a.cos(), a.sin()) * r;
            if fits(p) {
                return Some(p);
            }
        }
    }
    None
}

/// A squad's spots around `centre` by the rules' spread and spacing; where
/// the ground is too cramped for that, at half the spacing, and failing that
/// every soldier heads for `centre` itself.
pub fn squad_spots(
    world: &WorldGeometry,
    centre: V2,
    count: usize,
    rules: &InfantryMovementRules,
    radius: f64,
    solid: &impl Fn(&Prop) -> bool,
    rng: &mut Rng,
) -> Vec<V2> {
    let diameter = spread(rules, count);
    arrange(
        world,
        centre,
        count,
        diameter,
        rules.spacing_m,
        radius,
        solid,
        rng,
    )
    .or_else(|| {
        let spacing = rules.spacing_m / 2.0;
        arrange(world, centre, count, diameter, spacing, radius, solid, rng)
    })
    .unwrap_or_else(|| vec![centre; count])
}
