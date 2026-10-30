//! Visible mount pose and owner readiness.
use super::*;
use contract::observation::{MountReadiness, WeaponPose};

/// A mount's pose for animation: published for own units and identified enemies.
pub fn pose(mount: &Mount) -> WeaponPose {
    WeaponPose {
        mount: mount.spec as u8,
        operator: mount.operator,
        bearing: mount.bearing,
        elevation: mount.elevation,
        round: mount.round,
        shots: mount.shots,
    }
}

/// Exported readiness of a mount for its owner's panel and rings.
pub fn readiness(
    arsenal: &Arsenal,
    unit: &Unit,
    mount: &Mount,
    target_ref: Option<TargetRef>,
) -> MountReadiness {
    let spec = &arsenal.specs(unit.kind)[mount.spec];
    let loaded = mount.cycles.iter().find_map(Cycle::ready);
    let reloading = if loaded.is_some() {
        None
    } else {
        mount.cycles.iter().filter_map(|c| c.reload).min_by(|a, b| {
            let remaining = |(k, progress): (usize, f64)| {
                arsenal.weapons[spec.kinds[k]].def.reload_s - progress
            };
            remaining(*a).total_cmp(&remaining(*b))
        })
    };
    let aim = mount
        .cycles
        .iter()
        .filter(|c| cycle_active(unit, mount, c))
        .map(|c| {
            let k = c.loaded.or(c.reload.map(|(k, _)| k)).unwrap_or(0);
            (c.aim / arsenal.weapons[spec.kinds[k]].def.aim_s).min(1.0)
        })
        .fold(0.0, f64::max);
    let reload = reloading.map_or(0.0, |(k, p)| {
        (p / arsenal.weapons[spec.kinds[k]].def.reload_s).min(1.0)
    });
    MountReadiness {
        mount: mount.spec as u8,
        loaded: loaded.map(|k| k as u8),
        reloading: reloading.map(|(k, _)| k as u8),
        ammo: mount.ammo.clone(),
        aim,
        reload,
        target: target_ref,
        reason: mount.reason,
        guiding: mount.support.is_some(),
    }
}
