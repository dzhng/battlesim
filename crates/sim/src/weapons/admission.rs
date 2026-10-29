//! Weapon table publication and flight admission.
use super::*;
use crate::flight::FlightConfigError;
use crate::RulesError;

/// Admit every row's flight and the weapon table.
pub fn check_rules(rules: &Rules) -> Result<FlightConfig, RulesError> {
    let config = FlightConfig::new(&rules.physics.flight, rules.tick_hz).map_err(|e| {
        let path = match &e {
            FlightConfigError::NotPositive("tick_hz") => "tick_hz".to_owned(),
            FlightConfigError::NotPositive(field) => format!("physics.{field}"),
            _ => "physics.max_subsegments_per_tick".to_owned(),
        };
        RulesError::new(path, e.to_string())
    })?;
    let can_crash = rules.catalog.indices().any(|index| {
        let unit = rules.catalog.get(index);
        unit.hull().is_some() && matches!(unit.mobility, contract::catalog::Mobility::Air { .. })
    });
    if can_crash && !rules.weapons.contains_key("helicopter_crash") {
        return Err(RulesError::new(
            "weapons.helicopter_crash",
            "missing crash row",
        ));
    }
    for (id, weapon) in &rules.weapons {
        let profile = config
            .profile(&weapon.ballistics)
            .map_err(|e| flight_error(&format!("weapons.{id}"), e))?;
        if profile.turn_rad_s.is_some() {
            let reach = profile
                .motor
                .map_or(profile.speed_mps * profile.lifetime_s, |motor| {
                    motor.distance(profile.speed_mps, profile.lifetime_s)
                });
            if reach < weapon.ballistics.range_m {
                return Err(RulesError::new(
                    format!("weapons.{id}.lifetime_s"),
                    format!(
                        "missile reaches {reach} m before expiry, short of range_m {}",
                        weapon.ballistics.range_m
                    ),
                ));
            }
        }
        for (layer, fire) in &weapon.targets {
            config
                .check_layer(&weapon.ballistics, fire.range_m, fire.scatter_mrad)
                .map_err(|e| {
                    RulesError::new(
                        format!(
                            "weapons.{id}.targets.{}.scatter_mrad",
                            match layer {
                                contract::catalog::AltitudeLayer::Ground => "ground",
                                contract::catalog::AltitudeLayer::LowAir => "low_air",
                                contract::catalog::AltitudeLayer::HighAir => "high_air",
                            }
                        ),
                        e.to_string(),
                    )
                })?;
        }
    }
    if rules.weapons.len() > crate::publication::MAX_WEAPON_ROWS {
        return Err(RulesError::new(
            "weapons",
            format!(
                "weapons: more than {} rows, the publication limit",
                crate::publication::MAX_WEAPON_ROWS
            ),
        ));
    }
    for t in rules.catalog.indices() {
        for mount in rules.catalog.mounts(t) {
            if mount.def.weapons.len() > crate::publication::MAX_AMMO_KINDS {
                return Err(RulesError::new(
                    format!("units.{}.mounts", rules.catalog.id(t)),
                    format!(
                    "units.{}: mount {:?} has more than {} ammunition kinds, the publication limit",
                    rules.catalog.id(t),
                    mount.def.id,
                    crate::publication::MAX_AMMO_KINDS
                ),
                ));
            }
        }
    }
    Ok(config)
}

fn flight_error(prefix: &str, error: FlightConfigError) -> RulesError {
    let field = match &error {
        FlightConfigError::NotPositive(field) => *field,
        FlightConfigError::LifetimeExceedsBound { .. } => "lifetime_s",
        FlightConfigError::TooManySubsegments { .. } => "gravity_scale",
        FlightConfigError::Motor(_) => return RulesError::new(prefix, error.to_string()),
        FlightConfigError::TopAttack(_) => "top_attack",
        FlightConfigError::TighterThanCeiling { .. } => "scatter_mrad",
    };
    RulesError::new(format!("{prefix}.{field}"), error.to_string())
}
