//! Infantry cover, a hard-coded game rule (Q20): cover protects soldiers
//! only, through the spread of rounds aimed at them (Q5), never through a
//! height model; crouching is animation. A cover body is any body with a
//! tier (D6): a prop kind's row in the fixture, a vehicle by its weight
//! class, live or wrecked (Q24), and a crater under his feet. When a round
//! is aimed at a soldier, the strongest body within `reach_m` of him that
//! lies between him and the shooter widens that round's spread by its tier.
//! A ground body (a trench, a row that blocks no infantry) covers whoever
//! stands in it instead, whatever the direction, as a crater does.
//! A garrison keeps its building shelter instead (Q22).
//!
//! Soldiers seek cover too. A squad's spots are resolved when the order is
//! given (D4): each soldier claims the best spot near his place in the
//! arrangement, behind a body's face on the side away from the threat
//! (`spots`), and whoever finds none keeps his random place. A squad at rest
//! or halted re-resolves around where its soldiers stand (D5, Q11), keeping
//! whoever already has the best cover he could claim, and a soldier whose
//! line to his target is blocked steps to a clear spot nearby (D3).
//!
//! Which bodies count is the caller's: the true world for the spread, a
//! side's knowledge for seeking.
use contract::ids::UnitId;
use contract::map::MoverClass;
use contract::scenario::{CoverRules, Rules, UnitKind, WeightClass};

pub use contract::scenario::CoverTier as Tier;

use crate::ground::{GroundLayer, KnownGround};
use crate::knowledge::SideKnowledge;
use crate::math::{v2, Obb2, V2};
use crate::units::Unit;
use crate::weapons::Target;
use crate::world::{Prop, WorldGeometry};

/// A spot stands this far off the face it hides behind, beyond the
/// soldier's own radius.
const STANDOFF_M: f64 = 0.2;
/// A face counts as turned away from the threat within about 70° of
/// straight away (cosine): one the threat sees nearly edge-on hides nobody.
const AWAY: f64 = -0.35;
/// Step-out candidates lie on rings this far apart (D3).
const STEP_RING_M: f64 = 0.5;
/// A threat given only as a direction is placed this far off.
pub const FAR_M: f64 = 200.0;

/// What a squad's cover was last resolved against (Q11): the threat, when,
/// the side's planning revision and the craters it knew nearby, and each live vehicle a
/// soldier took cover behind with where it stood. `due`: arrival or a halt
/// asks for a re-resolve as soon as the throttle allows.
#[derive(Clone, Debug, Default)]
pub struct Watch {
    pub threat: Option<V2>,
    /// The threat is an enemy the side has seen, not the way it was sent.
    pub hostile: bool,
    pub resolved_at: u64,
    pub revision: u64,
    /// Crater cells the side knew around the squad (a new crater re-resolves).
    pub craters: u32,
    pub vehicles: Vec<(UnitId, V2)>,
    pub due: bool,
}

impl Watch {
    pub fn digest(&self, d: &mut crate::digest::Digest) {
        d.opt_v2(self.threat)
            .u64(self.hostile as u64)
            .u64(self.resolved_at)
            .u64(self.revision)
            .u64(self.craters as u64)
            .u64(self.due as u64)
            .u64(self.vehicles.len() as u64);
        for (id, p) in &self.vehicles {
            d.u64(id.0 as u64).f64(p.x).f64(p.y);
        }
    }
}

/// A body that can cover a soldier: its footprint and its tier. `vehicle`
/// names a live vehicle's hull (its users re-resolve when it drives off).
/// A `ground` body (a trench) covers who stands in it, not who hides behind.
#[derive(Clone, Copy, Debug)]
pub struct Body {
    pub rect: Obb2,
    pub tier: Tier,
    pub vehicle: Option<UnitId>,
    pub ground: bool,
}

/// A prop's cover tier: its body row's `cover_tier` column, its one reader.
pub fn prop_tier(prop: &Prop) -> Option<Tier> {
    prop.body.cover_tier
}

/// A prop as a cover body, if its row gives cover.
fn prop_body(prop: &Prop) -> Option<Body> {
    Some(Body {
        rect: prop.footprint(),
        tier: prop_tier(prop)?,
        vehicle: None,
        ground: !prop.blocks(MoverClass::Infantry),
    })
}

/// A live vehicle's cover tier, by its weight class (Q24): light, medium or
/// heavy as the class; its wreck's row keeps the same tier.
pub fn vehicle_tier(kind: UnitKind, rules: &Rules) -> Option<Tier> {
    match crate::units::body(kind, rules).weight_class? {
        WeightClass::Light => Some(Tier::Light),
        WeightClass::Medium => Some(Tier::Medium),
        WeightClass::Heavy => Some(Tier::Heavy),
        WeightClass::Immovable => None,
    }
}

/// Every live vehicle's hull as a cover body, either side.
pub fn hulls<'a>(units: impl IntoIterator<Item = &'a Unit>, rules: &Rules) -> Vec<Body> {
    units
        .into_iter()
        .filter(|u| u.alive())
        .filter_map(|u| {
            Some(Body {
                rect: u.hull_box()?,
                tier: vehicle_tier(u.kind, rules)?,
                vehicle: Some(u.id),
                ground: false,
            })
        })
        .collect()
}

/// Distance from `p` to a rectangle (0 inside).
fn distance(r: &Obb2, p: V2) -> f64 {
    let d = r.to_local(p);
    let (x, y) = (
        (d.x.abs() - r.half.x).max(0.0),
        (d.y.abs() - r.half.y).max(0.0),
    );
    x.hypot(y)
}

/// Whether `rect` covers a soldier of `radius` at `p` from a shooter at
/// `from` (Q20): it lies within `reach` of him, and between them: his disc,
/// swept `reach` toward the shooter, meets it. A soldier peering past a
/// trunk's edge is still behind it; one beside a wall's end is not.
pub fn covers(rect: &Obb2, p: V2, from: V2, reach: f64, radius: f64) -> bool {
    let d = from - p;
    let len = d.length();
    len > 1e-9
        && distance(rect, p) <= reach
        && rect.meets_segment(p, p + d * (reach.min(len) / len), radius)
}

/// Whether `b` covers a soldier at `p` from `from`: from behind it, or for
/// a ground body from inside it.
fn body_covers(b: &Body, p: V2, from: V2, reach: f64, radius: f64) -> bool {
    if b.ground {
        b.rect.contains(p, 0.0)
    } else {
        covers(&b.rect, p, from, reach, radius)
    }
}

/// The strongest tier among `bodies` covering a soldier at `p` from `from`,
/// and `ground` (a crater he stands in) whatever the direction.
pub fn strongest<'a>(
    bodies: impl IntoIterator<Item = &'a Body>,
    ground: Option<Tier>,
    p: V2,
    from: V2,
    rules: &CoverRules,
    radius: f64,
) -> Option<Tier> {
    bodies
        .into_iter()
        .filter(|b| body_covers(b, p, from, rules.reach_m, radius))
        .map(|b| b.tier)
        .chain(ground)
        .max()
}

/// The crater tier at `p`, from a crater's fill there.
fn crater(fill: f64, rules: &CoverRules) -> Option<Tier> {
    (fill >= rules.crater_min_fill).then_some(rules.crater)
}

/// The cover a soldier at `p` has now against a round from `from`, in the
/// true world: every tiered prop, every live hull in `hulls`, and the crater
/// under him. What the spread reads (weapons) and the scenario checks judge.
pub fn at(
    world: &WorldGeometry,
    ground: &GroundLayer,
    hulls: &[Body],
    rules: &Rules,
    p: V2,
    from: V2,
) -> Option<Tier> {
    let c = &rules.cover;
    let r = rules.physics.soldier_radius_m;
    let props: Vec<Body> = world
        .props_near(p, c.reach_m)
        .into_iter()
        .filter_map(prop_body)
        .collect();
    let near = hulls
        .iter()
        .filter(|h| (h.rect.center - p).length() <= h.rect.half.length() + c.reach_m);
    let fill = ground.crater_fill(p.x, p.y, &rules.ground);
    strongest(props.iter().chain(near), crater(fill, c), p, from, c, r)
}

/// The spread multiplier a soldier's cover `tier` gives a round aimed at him.
pub fn spread(tier: Option<Tier>, rules: &CoverRules) -> f64 {
    tier.map_or(1.0, |t| rules.tiers.spread(t))
}

/// Cover rules the simulation can honour: every tier widens the spread,
/// heavier tiers no less; a wreck keeps its live vehicle's tier (Q24).
pub fn validate(rules: &Rules) {
    let c = &rules.cover;
    let t = &c.tiers;
    assert!(
        1.0 <= t.light && t.light <= t.medium && t.medium <= t.heavy,
        "cover.tiers must widen, heavier no less: {} {} {}",
        t.light,
        t.medium,
        t.heavy
    );
    for (kind, mover) in &rules.bodies {
        if let Some(wreck) = mover.wreck {
            let row = rules.props.get(&wreck).map(|b| b.cover_tier);
            assert_eq!(
                row,
                Some(vehicle_tier(*kind, rules)),
                "props.{wreck:?}: a wreck keeps its vehicle's cover tier (Q24)"
            );
        }
    }
    for (name, v) in [
        ("reach_m", c.reach_m),
        ("search_m", c.search_m),
        ("step_out_m", c.step_out_m),
        ("reresolve_s", c.reresolve_s),
        ("vehicle_moved_m", c.vehicle_moved_m),
        ("crater_min_fill", c.crater_min_fill),
    ] {
        assert!(v > 0.0, "cover.{name} must be positive");
    }
}

/// The centres of the cells within `radius` of `centre` where a side has
/// seen a crater deep enough to be cover.
pub fn craters(ground: &KnownGround, rules: &Rules, centre: V2, radius: f64) -> Vec<V2> {
    let cell = rules.ground.cell_m;
    let n = (radius / cell).ceil() as i64;
    let mut out = Vec::new();
    for j in -n..=n {
        for i in -n..=n {
            let p = v2(
                ((centre.x / cell).floor() + i as f64 + 0.5) * cell,
                ((centre.y / cell).floor() + j as f64 + 0.5) * cell,
            );
            if (p - centre).length() > radius {
                continue;
            }
            let fill = ground.cell(p.x, p.y).crater as f64 / rules.ground.crater_full_depth;
            if crater(fill, &rules.cover).is_some() {
                out.push(p);
            }
        }
    }
    out
}

/// Cover a side may seek around a squad: the tiered props it plans with,
/// its own live vehicles, and the craters it has seen.
pub struct Known {
    pub bodies: Vec<Body>,
    pub craters: Vec<V2>,
    crater_tier: Tier,
    cell: f64,
}

impl Known {
    /// Everything within `radius` of `centre` that `knows` (the side's
    /// planning knowledge) admits, plus `own` vehicles' hulls.
    pub fn gather(
        world: &WorldGeometry,
        ground: &KnownGround,
        rules: &Rules,
        own: &[Body],
        knows: &impl Fn(&Prop) -> bool,
        centre: V2,
        radius: f64,
    ) -> Known {
        let c = &rules.cover;
        let mut bodies: Vec<Body> = world
            .props_near(centre, radius)
            .into_iter()
            .filter(|q| knows(q))
            .filter_map(prop_body)
            .collect();
        bodies.extend(
            own.iter()
                .filter(|h| (h.rect.center - centre).length() <= radius + h.rect.half.length()),
        );
        let craters = craters(ground, rules, centre, radius);
        Known {
            bodies,
            craters,
            crater_tier: c.crater,
            cell: rules.ground.cell_m,
        }
    }

    /// The cover this side believes a soldier at `p` has from `from`.
    pub fn tier(&self, p: V2, from: V2, rules: &CoverRules, radius: f64) -> Option<Tier> {
        let half = self.cell / 2.0;
        let ground = self
            .craters
            .iter()
            .any(|c| (c.x - p.x).abs() <= half && (c.y - p.y).abs() <= half)
            .then_some(self.crater_tier);
        strongest(&self.bodies, ground, p, from, rules, radius)
    }

    /// The live vehicle whose hull covers `p` from `from`, if any.
    pub fn vehicle(&self, p: V2, from: V2, rules: &CoverRules, radius: f64) -> Option<UnitId> {
        self.bodies
            .iter()
            .filter(|b| body_covers(b, p, from, rules.reach_m, radius))
            .max_by_key(|b| b.tier)
            .and_then(|b| b.vehicle)
    }
}

/// A place a soldier can take cover, and the tier it gives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub at: V2,
    pub tier: Tier,
}

/// Cover spots against `threat` (Q7, the mock lesson): behind each face of
/// a known body turned away from the threat, along its normal, `spacing`
/// apart along the face; and in each seen crater. A spot counts only where
/// the side believes it covers, and a soldier can stand (`stands`).
pub fn spots(
    known: &Known,
    threat: V2,
    rules: &CoverRules,
    radius: f64,
    spacing: f64,
    stands: &impl Fn(V2) -> bool,
) -> Vec<Spot> {
    let mut out = Vec::new();
    let mut offer = |p: V2| {
        if let Some(tier) = known.tier(p, threat, rules, radius) {
            if stands(p) {
                out.push(Spot { at: p, tier });
            }
        }
    };
    for b in &known.bodies {
        let r = &b.rect;
        if b.ground {
            // A ground body is cover inside it: spots down its long middle.
            let (along, half) = if r.half.x >= r.half.y {
                (v2(1.0, 0.0).rotated(r.yaw), r.half.x)
            } else {
                (v2(0.0, 1.0).rotated(r.yaw), r.half.y)
            };
            let reach = (half - radius).max(0.0);
            let n = (2.0 * reach / spacing).floor() as usize + 1;
            for k in 0..n {
                let t = match n {
                    1 => 0.0,
                    _ => -reach + 2.0 * reach * k as f64 / (n - 1) as f64,
                };
                offer(r.center + along * t);
            }
            continue;
        }
        let toward = threat - r.center;
        if toward.length() < 1e-9 {
            continue;
        }
        let toward = toward.normalized();
        let ax = v2(1.0, 0.0).rotated(r.yaw);
        let ay = v2(0.0, 1.0).rotated(r.yaw);
        for (normal, along, depth, half) in [
            (ax, ay, r.half.x, r.half.y),
            (-ax, ay, r.half.x, r.half.y),
            (ay, ax, r.half.y, r.half.x),
            (-ay, ax, r.half.y, r.half.x),
        ] {
            if normal.dot(toward) > AWAY {
                continue;
            }
            // Spots from corner to corner, `spacing` or more apart; a face
            // shorter than that holds one, at its middle.
            let n = (2.0 * half / spacing).floor() as usize + 1;
            let base = r.center + normal * (depth + radius + STANDOFF_M);
            for k in 0..n {
                let t = match n {
                    1 => 0.0,
                    _ => -half + 2.0 * half * k as f64 / (n - 1) as f64,
                };
                offer(base + along * t);
            }
        }
    }
    for &c in &known.craters {
        offer(c);
    }
    out
}

/// Who takes which spot (D4, Q11). Each soldier may take a spot within
/// `search` of his place `from[k]`, or stay there if `stay[k]` (which then
/// gives the tier it holds). Best tier first; on a tie a soldier staying
/// wins, then the nearest. A spot is taken by one soldier, `spacing` from
/// every other taken place and clear of `keep_clear` (where others stand).
/// Returns each soldier's spot index, or `None` for his own place.
pub fn claim(
    from: &[V2],
    stay: &[Option<Option<Tier>>],
    spots: &[Spot],
    search: f64,
    spacing: f64,
    keep_clear: f64,
) -> Vec<Option<usize>> {
    // (tier, staying, distance, soldier, spot)
    type Offer = (Option<Tier>, bool, f64, usize, Option<usize>);
    let mut offers: Vec<Offer> = Vec::new();
    for (k, &p) in from.iter().enumerate() {
        if let Some(tier) = stay[k] {
            offers.push((tier, true, 0.0, k, None));
        }
        for (i, s) in spots.iter().enumerate() {
            let d = (s.at - p).length();
            if d <= search {
                offers.push((Some(s.tier), false, d, k, Some(i)));
            }
        }
    }
    offers.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(b.1.cmp(&a.1))
            .then(a.2.total_cmp(&b.2))
            .then(a.3.cmp(&b.3))
            .then(a.4.cmp(&b.4))
    });
    let mut chosen: Vec<Option<Option<usize>>> = vec![None; from.len()];
    let mut taken: Vec<V2> = Vec::new();
    for (_, staying, _, k, spot) in offers {
        if chosen[k].is_some() {
            continue;
        }
        let Some(i) = spot else {
            debug_assert!(staying);
            chosen[k] = Some(None);
            taken.push(from[k]);
            continue;
        };
        let at = spots[i].at;
        let crowded = taken.iter().any(|t| (*t - at).length() < spacing)
            || from
                .iter()
                .enumerate()
                .any(|(j, q)| j != k && chosen[j].is_none() && (*q - at).length() < keep_clear);
        if !crowded {
            chosen[k] = Some(Some(i));
            taken.push(at);
        }
    }
    chosen.into_iter().map(Option::flatten).collect()
}

/// An enemy a squad reckons with: where its side knows it to be, and the
/// enemy unit, if it is one (a ground point under fire is not).
#[derive(Clone, Copy, Debug)]
pub struct Enemy {
    pub at: V2,
    pub unit: Option<UnitId>,
}

/// What a squad's weapons are engaging, as its side knows it: an identified
/// enemy or a ground point. Area fire at a contact names no place (its
/// centre is only an estimate).
pub fn engaged(unit: &Unit, knowledge: &SideKnowledge) -> Option<Enemy> {
    unit.mounts.iter().find_map(|m| {
        let lock = m.lock.as_ref().filter(|l| l.engaging)?;
        match lock.target {
            Target::Unit(u) => knowledge.track(u).map(|t| Enemy {
                at: t.position.xy(),
                unit: Some(u),
            }),
            Target::Contact(_) => None,
            Target::Ground(p) => Some(Enemy {
                at: p.xy(),
                unit: None,
            }),
        }
    })
}

/// Where a squad's threat is (Q7, Q9): the enemy one of its weapons is
/// engaging, else the nearest enemy its side identified at its latest
/// sensing, `sensed`. `None`: no enemy in view, and the caller keeps the
/// last threat or the order's direction.
pub fn threat(unit: &Unit, knowledge: &SideKnowledge, sensed: u64) -> Option<Enemy> {
    let here = unit.position.xy();
    engaged(unit, knowledge).or_else(|| {
        knowledge
            .identified_now(sensed)
            .map(|(u, t)| Enemy {
                at: t.position.xy(),
                unit: Some(u),
            })
            .min_by(|a, b| (a.at - here).length().total_cmp(&(b.at - here).length()))
    })
}

/// A step-out spot (D3, Q8) for a soldier at `from` whose line to `target`
/// is blocked: the nearest place within `rules.step_out_m`, best cover
/// first, where he stands (`stands`) and his line is `clear`. `None`: he
/// sits out.
pub fn step_out(
    from: V2,
    target: V2,
    known: &Known,
    rules: &CoverRules,
    radius: f64,
    stands: &impl Fn(V2) -> bool,
    clear: &impl Fn(V2) -> bool,
) -> Option<V2> {
    let rings = (rules.step_out_m / STEP_RING_M).floor() as usize;
    let mut best: Option<(Option<Tier>, f64, V2)> = None;
    for ring in 1..=rings {
        let r = ring as f64 * STEP_RING_M;
        let n = ((std::f64::consts::TAU * r / STEP_RING_M).ceil() as usize).max(6);
        for k in 0..n {
            let a = std::f64::consts::TAU * k as f64 / n as f64;
            let p = from + v2(a.cos(), a.sin()) * r;
            let tier = known.tier(p, target, rules, radius);
            if best.is_some_and(|(t, _, _)| t >= tier) {
                continue;
            }
            if stands(p) && clear(p) {
                best = Some((tier, r, p));
            }
        }
    }
    best.map(|(_, _, p)| p)
}
