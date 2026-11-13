//! Who works each single gun of an infantry squad.
use super::{Arsenal, Mount};
use crate::units::Unit;

/// Assign single infantry guns exclusively. Living original carriers get
/// their own weapon, then survivors keep handoffs, then free soldiers take
/// stocked spares in mount order. An empty launcher stays operated only while
/// it guides its last missile; after that its soldier can pick up a spare.
pub fn assign_operators(arsenal: &Arsenal, unit: &mut Unit) {
    if unit.hull.is_some() {
        return;
    }
    let specs = arsenal.specs(unit.kind);
    let mut assigned = vec![None; unit.mounts.len()];
    let mut used = Vec::new();
    let needs = |m: &Mount| !specs[m.spec].squad && (!m.out_of_ammo() || m.support.is_some());
    for (i, mount) in unit.mounts.iter().enumerate().filter(|(_, m)| needs(m)) {
        if let Some(s) = unit.members.iter().find(|s| {
            s.alive() && specs[mount.spec].carriers.contains(&s.slot) && !used.contains(&s.id)
        }) {
            assigned[i] = Some(s.id);
            used.push(s.id);
        }
    }
    for (i, mount) in unit
        .mounts
        .iter()
        .enumerate()
        .filter(|(_, m)| needs(m) && specs[m.spec].special)
    {
        if assigned[i].is_none() {
            if let Some(s) = unit
                .members
                .iter()
                .find(|s| s.alive() && Some(s.id) == mount.operator && !used.contains(&s.id))
            {
                assigned[i] = Some(s.id);
                used.push(s.id);
            }
        }
    }
    for (i, _) in unit
        .mounts
        .iter()
        .enumerate()
        .filter(|(_, m)| needs(m) && specs[m.spec].special)
    {
        if assigned[i].is_none() {
            if let Some(s) = unit
                .members
                .iter()
                .find(|s| s.alive() && !used.contains(&s.id))
            {
                assigned[i] = Some(s.id);
                used.push(s.id);
            }
        }
    }
    for (mount, operator) in unit.mounts.iter_mut().zip(assigned) {
        mount.operator = operator;
    }
}
