//! Who works each single gun of an infantry squad.
use super::{Arsenal, Mount, MountSpec};
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
    let needs = |m: &Mount| !specs[m.spec].squad && (!m.out_of_ammo() || m.support.is_some());
    let guns: Vec<_> = unit
        .mounts
        .iter()
        .map(|m| needs(m).then(|| &specs[m.spec]))
        .collect();
    let members: Vec<_> = unit
        .members
        .iter()
        .map(|s| s.alive().then_some(s.slot))
        .collect();
    let mut assigned: Vec<_> = original_carriers(&guns, &members)
        .into_iter()
        .map(|k| k.map(|k| unit.members[k].id))
        .collect();
    let mut used: Vec<_> = assigned.iter().flatten().copied().collect();
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

/// Each single gun's first claim, by member index: every gun `wanted` goes to
/// the first living member whose slot carries it and who holds no other gun
/// yet. `members` holds each member's slot, `None` once he has fallen. Squad
/// guns, and guns not wanted, go to no one.
pub fn original_carriers(
    guns: &[Option<&MountSpec>],
    members: &[Option<usize>],
) -> Vec<Option<usize>> {
    let mut used = vec![false; members.len()];
    guns.iter()
        .map(|gun| {
            let spec = gun.filter(|spec| !spec.squad)?;
            let k = members.iter().enumerate().position(|(k, slot)| {
                !used[k] && slot.is_some_and(|slot| spec.carriers.contains(&slot))
            })?;
            used[k] = true;
            Some(k)
        })
        .collect()
}
