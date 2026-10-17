//! Forced exits from a destroyed host.  Each event owns its random stream so
//! a bail-out cannot perturb combat damage or any later damage decision.

use contract::ids::Tick;
use contract::random::{mix, Rng};
use contract::scenario::BailOutRow;

use crate::arrangement;
use crate::math::{V2, V3};
use crate::units::Unit;
use crate::world::{PropId, WorldGeometry};

/// Stream tag reserved for forced-exit events.
pub const BAIL_OUT_STREAM: u64 = 0x6261_696c_6f75_7421;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    Building(PropId),
    Ground(u32),
    Air(u32),
}

impl Host {
    fn key(self) -> u64 {
        match self {
            Host::Building(id) => u64::from(id),
            Host::Ground(id) => u64::from(id),
            Host::Air(id) => u64::from(id),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeathCause {
    SquadLost,
    Roll,
    NoRoom,
    LeftMap,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fate {
    Out { member: usize, at: V3, hp_lost: f64 },
    Died { member: usize, cause: DeathCause },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Result {
    pub fates: Vec<Fate>,
}

impl Result {
    pub fn all_died(&self) -> bool {
        self.fates.iter().all(|f| matches!(f, Fate::Died { .. }))
    }
}

/// Apply one host's bail-out to a squad.  The caller clears the hold and
/// orders after this returns; this function owns health, corpses and spots.
#[allow(clippy::too_many_arguments)]
pub fn apply(
    world: &WorldGeometry,
    unit: &mut Unit,
    host: Host,
    _from: V3,
    rule: &BailOutRow,
    seed: u64,
    tick: Tick,
    radius: f64,
    hulls: &[crate::math::Obb2],
    soldiers: &[V2],
) -> Result {
    let mut rng = Rng::new(mix(seed ^ BAIL_OUT_STREAM ^ host.key() ^ tick));
    let mut fates = Vec::new();
    let squad_loss = rule.squad_loss.is_some_and(|chance| rng.unit() < chance);
    let mut taken = Vec::new();
    for (member, soldier) in unit.members.iter_mut().enumerate() {
        if !soldier.alive() {
            continue;
        }
        let at = soldier.position;
        if squad_loss {
            soldier.fall(
                at.xy()
                    .with_z(world.surface_at(at.x, at.y).map_or(0.0, |s| s.z)),
                unit.yaw,
                None,
            );
            fates.push(Fate::Died {
                member,
                cause: DeathCause::SquadLost,
            });
            continue;
        }
        if rng.unit() < rule.death {
            soldier.fall(
                at.xy()
                    .with_z(world.surface_at(at.x, at.y).map_or(0.0, |s| s.z)),
                unit.yaw,
                None,
            );
            fates.push(Fate::Died {
                member,
                cause: DeathCause::Roll,
            });
            continue;
        }
        let hp_lost = rule.hp_lost[0] + (rule.hp_lost[1] - rule.hp_lost[0]) * rng.unit();
        soldier.hp -= hp_lost;
        if !soldier.alive() {
            soldier.fall(
                at.xy()
                    .with_z(world.surface_at(at.x, at.y).map_or(0.0, |s| s.z)),
                unit.yaw,
                None,
            );
            fates.push(Fate::Died {
                member,
                cause: DeathCause::Roll,
            });
            continue;
        }
        let occupied: Vec<V2> = soldiers
            .iter()
            .copied()
            .chain(taken.iter().copied())
            .filter(|p| (*p - at.xy()).length() > f64::EPSILON)
            .collect();
        let Some(place) =
            arrangement::forced_exit_spot(world, at.xy(), rule.search_m, radius, hulls, &occupied)
        else {
            soldier.fall(
                at.xy()
                    .with_z(world.surface_at(at.x, at.y).map_or(0.0, |s| s.z)),
                unit.yaw,
                None,
            );
            fates.push(Fate::Died {
                member,
                cause: DeathCause::NoRoom,
            });
            continue;
        };
        taken.push(place.xy());
        soldier.position = place;
        fates.push(Fate::Out {
            member,
            at: place,
            hp_lost,
        });
    }
    Result { fates }
}
