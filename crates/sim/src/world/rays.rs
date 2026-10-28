//! What a line meets: a round's flight and an eye's sight against the
//! terrain, the water's surface and the bodies that stop or hide.
use super::{Prop, PropId, WorldGeometry};
use crate::math::{v2, v3, V2, V3};
use contract::catalog::PropBody;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Collider {
    Terrain,
    Prop(PropId),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub t: f64,
    pub point: V3,
    pub normal: V3,
    pub collider: Collider,
}

impl WorldGeometry {
    /// Earliest hit of a round along `origin + dir * t`, t ∈ [0, max_t]: the
    /// terrain and every body that stops rounds (flight's colliders, Q28).
    /// `dir` should be unit length for `t` to be metres. Ties resolve to the
    /// terrain, then the lowest prop id.
    pub fn raycast(&self, origin: V3, dir: V3, max_t: f64) -> Option<Hit> {
        self.raycast_by(origin, dir, max_t, None, |b| b.stops_rounds)
    }

    /// [`raycast`](Self::raycast) passing through `past`: the body a
    /// soldier fires from behind.
    pub fn raycast_past(
        &self,
        origin: V3,
        dir: V3,
        max_t: f64,
        past: Option<PropId>,
    ) -> Option<Hit> {
        self.raycast_by(origin, dir, max_t, past, |b| b.stops_rounds)
    }

    /// The destroyable bodies a round flies into along `origin + dir * t`,
    /// t ∈ (0, max_t], without being stopped: rows with integrity that do not
    /// stop rounds. Appends (t, id) in order of t, then id. A segment
    /// starting inside a body does not enter it again, so a round flying
    /// chord by chord meets each body once. `past` is never met.
    pub fn passes(
        &self,
        origin: V3,
        dir: V3,
        max_t: f64,
        past: Option<PropId>,
        out: &mut Vec<(f64, PropId)>,
    ) {
        let first = out.len();
        let mut ids = Vec::new();
        self.props
            .along(origin.xy(), (origin + dir * max_t).xy(), &mut ids);
        for id in ids
            .into_iter()
            .filter(|&id| !self.skips_structure(id, past))
        {
            let prop = self.props.get(id).expect("indexed prop is live");
            if prop.body.stops_rounds
                || prop.body.hp.is_none()
                || !track_near(origin, dir, max_t, prop)
            {
                continue;
            }
            if let Some((t, _)) = prop.raycast(origin, dir, max_t) {
                if t > 0.0 {
                    out.push((t, id));
                }
            }
        }
        out[first..].sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    }

    /// [`raycast`](Self::raycast) against the terrain and the props whose
    /// row `admits`, passing through `skip`.
    fn raycast_by(
        &self,
        origin: V3,
        dir: V3,
        max_t: f64,
        skip: Option<PropId>,
        admits: impl Fn(&PropBody) -> bool,
    ) -> Option<Hit> {
        let ground = self.field.raycast(origin, dir, max_t);
        let water = self
            .water_hit(origin, dir, max_t)
            .filter(|&t| ground.is_none_or(|(ground, _)| t < ground))
            .map(|t| (t, v3(0.0, 0.0, 1.0)));
        let mut best: Option<Hit> = water.or(ground).map(|(t, normal)| Hit {
            t,
            point: origin + dir * t,
            normal,
            collider: Collider::Terrain,
        });
        // Bodies are offered as the index holds them (each whose footprint
        // circle the track passes, perhaps more than once): the nearest hit
        // wins, a body only before the ground strictly nearer, and among
        // bodies hit at the same distance the lowest id.
        let (a, b) = (origin.xy(), (origin + dir * max_t).xy());
        self.props.any_along(a, b, 0.0, |id| {
            if self.skips_structure(id, skip) {
                return false;
            }
            let prop = self.props.get(id).expect("indexed prop is live");
            if !admits(&prop.body) {
                return false;
            }
            if let Some((t, normal)) = prop.raycast(origin, dir, max_t) {
                let nearer = best.is_none_or(|h| {
                    t < h.t || (t == h.t && matches!(h.collider, Collider::Prop(o) if id < o))
                });
                if nearer {
                    best = Some(Hit {
                        t,
                        point: origin + dir * t,
                        normal,
                        collider: Collider::Prop(id),
                    });
                }
            }
            false
        });
        best
    }

    /// Whether a round could fly the straight segment between two points: it
    /// meets neither the terrain nor a body that stops rounds.
    pub fn segment_clear(&self, a: V3, b: V3) -> bool {
        self.segment_clear_except(a, b, None)
    }

    /// [`segment_clear`](Self::segment_clear) ignoring one prop: an occupied
    /// building shelters its occupants through cover, not as a second wall.
    pub fn segment_clear_except(&self, a: V3, b: V3, skip: Option<PropId>) -> bool {
        let d = b - a;
        let len = d.length();
        len == 0.0 || !self.blocked_by(a, d * (1.0 / len), len, skip, |b| b.stops_rounds)
    }

    /// Whether an eye at `a` sees `b`: neither the terrain nor a body that
    /// occludes lies between (Q25: sensing's line of sight).
    pub fn sight_clear(&self, a: V3, b: V3) -> bool {
        let d = b - a;
        let len = d.length();
        len == 0.0
            || !(self.blocked_by(a, d * (1.0 / len), len, None, |b| b.occludes)
                || self.squeezed(a, b))
    }

    /// Whether the line from `a` to `b` squeezes between bodies that hide
    /// what lies behind them, one on its left and one on its right, with
    /// less than `min_sight_gap_m` of room on its two sides together:
    /// through the crack between two houses, or the sliver between a near
    /// house on one side and a far one on the other. A body that stands
    /// within half that of either end is passed, so one at a wall or in a
    /// crack still sees and is seen. Where the line passes a box it misses
    /// most closely is a corner; the body counts where it stands above the
    /// line.
    fn squeezed(&self, a: V3, b: V3) -> bool {
        let least = self.min_sight_gap_m;
        let (from, to) = (a.xy(), b.xy());
        let len = (to - from).length();
        if least <= 0.0 || len <= least {
            return false;
        }
        let dir = (to - from) * (1.0 / len);
        let (mut left, mut right) = (f64::INFINITY, f64::INFINITY);
        self.props.any_along(from, to, least, |id| {
            let prop = self.props.get(id).expect("indexed prop is live");
            let f = prop.footprint();
            if !prop.body.occludes || f.distance(from) < least / 2.0 || f.distance(to) < least / 2.0
            {
                return false;
            }
            let (x, y) = (v2(1.0, 0.0).rotated(f.yaw), v2(0.0, 1.0).rotated(f.yaw));
            let mut nearest: Option<(f64, f64)> = None;
            let mut side = 0.0;
            for (sx, sy) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                let v = f.center + x * (sx * f.half.x) + y * (sy * f.half.y) - from;
                let t = v.dot(dir);
                if t <= 0.0 || t >= len {
                    continue;
                }
                let off = dir.x * v.y - dir.y * v.x;
                if side * off < 0.0 {
                    // Corners either side: the line crosses the box, which
                    // the body test judges.
                    return false;
                }
                side = off.signum();
                if nearest.is_none_or(|(o, _)| off.abs() < o) {
                    nearest = Some((off.abs(), t));
                }
            }
            if let Some((off, t)) = nearest {
                let z = a.z + (b.z - a.z) * (t / len);
                if prop.top_z() > z {
                    let room = if side > 0.0 { &mut left } else { &mut right };
                    *room = room.min(off);
                }
            }
            left + right < least
        })
    }

    /// Whether [`raycast_by`](Self::raycast_by) would hit anything, without
    /// finding the nearest hit: the first prop found ends the search, only
    /// props whose footprint circle the segment meets are ray-tested, and
    /// the terrain is only searched when no prop is met.
    fn blocked_by(
        &self,
        origin: V3,
        dir: V3,
        max_t: f64,
        skip: Option<PropId>,
        admits: impl Fn(&PropBody) -> bool,
    ) -> bool {
        // A hit lies on the segment inside the footprint, so within its
        // circle: the index offers every prop that could be hit.
        let (a, b) = (origin.xy(), (origin + dir * max_t).xy());
        let by_prop = self.props.any_along(a, b, 0.0, |id| {
            let prop = self.props.get(id).expect("indexed prop is live");
            !self.skips_structure(id, skip)
                && admits(&prop.body)
                && prop.raycast(origin, dir, max_t).is_some()
        });
        by_prop
            || self.field.raycast(origin, dir, max_t).is_some()
            || self.water_hit(origin, dir, max_t).is_some()
    }

    /// Where a line falling along `origin + dir * t`, t ∈ [0, max_t], meets
    /// standing water's surface, a river's or a swamp's: water stops a round
    /// as the ground does, so a shell bursts on the water, not on its bed.
    fn water_hit(&self, origin: V3, dir: V3, max_t: f64) -> Option<f64> {
        if dir.z >= 0.0 {
            return None;
        }
        let meets = |surface_z: f64, wet: &dyn Fn(V2) -> bool| {
            let t = (surface_z - origin.z) / dir.z;
            let p = origin + dir * t;
            ((0.0..=max_t).contains(&t) && self.field.contains(p.x, p.y) && wet(p.xy()))
                .then_some(t)
        };
        let surfaces = &self.surfaces;
        let rivers = surfaces
            .rivers()
            .iter()
            .enumerate()
            .filter_map(|(river, r)| meets(r.surface_z(), &|p| surfaces.in_river(river, p)));
        let swamps = (0..surfaces.swamps().len()).filter_map(|swamp| {
            meets(surfaces.swamp_surface_z(swamp), &|p| {
                surfaces.in_swamp(swamp, p)
            })
        });
        rivers.chain(swamps).min_by(f64::total_cmp)
    }
}

/// Whether the ground track of `origin + dir * t`, t ∈ [0, max_t], comes
/// within `prop`'s footprint circle (its half-sides bound the half-diagonal
/// without a square root): a ray whose track stays outside it cannot meet
/// the box, so the slab test is skipped for most bodies a ray's buckets hold.
fn track_near(origin: V3, dir: V3, max_t: f64, prop: &Prop) -> bool {
    let (a, ab) = (origin.xy(), dir.xy() * max_t);
    let ac = prop.center - a;
    let t = (ac.dot(ab) / ab.dot(ab).max(1e-300)).clamp(0.0, 1.0);
    let off = ab * t - ac;
    let reach = prop.half.x.abs() + prop.half.y.abs();
    off.dot(off) <= reach * reach * (1.0 + 1e-9) + 1e-9
}
