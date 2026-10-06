//! Condition-based retirement; ordinary movement owns the vulnerable journey.
pub fn fraction(health: f64, ammo: f64, age_s: f64) -> f64 {
    0.15 + 0.60
        * ((health.clamp(0.0, 1.0) + ammo.clamp(0.0, 1.0)) / 2.0)
        * (1.0 - age_s.max(0.0) / 1200.0).max(0.5)
}

/// Remaining original health and finite resources, including lost carriers.
pub fn condition(
    unit: &crate::units::Unit,
    rules: &contract::scenario::Rules,
    arsenal: &crate::weapons::Arsenal,
) -> (f64, f64) {
    use contract::catalog::Body;
    use contract::weapons::AmmoCapacity;
    let health = match &unit.unit_type(rules).body {
        Body::Hull(_) => unit.hp / unit.max_hp(rules),
        Body::Squad { slots } => {
            let full: f64 = slots
                .iter()
                .map(|kind| rules.catalog.soldier(kind).hp)
                .sum();
            unit.members
                .iter()
                .map(|member| member.hp.max(0.0))
                .sum::<f64>()
                / full
        }
    };
    let mut resources = 0.0;
    let mut rows = 0;
    let specs = arsenal.specs(unit.kind);
    for mount in &unit.mounts {
        let spec = &specs[mount.spec];
        let held = unit.hull.is_some()
            || if spec.special {
                unit.members.iter().any(|member| member.alive())
            } else {
                unit.members
                    .iter()
                    .any(|member| member.alive() && spec.carriers.contains(&member.slot))
            };
        for (index, &row) in spec.kinds.iter().enumerate() {
            if let AmmoCapacity::Rounds(full) = arsenal.weapons[row].def.ammo {
                rows += 1;
                resources += if held {
                    f64::from(mount.ammo[index].unwrap_or(0)) / f64::from(full)
                } else {
                    0.0
                };
            }
        }
    }
    if let Some(protection) = &unit.protection {
        let capability = unit
            .unit_type(rules)
            .capabilities
            .active_protection
            .as_ref()
            .unwrap();
        rows += 1;
        resources += f64::from(protection.charges) / f64::from(capability.capacity);
    }
    if let Some(stock) = unit.stock {
        if let Some(supply) = unit.unit_type(rules).capabilities.supply.as_ref() {
            rows += 1;
            resources += f64::from(stock) / f64::from(supply.stock);
        }
    }
    (
        health.clamp(0.0, 1.0),
        if rows == 0 {
            1.0
        } else {
            resources / f64::from(rows)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn health_ammo_and_age_preserve_the_refund_floor_and_commitment() {
        for (health, ammo, age, expected) in [
            (1.0, 1.0, 0.0, 0.75),
            (1.0, 1.0, 600.0, 0.45),
            (0.5, 0.5, 600.0, 0.30),
            (0.0, 0.0, 600.0, 0.15),
            (1.0, 1.0, 3600.0, 0.45),
        ] {
            assert!((fraction(health, ammo, age) - expected).abs() < 1e-12);
        }
    }
}
