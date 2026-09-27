//! Recovery with finite stock. A fully deployed supply vehicle
//! serves its side's living units within its radius that stand still and did
//! not fire this tick (L03, L04): finite ammunition first, then vehicle health,
//! then replacement soldiers, each at its configured rate and paid from the
//! vehicle's stock. Stock is debited whole and in ascending unit order, never
//! below zero, and never refilled (L05). Readiness is the deployment module's.
use std::collections::BTreeSet;

use contract::ids::UnitId;
use contract::map::MoverClass;
use contract::observation::ServiceStatus;
use contract::scenario::Rules;
use contract::weapons::AmmoCapacity;

use crate::arrangement;
use crate::deployment;
use crate::math::V2;
use crate::units::{max_hp, squad_size, Soldier, Unit};
use crate::weapons;
use crate::weapons::Arsenal;
use crate::world::{Prop, WorldGeometry};

/// Seconds of service accumulated toward the next item of each kind; paused,
/// not lost, while the recipient is ineligible.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Progress {
    pub ammo_s: f64,
    pub hp_s: f64,
    pub soldier_s: f64,
}

/// What a recipient needs next, in service order.
#[derive(Clone, Copy)]
enum Need {
    /// (mount, kind, weapon row).
    Round(usize, usize, usize),
    Health,
    Soldier,
}

fn need(unit: &Unit, arsenal: &Arsenal, rules: &Rules) -> Option<Need> {
    let specs = arsenal.specs(unit.kind);
    for (m, mount) in unit.mounts.iter().enumerate() {
        for (k, &row) in specs[mount.spec].kinds.iter().enumerate() {
            if let (AmmoCapacity::Rounds(cap), Some(n)) =
                (&arsenal.weapons[row].def.ammo, mount.ammo[k])
            {
                if n < *cap {
                    return Some(Need::Round(m, k, row));
                }
            }
        }
    }
    if unit.hull.is_some() && unit.hp < max_hp(unit.kind, rules) {
        return Some(Need::Health);
    }
    let living = unit.members.iter().filter(|s| s.alive()).count() as u32;
    if unit.hull.is_none() && living < squad_size(unit.kind, rules) {
        return Some(Need::Soldier);
    }
    None
}

/// The stock an item costs.
fn price(need: Need, arsenal: &Arsenal, rules: &Rules) -> u32 {
    let s = &rules.service;
    match need {
        Need::Round(_, _, row) => s.round_costs[&arsenal.weapons[row].name],
        Need::Health => s.stock_per_hp,
        Need::Soldier => s.stock_per_soldier,
    }
}

/// Every finite round must have a price, or a truck could give it away (L05).
pub fn validate(arsenal: &Arsenal, rules: &Rules) {
    for w in &arsenal.weapons {
        if matches!(w.def.ammo, AmmoCapacity::Rounds(_)) {
            assert!(
                rules.service.round_costs.contains_key(&w.name),
                "service.round_costs has no price for finite weapon row {}",
                w.name
            );
        }
    }
}

/// Serve one tick. Recipients go in ascending unit order, so an earlier unit
/// pays first; each takes the first set-up truck in reach that can pay.
/// `fired` holds the units that launched this tick; `next_soldier` issues
/// fresh soldier ids for replacements.
pub fn service(
    world: &WorldGeometry,
    units: &mut [Unit],
    arsenal: &Arsenal,
    rules: &Rules,
    moved: &[bool],
    fired: &BTreeSet<UnitId>,
    next_soldier: &mut u32,
) {
    let radius = rules.service.radius_m;
    let sources: Vec<usize> = (0..units.len())
        .filter(|&i| units[i].alive() && units[i].stock.is_some())
        .collect();
    for r in 0..units.len() {
        let unit = &units[r];
        // Supply vehicles are never serviced, by themselves or each other.
        let mut status = ServiceStatus::OutOfRange;
        let reach: Vec<usize> = sources
            .iter()
            .copied()
            .filter(|&s| {
                units[s].side == unit.side
                    && (units[s].position.xy() - unit.position.xy()).length() <= radius
            })
            .collect();
        if unit.alive() && unit.stock.is_none() && !reach.is_empty() {
            let ready: Vec<usize> = reach
                .into_iter()
                .filter(|&s| deployment::fully_deployed(&units[s]) && !moved[s])
                .collect();
            status = if ready.is_empty() {
                ServiceStatus::SourceNotDeployed
            } else if moved[r] || unit.movement_goal().is_some() {
                // Standing still means no movement at all, turning included.
                ServiceStatus::Moving
            } else if fired.contains(&unit.id) || weapons::engaged(unit) {
                ServiceStatus::Firing
            } else {
                match need(unit, arsenal, rules) {
                    None => ServiceStatus::Full,
                    Some(Need::Soldier) if unit.garrison.is_some() => ServiceStatus::Garrisoned,
                    Some(item) => {
                        let cost = price(item, arsenal, rules);
                        match ready
                            .into_iter()
                            .find(|&s| units[s].stock.is_some_and(|n| n >= cost))
                        {
                            // Never partly paid, never below zero: wait (L05).
                            None => ServiceStatus::NoStock,
                            Some(src) => {
                                serve(world, units, src, r, item, cost, rules, next_soldier)
                            }
                        }
                    }
                }
            };
        }
        units[r].service = status;
    }
}

/// Advance the recipient's next item and pay for it when it completes.
#[allow(clippy::too_many_arguments)]
fn serve(
    world: &WorldGeometry,
    units: &mut [Unit],
    src: usize,
    r: usize,
    need: Need,
    cost: u32,
    rules: &Rules,
    next_soldier: &mut u32,
) -> ServiceStatus {
    let s = &rules.service;
    let dt = 1.0 / rules.tick_hz as f64;
    let progress = &mut units[r].progress_service;
    let (clock, period) = match need {
        Need::Round(..) => (&mut progress.ammo_s, 1.0 / s.ammo_rounds_per_s),
        Need::Health => (&mut progress.hp_s, 1.0 / s.vehicle_hp_per_s),
        Need::Soldier => (&mut progress.soldier_s, s.soldier_replacement_s),
    };
    *clock += dt;
    if *clock + 1e-9 < period {
        return ServiceStatus::Serving;
    }
    *clock -= period;
    let stock = units[src].stock.as_mut().expect("a supply source");
    *stock -= cost;
    let unit = &mut units[r];
    match need {
        Need::Round(m, k, _) => {
            if let Some(n) = unit.mounts[m].ammo[k].as_mut() {
                *n += 1;
            }
        }
        Need::Health => unit.hp = (unit.hp + 1.0).min(max_hp(unit.kind, rules)),
        Need::Soldier => {
            // A new soldier (new id) joins at the free spot nearest the
            // squad's middle, spaced from his squadmates; the fallen one's
            // record stays where it lies.
            let centre = unit.position.xy();
            let radius = rules.physics.soldier_radius_m;
            let spacing = rules.infantry_movement.spacing_m;
            let solid = |p: &Prop| p.blocks(MoverClass::Infantry);
            let living: Vec<V2> = unit.member_positions().map(|p| p.xy()).collect();
            let search = arrangement::spread(&rules.infantry_movement, living.len() + 1);
            let at = arrangement::nearest_free(centre, search, |p| {
                living.iter().all(|q| (*q - p).length() >= spacing)
                    && arrangement::standing_room(world, p, radius, &solid)
                    && arrangement::reachable(world, centre, p, radius, &solid)
            })
            .unwrap_or(centre);
            let z = world
                .surface_at(at.x, at.y)
                .map_or(unit.position.z, |s| s.z);
            *next_soldier += 1;
            unit.members.push(Soldier::new(
                *next_soldier,
                at.with_z(z),
                rules.health.soldier,
            ));
            unit.settle();
        }
    }
    ServiceStatus::Serving
}
