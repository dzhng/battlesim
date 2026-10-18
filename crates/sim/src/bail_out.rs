//! Forced exits from a destroyed host. Each event owns its random stream so
//! a bail-out cannot perturb combat damage or any later damage decision.

use contract::ids::Tick;
use contract::random::{mix, Rng};
use contract::scenario::{BailOutHost, BailOutRow, Rules};

use crate::arrangement;
use crate::hold::Host;
use crate::math::{V2, V3};
use crate::units::{Soldier, Unit};
use crate::world::{PropId, WorldGeometry};

/// Stream tag reserved for forced-exit events.
pub const BAIL_OUT_STREAM: u64 = 0x6261_696c_6f75_7421;

impl Host {
    fn key(self) -> u64 {
        match self {
            Self::Building(id) => u64::from(id),
            Self::Carrier(id) => (1 << 32) | u64::from(id.0),
        }
    }
}

/// Select the row for the host's physical kind. Callers decide whether a hull
/// is ground or air; the hold host only supplies event identity.
pub fn row(rules: &Rules, kind: BailOutHost) -> &BailOutRow {
    rules.bail_out.get(&kind).expect("bail-out rule")
}

pub struct Occupancy<'a> {
    pub radius: f64,
    pub hulls: &'a [crate::math::Obb2],
    pub soldiers: &'a [V2],
    pub ignored_props: &'a [PropId],
}

/// How a squad came out of its host: how many of its soldiers got out alive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Result {
    pub out: usize,
}

impl Result {
    pub fn all_died(&self) -> bool {
        self.out == 0
    }
}

/// Apply one host's bail-out to a squad. The caller clears the hold and orders
/// after this returns; this function owns health, corpses and spots.
pub fn apply(
    world: &WorldGeometry,
    unit: &mut Unit,
    host: Host,
    rule: &BailOutRow,
    seed: u64,
    tick: Tick,
    occupancy: Occupancy<'_>,
) -> Result {
    // Each squad draws its injuries from its own stream within the event, so
    // two squads leaving one host are hurt independently.
    let event = mix(seed ^ BAIL_OUT_STREAM ^ host.key() ^ tick);
    let mut rng = Rng::new(mix(event ^ u64::from(unit.id.0)));
    let mut out = 0;
    let squad_loss = rule.squad_loss.is_some_and(|chance| {
        // A carrier event owns one independent squad-level draw per passenger
        // squad, apart from its injury draws below.
        let mut squad_rng = Rng::new(mix(seed
            ^ BAIL_OUT_STREAM
            ^ host.key()
            ^ tick
            ^ u64::from(unit.id.0)));
        squad_rng.unit() < chance
    });
    let mut taken = Vec::new();
    for soldier in unit.members.iter_mut() {
        if !soldier.alive() {
            continue;
        }
        let at = soldier.position;
        if squad_loss {
            fall(world, soldier, at, unit.yaw);
            continue;
        }
        if rng.unit() < rule.death {
            fall(world, soldier, at, unit.yaw);
            continue;
        }
        let hp_lost = rule.hp_lost[0] + (rule.hp_lost[1] - rule.hp_lost[0]) * rng.unit();
        soldier.hp -= hp_lost;
        if !soldier.alive() {
            fall(world, soldier, at, unit.yaw);
            continue;
        }
        let occupied: Vec<V2> = occupancy
            .soldiers
            .iter()
            .copied()
            .chain(taken.iter().copied())
            .filter(|p| (*p - at.xy()).length() > f64::EPSILON)
            .collect();
        let Some(place) = arrangement::forced_exit_spot(
            world,
            at.xy(),
            rule.search_m,
            occupancy.radius,
            occupancy.hulls,
            &occupied,
            occupancy.ignored_props,
        ) else {
            fall(world, soldier, at, unit.yaw);
            continue;
        };
        taken.push(place.xy());
        soldier.position = place;
        out += 1;
    }
    Result { out }
}

/// A host that leaves the map takes its passengers with it: there is no
/// ground to bail out onto, so every live member dies where the host was.
pub fn lost_with_host(unit: &mut Unit, at: V3) -> Result {
    let yaw = unit.yaw;
    for soldier in unit.members.iter_mut().filter(|soldier| soldier.alive()) {
        soldier.fall(at, yaw, None);
    }
    Result { out: 0 }
}

fn fall(world: &WorldGeometry, soldier: &mut Soldier, at: V3, yaw: f64) {
    soldier.fall(
        at.xy()
            .with_z(world.surface_at(at.x, at.y).map_or(0.0, |s| s.z)),
        yaw,
        None,
    );
}
