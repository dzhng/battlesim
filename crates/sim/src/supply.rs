//! Recovery with finite stock. A fully deployed supply vehicle
//! serves its side's living units within its radius that stand still. Every
//! need runs at once, each on its own clock at its configured rate and paid
//! from the vehicle's stock: each finite ammunition kind, defensive charges
//! (active protection, flares), vehicle health, replacement soldiers, first
//! aid for wounded ones, and smoke charges. Stock is debited in ascending unit
//! order, then in that order of needs; its final positive remainder buys one
//! last item, never going below zero or refilling (L05). Readiness is the
//! deployment module's.
use contract::map::MoverClass;
use contract::observation::ServiceStatus;
use contract::scenario::Rules;
use contract::weapons::AmmoCapacity;

use crate::arrangement;
use crate::deployment;
use crate::garrison;
use crate::math::V2;
use crate::units::{Soldier, Unit};
use crate::weapons::Arsenal;
use crate::world::{Prop, WorldGeometry};

/// Seconds of service accumulated toward the next item of each kind; paused,
/// not lost, while the recipient is ineligible.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Progress {
    pub protection_s: f64,
    pub flare_s: f64,
    pub hp_s: f64,
    pub soldier_s: f64,
    pub first_aid_s: f64,
    pub smoke_s: f64,
}

/// An item a recipient can be served.
#[derive(Clone, Copy)]
enum Need {
    /// (mount, kind, weapon row).
    Round(usize, usize, usize),
    /// A protection charge, up to its capacity, each taking its service time.
    Protection {
        capacity: u32,
        service_s: f64,
        stock_per_charge: u32,
    },
    /// A flare, up to the unit's capacity, at the service rules' rate and
    /// price.
    Flare {
        capacity: u32,
    },
    Health,
    Soldier,
    /// A treatment for the squad's most wounded living soldier.
    FirstAid,
    /// A smoke charge, up to its capacity, at the capability's rate and price.
    Smoke {
        capacity: u32,
        service_s: f64,
        stock_per_charge: u32,
    },
}

/// Every item the unit is short of, in service order.
/// `room`: a replacement soldier has a seat in the squad's host, if it has one.
fn needs(unit: &Unit, room: bool, arsenal: &Arsenal, rules: &Rules) -> Vec<Need> {
    let specs = arsenal.specs(unit.kind);
    let t = unit.unit_type(rules);
    let mut needs = Vec::new();
    for (m, mount) in unit.mounts.iter().enumerate() {
        for (k, &row) in specs[mount.spec].kinds.iter().enumerate() {
            if let (AmmoCapacity::Rounds(cap), Some(n)) =
                (&arsenal.weapons[row].def.ammo, mount.ammo[k])
            {
                if n < *cap {
                    needs.push(Need::Round(m, k, row));
                }
            }
        }
    }
    if let (Some(state), Some(cap)) = (unit.protection, &t.capabilities.active_protection) {
        if state.charges < cap.capacity {
            needs.push(Need::Protection {
                capacity: cap.capacity,
                service_s: cap.service_s,
                stock_per_charge: cap.stock_per_charge,
            });
        }
    }
    if let (Some(state), Some(cap)) = (unit.flares, &t.capabilities.flares) {
        if state.charges < cap.capacity {
            needs.push(Need::Flare {
                capacity: cap.capacity,
            });
        }
    }
    if unit.hp < unit.max_hp(rules) {
        needs.push(Need::Health);
    }
    let living = unit.members.iter().filter(|s| s.alive()).count();
    if room && living < t.squad_size() {
        needs.push(Need::Soldier);
    }
    if most_wounded(unit, rules).is_some() {
        needs.push(Need::FirstAid);
    }
    if let (Some(state), Some(cap)) = (unit.smoke, &t.capabilities.smoke) {
        if state.charges < cap.capacity {
            needs.push(Need::Smoke {
                capacity: cap.capacity,
                service_s: cap.service_s,
                stock_per_charge: cap.stock_per_charge,
            });
        }
    }
    needs
}

/// The living soldier furthest below his type's health, the first of equals,
/// with that health; none when nobody is hurt.
fn most_wounded(unit: &Unit, rules: &Rules) -> Option<(usize, f64)> {
    let slots = unit.unit_type(rules).slots()?;
    unit.members
        .iter()
        .enumerate()
        .filter(|(_, s)| s.alive())
        .map(|(k, s)| (k, rules.catalog.soldier(&slots[s.slot]).hp, s.hp))
        .filter(|&(_, max, hp)| hp < max)
        .fold(None, |best: Option<(usize, f64, f64)>, c| match best {
            Some(b) if b.2 - b.1 <= c.2 - c.1 => Some(b),
            _ => Some(c),
        })
        .map(|(k, max, _)| (k, max))
}

/// The stock an item costs.
fn price(need: Need, arsenal: &Arsenal, rules: &Rules) -> u32 {
    let s = &rules.service;
    match need {
        Need::Round(_, _, row) => arsenal.weapons[row]
            .def
            .round_cost
            .expect("finite ammunition is priced at admission"),
        Need::Protection {
            stock_per_charge, ..
        } => stock_per_charge,
        Need::Flare { .. } => s.stock_per_flare,
        Need::Health => s.stock_per_hp,
        Need::Soldier => s.stock_per_soldier,
        Need::FirstAid => s.stock_per_first_aid,
        Need::Smoke {
            stock_per_charge, ..
        } => stock_per_charge,
    }
}

/// Every finite round has an authored price and positive delivery rate.
pub(crate) fn validate(rules: &Rules) -> Result<(), crate::RulesError> {
    for (id, weapon) in &rules.weapons {
        if !matches!(weapon.ammo, AmmoCapacity::Rounds(_)) {
            continue;
        }
        if weapon.round_cost.is_none() {
            return Err(crate::RulesError::new(
                format!("weapons.{id}.round_cost"),
                "missing price for finite weapon row",
            ));
        }
        if !weapon
            .resupply_rounds_per_s
            .is_some_and(|rate| rate.is_finite() && rate > 0.0)
        {
            return Err(crate::RulesError::new(
                format!("weapons.{id}.resupply_rounds_per_s"),
                "finite weapon row needs a finite positive resupply rate",
            ));
        }
    }
    Ok(())
}

/// Serve one tick. Recipients go in ascending unit order, so an earlier unit
/// pays first; each item takes the first set-up truck in reach with stock.
/// `next_soldier` issues fresh soldier ids for replacements.
pub fn service(
    world: &WorldGeometry,
    units: &mut [Unit],
    windows: &garrison::Windows,
    arsenal: &Arsenal,
    rules: &Rules,
    moved: &[bool],
    next_soldier: &mut u32,
) {
    let radius = rules.service.radius_m;
    let sources: Vec<usize> = (0..units.len())
        .filter(|&i| units[i].on_field() && units[i].stock.is_some())
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
        if unit.on_field() && unit.stock.is_none() && !reach.is_empty() {
            let ready: Vec<usize> = reach
                .into_iter()
                .filter(|&s| deployment::fully_deployed(&units[s]) && !moved[s])
                .collect();
            status = if ready.is_empty() {
                ServiceStatus::SourceNotDeployed
            } else if moved[r] || unit.movement_goal().is_some() {
                // Standing still means the unit's position did not change;
                // turning in place still counts as standing.
                ServiceStatus::Moving
            } else {
                let room = unit.hold.as_ref().is_none_or(|hold| {
                    crate::hold::Seats::of(world, units, hold.host, windows, rules).free() > 0
                });
                let items = needs(unit, room, arsenal, rules);
                let mut status = if items.is_empty() {
                    ServiceStatus::Full
                } else {
                    ServiceStatus::NoStock
                };
                for item in items {
                    let cost = price(item, arsenal, rules);
                    if let Some(src) = ready
                        .iter()
                        .copied()
                        .find(|&s| units[s].stock.is_some_and(|n| n > 0))
                    {
                        status = serve(
                            world,
                            units,
                            windows,
                            src,
                            r,
                            item,
                            cost,
                            arsenal,
                            rules,
                            next_soldier,
                        );
                    }
                }
                status
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
    windows: &garrison::Windows,
    src: usize,
    r: usize,
    need: Need,
    cost: u32,
    arsenal: &Arsenal,
    rules: &Rules,
    next_soldier: &mut u32,
) -> ServiceStatus {
    let s = &rules.service;
    let dt = 1.0 / rules.tick_hz as f64;
    let unit = &mut units[r];
    let progress = &mut unit.progress_service;
    let (clock, period) = match need {
        Need::Round(m, k, row) => (
            &mut unit.mounts[m].resupply_s[k],
            1.0 / arsenal.weapons[row]
                .def
                .resupply_rounds_per_s
                .expect("finite ammunition has a resupply rate at admission"),
        ),
        Need::Protection { service_s, .. } => (&mut progress.protection_s, service_s),
        Need::Flare { .. } => (&mut progress.flare_s, s.flare_service_s),
        Need::Health => (&mut progress.hp_s, 1.0 / s.vehicle_hp_per_s),
        Need::Soldier => (&mut progress.soldier_s, s.soldier_replacement_s),
        Need::FirstAid => (&mut progress.first_aid_s, s.first_aid_s),
        Need::Smoke { service_s, .. } => (&mut progress.smoke_s, service_s),
    };
    *clock += dt;
    if *clock + 1e-9 < period {
        return ServiceStatus::Serving;
    }
    *clock -= period;
    let stock = units[src].stock.as_mut().expect("a supply source");
    *stock = stock.saturating_sub(cost);
    let unit = &mut units[r];
    match need {
        Need::Round(m, k, _) => {
            if let Some(n) = unit.mounts[m].ammo[k].as_mut() {
                *n += 1;
            }
        }
        Need::Protection { capacity, .. } => {
            let state = unit.protection.as_mut().expect("a protection recipient");
            state.charges = (state.charges + 1).min(capacity);
        }
        Need::Flare { capacity } => {
            let state = unit.flares.as_mut().expect("a flare recipient");
            state.charges = (state.charges + 1).min(capacity);
        }
        Need::Health => unit.hp = (unit.hp + 1.0).min(unit.max_hp(rules)),
        Need::FirstAid => {
            let (k, max) = most_wounded(unit, rules).expect("a wounded soldier");
            let soldier = &mut unit.members[k];
            soldier.hp = (soldier.hp + s.first_aid_hp).min(max);
        }
        Need::Smoke { capacity, .. } => {
            let state = unit.smoke.as_mut().expect("a smoke recipient");
            state.charges = (state.charges + 1).min(capacity);
        }
        Need::Soldier => {
            // A new soldier (new id) joins his squad where it fights: at a
            // window nobody holds while it holds a building
            // (`Windows::free_seat`), else at the free spot nearest the
            // squad's middle, spaced from his squadmates. The fallen one's
            // record stays where it lies.
            let seat = windows.free_seat(units, r);
            let unit = &mut units[r];
            let at = match seat {
                Some((_, p)) => p,
                None => {
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
                    at.with_z(z)
                }
            };
            // He fills the first slot no living soldier holds.
            let t = unit.unit_type(rules);
            let slot = (0..t.squad_size())
                .find(|&k| !unit.members.iter().any(|s| s.alive() && s.slot == k))
                .expect("a squad short of soldiers has a free slot");
            let hp = rules.catalog.soldier(&t.slots().expect("a squad")[slot]).hp;
            *next_soldier += 1;
            unit.members.push(Soldier::new(*next_soldier, slot, at, hp));
            if let (Some(hold), Some((k, _))) = (unit.hold.as_mut(), seat) {
                hold.building_mut()
                    .expect("building hold")
                    .seats
                    .push(Some(k));
            }
            unit.settle();
        }
    }
    ServiceStatus::Serving
}
