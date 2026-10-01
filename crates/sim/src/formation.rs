//! Joint destination placement on the side's known map, without route search.
use std::collections::{BTreeMap, BTreeSet};

use crate::math::{v2, Rotation, V2};
use contract::ids::UnitId;
use contract::scenario::FormationRules;

pub struct Member {
    pub id: UnitId,
    pub position: V2,
    /// Settled footprint, independent of the unit's currently scattered pose.
    pub radius: f64,
    pub yaw: f64,
}

pub struct Slot {
    pub id: UnitId,
    pub point: Option<V2>,
}

pub struct Placement {
    pub slots: Vec<Slot>,
    pub radius_m: f64,
    pub checks: usize,
}

/// Centre buckets sized to the largest possible exclusion diameter. A
/// candidate can conflict only with its bucket or its eight neighbors.
struct Reservations {
    cell_m: f64,
    spacing_m: f64,
    buckets: BTreeMap<(i64, i64), Vec<(V2, f64)>>,
}

impl Reservations {
    fn new(max_radius: f64, spacing_m: f64) -> Self {
        Self {
            cell_m: 2.0 * max_radius + spacing_m,
            spacing_m,
            buckets: BTreeMap::new(),
        }
    }
    fn key(&self, p: V2) -> (i64, i64) {
        (
            (p.x / self.cell_m).floor() as i64,
            (p.y / self.cell_m).floor() as i64,
        )
    }
    fn free(&self, p: V2, radius: f64) -> bool {
        let (x, y) = self.key(p);
        (-1..=1).all(|dx| {
            (-1..=1).all(|dy| {
                self.buckets.get(&(x + dx, y + dy)).is_none_or(|members| {
                    members
                        .iter()
                        .all(|&(q, r)| (p - q).dot(p - q) >= (radius + r + self.spacing_m).powi(2))
                })
            })
        })
    }
    fn reserve(&mut self, p: V2, radius: f64) {
        self.buckets
            .entry(self.key(p))
            .or_default()
            .push((p, radius));
    }
}

/// Every lattice point once, in successive square rings; no RNG or retry loop.
fn spiral(n: usize) -> V2 {
    let ring = ((n as f64).sqrt() + 1.0).floor() as usize / 2;
    let side = 2 * ring;
    let k = n - (2 * ring - 1).pow(2);
    let r = ring as f64;
    match k / side {
        0 => v2(r, -r + k as f64),
        1 => v2(r - (k - side) as f64, r),
        2 => v2(-r, r - (k - 2 * side) as f64),
        _ => v2(-r + (k - 3 * side) as f64, -r),
    }
}

pub fn place(
    members: &[Member],
    anchor: V2,
    facing: Option<f64>,
    rules: &FormationRules,
    map_reach: f64,
    standing: impl Fn(UnitId, V2) -> Option<V2>,
) -> Placement {
    let mut seen = BTreeSet::new();
    let members: Vec<_> = members.iter().filter(|m| seen.insert(m.id)).collect();
    if members.is_empty() {
        return Placement {
            slots: Vec::new(),
            radius_m: 0.0,
            checks: 0,
        };
    }
    let max_radius = members.iter().map(|m| m.radius).fold(0.0, f64::max);
    // The front-center constraint leaves a rear half-disc available. Reserve
    // slack from its footprint demand rather than capping every group at 40 m.
    let demand = members
        .iter()
        .map(|m| (m.radius + rules.spacing_m / 2.0).powi(2))
        .sum::<f64>();
    let initial_radius = rules
        .min_radius_m
        .max(max_radius + (2.0 * demand / rules.fill_ratio).sqrt());
    let centre =
        members.iter().fold(v2(0.0, 0.0), |p, m| p + m.position) * (1.0 / members.len() as f64);
    let source = Rotation::new(-members[0].yaw);
    let spread = members
        .iter()
        .map(|m| (m.position - centre).length())
        .fold(0.0, f64::max);
    let scale = if spread > 0.0 {
        ((initial_radius - max_radius) / spread).min(1.0)
    } else {
        1.0
    };
    let local: Vec<_> = members
        .iter()
        .map(|m| source.apply(m.position - centre) * scale)
        .collect();
    let front = local
        .iter()
        .zip(&members)
        .map(|(p, m)| p.x + m.radius)
        .fold(f64::NEG_INFINITY, f64::max);
    let left = local
        .iter()
        .zip(&members)
        .map(|(p, m)| p.y - m.radius)
        .fold(f64::INFINITY, f64::min);
    let right = local
        .iter()
        .zip(&members)
        .map(|(p, m)| p.y + m.radius)
        .fold(f64::NEG_INFINITY, f64::max);
    let group = members.len() > 1;
    let pivot = if group {
        v2(front, (left + right) / 2.0)
    } else {
        v2(0.0, 0.0)
    };
    let rotation = Rotation::new(facing.unwrap_or(members[0].yaw));
    let inverse = Rotation::new(-facing.unwrap_or(members[0].yaw));
    let wanted: Vec<_> = local.iter().map(|&p| p - pivot).collect();
    let mut slots: Vec<_> = members
        .iter()
        .map(|m| Slot {
            id: m.id,
            point: None,
        })
        .collect();
    let mut order: Vec<_> = (0..members.len()).collect();
    order.sort_by(|&a, &b| {
        members[b]
            .radius
            .total_cmp(&members[a].radius)
            .then(a.cmp(&b))
    });
    let mut checks = 0;
    let mut radius = initial_radius;
    let max_extent = map_reach.max(initial_radius);
    let mut expansions = 0;
    let mut remaining = rules.candidate_checks as usize;
    let mut attempted = vec![false; members.len()];
    let mut holding = vec![false; members.len()];
    loop {
        let mut reservations = Reservations::new(max_radius, rules.spacing_m);
        for (i, member) in members.iter().enumerate() {
            let occupied = if holding[i] {
                Some(member.position)
            } else {
                slots[i].point
            };
            if let Some(p) = occupied {
                reservations.reserve(inverse.apply(p - anchor), member.radius);
            }
        }
        let unresolved: Vec<_> = order
            .iter()
            .copied()
            .filter(|&i| !holding[i] && slots[i].point.is_none())
            .collect();
        for (rank, &i) in unresolved.iter().enumerate() {
            let member = members[i];
            let valid = |p: V2, radius: f64, reservations: &Reservations| {
                if (group && p.x + member.radius > 1e-9)
                    || (p + pivot).length() + member.radius > radius + 1e-9
                    || !reservations.free(p, member.radius)
                {
                    return None;
                }
                let world = standing(member.id, anchor + rotation.apply(p))?;
                let p = inverse.apply(world - anchor);
                ((!group || p.x + member.radius <= 1e-9)
                    && (p + pivot).length() + member.radius <= radius + 1e-9
                    && reservations.free(p, member.radius))
                .then_some(p)
            };
            // Every member gets its first intended-point check. Repacking
            // around a newly held unit shares the same alternative allowance.
            if attempted[i] {
                if remaining == 0 {
                    holding[i] = true;
                    continue;
                }
                remaining -= 1;
            }
            attempted[i] = true;
            checks += 1;
            let mut placed = valid(wanted[i], radius, &reservations);
            // Retain work to repair earlier slots if this member has to hold.
            let repair_reserve = if members.len() > 1 {
                remaining / 2
            } else {
                remaining
            };
            let quota = remaining
                .div_ceil(unresolved.len() - rank)
                .min(repair_reserve);
            let step = reservations.cell_m.min(
                3.0 * (2.0 * member.radius + rules.spacing_m).max(crate::navigation::NAV_CELL_M),
            );
            for k in 0..quota {
                if placed.is_some() {
                    break;
                }
                checks += 1;
                remaining -= 1;
                let p = wanted[i] + spiral(k + 1) * step;
                let needed = (p + pivot).length() + member.radius;
                if !group || p.x + member.radius <= 1e-9 {
                    while needed > radius
                        && expansions < rules.max_expansions
                        && radius < max_extent
                    {
                        radius = (radius * rules.growth_factor).min(max_extent);
                        expansions += 1;
                    }
                }
                placed = valid(p, radius, &reservations);
            }
            if let Some(p) = placed {
                reservations.reserve(p, member.radius);
                slots[i].point = Some(anchor + rotation.apply(p));
            } else {
                holding[i] = true;
            }
        }
        // A failed member stays at its source position. Preserve unaffected
        // slots and only reconsider destinations intersecting these held bodies.
        let mut held = Reservations::new(max_radius, rules.spacing_m);
        for (i, member) in members.iter().enumerate().filter(|(i, _)| holding[*i]) {
            held.reserve(inverse.apply(member.position - anchor), members[i].radius);
        }
        let mut reconsider = false;
        for (i, slot) in slots.iter_mut().enumerate() {
            if slot
                .point
                .is_some_and(|p| !held.free(inverse.apply(p - anchor), members[i].radius))
            {
                slot.point = None;
                reconsider = true;
            }
        }
        if !reconsider {
            break;
        }
    }
    Placement {
        slots,
        radius_m: radius,
        checks,
    }
}
