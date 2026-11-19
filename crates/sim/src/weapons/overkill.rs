//! Committed fire: the side's flagged rounds on their way to each target,
//! and whether a shot would only add to a kill they already cover.
use std::collections::BTreeMap;

use contract::ids::{Side, UnitId};
use contract::scenario::Rules;
use contract::weapons::WeaponDefinition;

use crate::damage::pierce_share;
use crate::math::V3;
use crate::units::Unit;
use crate::weapons::{FireContext, MountSpec, Ready, Target};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub arrive_tick: f64,
    pub damage: f64,
    pub interceptable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CountedHit {
    arrive_tick: f64,
    damage: f64,
    interceptable: bool,
    seq: u64,
}

/// Counted rounds, grouped by the side that fired and their target.
#[derive(Clone, Debug, Default)]
pub struct Committed {
    hits: BTreeMap<(Side, UnitId), Vec<CountedHit>>,
    next_seq: u64,
}

impl Committed {
    /// Add a counted round; equal arrivals retain insertion order.
    pub fn add(&mut self, side: Side, target: UnitId, hit: Hit) {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        let list = self.hits.entry((side, target)).or_default();
        list.push(CountedHit {
            arrive_tick: hit.arrive_tick,
            damage: hit.damage,
            interceptable: hit.interceptable,
            seq,
        });
        list.sort_by(|a, b| {
            a.arrive_tick
                .total_cmp(&b.arrive_tick)
                .then_with(|| a.seq.cmp(&b.seq))
        });
    }

    /// Count damage arriving by `arrive_tick` after active protection absorbs
    /// the earliest interceptable counted rounds.
    fn landing_by(&self, side: Side, target: UnitId, arrive_tick: f64, charges: u32) -> f64 {
        let Some(hits) = self.hits.get(&(side, target)) else {
            return 0.0;
        };
        let mut remaining_charges = charges;
        hits.iter()
            .filter(|hit| {
                if hit.interceptable && remaining_charges > 0 {
                    remaining_charges -= 1;
                    false
                } else {
                    true
                }
            })
            .filter(|hit| hit.arrive_tick <= arrive_tick)
            .map(|hit| hit.damage)
            .sum()
    }
}

/// A vehicle's health, or the health of its living soldiers.
pub fn remaining_hp(unit: &Unit) -> f64 {
    if unit.hull.is_some() {
        unit.hp
    } else {
        squad_hp(&unit.members)
    }
}

fn squad_hp(members: &[crate::units::Soldier]) -> f64 {
    members
        .iter()
        .filter(|soldier| soldier.alive())
        .map(|soldier| soldier.hp)
        .sum()
}

/// Damage after penetration on the face the target shows the incoming round.
pub fn hit_damage(def: &WeaponDefinition, rules: &Rules, target: &Unit, from: V3) -> f64 {
    let Some(armor) = target.armor(rules) else {
        return def.damage;
    };
    let face = target.hull_face(from);
    def.damage * pierce_share(def, 0, rules, armor, face)
}

/// Whether committed damage landing by this shot's arrival destroys `target`.
pub fn doomed(committed: &Committed, side: Side, target: &Unit, arrive_tick: f64) -> bool {
    let charges = target.protection.as_ref().map_or(0, |state| state.charges);
    committed.landing_by(side, target.id, arrive_tick, charges) >= remaining_hp(target)
}

/// Whether a flagged kind's shot at `target`, landing after its time of
/// flight, would only add to damage already committed to kill it.
pub(super) fn overkill(
    ctx: &FireContext,
    units: &[Unit],
    side: Side,
    spec: &MountSpec,
    ready: &Ready,
    target: Target,
) -> bool {
    let Target::Unit(target) = target else {
        return false;
    };
    ctx.arsenal.weapons[spec.kinds[ready.kind]]
        .def
        .avoids_overkill
        && doomed(
            ctx.committed,
            side,
            &units[target.0 as usize],
            ctx.tick as f64 + ready.time_of_flight_s * f64::from(ctx.rules.tick_hz),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(arrive_tick: f64, damage: f64, interceptable: bool) -> Hit {
        Hit {
            arrive_tick,
            damage,
            interceptable,
        }
    }

    #[test]
    fn landing_cutoff_includes_ties_and_excludes_later_hits() {
        let mut committed = Committed::default();
        committed.add(Side::Blue, UnitId(1), hit(3.0, 2.0, false));
        committed.add(Side::Blue, UnitId(1), hit(4.0, 5.0, false));
        assert_eq!(committed.landing_by(Side::Blue, UnitId(1), 3.0, 0), 2.0);
    }

    #[test]
    fn trophy_absorbs_earliest_interceptable_hits_only() {
        let mut committed = Committed::default();
        committed.add(Side::Blue, UnitId(1), hit(1.0, 2.0, true));
        committed.add(Side::Blue, UnitId(1), hit(1.0, 11.0, true));
        committed.add(Side::Blue, UnitId(1), hit(2.0, 3.0, false));
        committed.add(Side::Blue, UnitId(1), hit(3.0, 5.0, true));
        committed.add(Side::Blue, UnitId(1), hit(4.0, 7.0, true));
        assert_eq!(committed.landing_by(Side::Blue, UnitId(1), 4.0, 1), 26.0);
        assert_eq!(committed.landing_by(Side::Blue, UnitId(1), 4.0, 5), 3.0);
    }

    #[test]
    fn squad_health_sums_living_soldiers() {
        let members = vec![
            crate::units::Soldier::new(1, 0, V3::default(), 4.0),
            crate::units::Soldier::new(2, 1, V3::default(), 0.0),
            crate::units::Soldier::new(3, 2, V3::default(), 6.0),
        ];
        assert_eq!(squad_hp(&members), 10.0);
    }
}
