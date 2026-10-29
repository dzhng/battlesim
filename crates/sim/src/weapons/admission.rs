//! Weapon table publication and flight admission.
use super::*;

/// Admit every row's flight and the weapon table.
pub fn check_rules(rules: &Rules) -> Result<FlightConfig, String> {
    let config = FlightConfig::new(&rules.physics.flight, rules.tick_hz)
        .map_err(|e| format!("physics: {e}"))?;
    for (id, weapon) in &rules.weapons {
        config
            .profile(&weapon.ballistics)
            .map_err(|e| format!("weapons.{id}: {e}"))?;
        for (layer, fire) in &weapon.targets {
            config
                .check_layer(&weapon.ballistics, fire.range_m, fire.scatter_mrad)
                .map_err(|e| format!("weapons.{id}.{layer:?}: {e}"))?;
        }
    }
    if rules.weapons.len() > crate::publication::MAX_WEAPON_ROWS {
        return Err(format!(
            "weapons: more than {} rows, the publication limit",
            crate::publication::MAX_WEAPON_ROWS
        ));
    }
    for t in rules.catalog.indices() {
        for mount in rules.catalog.mounts(t) {
            if mount.def.weapons.len() > crate::publication::MAX_AMMO_KINDS {
                return Err(format!(
                    "units.{}: mount {:?} has more than {} ammunition kinds, the publication limit",
                    rules.catalog.id(t),
                    mount.def.id,
                    crate::publication::MAX_AMMO_KINDS
                ));
            }
        }
    }
    Ok(config)
}
