//! A ground strike's footprint: how far from an attack-ground aim a unit's
//! rounds can do damage. The page draws it round the attack-ground cursor, so
//! the circle says what the simulation does.
use contract::catalog::{AltitudeLayer, FlightModel, TypeIndex};
use contract::scenario::Rules;

use crate::air::pass::{choose_weapon, firing_range_m, pass_weapon, PassWeapon};
use crate::weapons::{Arsenal, Weapon};

/// The radius round a ground aim `distance_m` from a unit of `kind` within
/// which its strikes there can do damage: the farthest its rounds stray at the
/// range they are fired from, plus their blast, plus half a salvo's walk along
/// a jet's path. A ground unit fires every row that can strike an area, from
/// where it stands, closing to its reach first: the widest counts. A jet
/// makes its runs with the one weapon a pass chooses (`pass::choose_weapon`),
/// from its run wherever it is now, and always moving. `None` without a row
/// that strikes an area.
pub fn footprint_m(
    rules: &Rules,
    arsenal: &Arsenal,
    kind: TypeIndex,
    distance_m: f64,
) -> Option<f64> {
    let mobility = &rules.catalog.get(kind).mobility;
    let jet = mobility.flight() == Some(FlightModel::FixedWing);
    // A jet never stops: its salvo walks along its path at cruise.
    let speed = if jet {
        mobility.speeds_kmh().0 / 3.6
    } else {
        0.0
    };
    let rows: Vec<&Weapon> = arsenal
        .specs(kind)
        .iter()
        .flat_map(|mount| mount.kinds.iter())
        .map(|&k| &arsenal.weapons[k])
        .filter(|weapon| crate::weapons::area_capable(&weapon.def))
        .collect();
    let pass = jet
        .then(|| {
            let modes: Vec<PassWeapon> = rows.iter().filter_map(|w| pass_weapon(&w.def)).collect();
            choose_weapon(&modes)
        })
        .flatten();
    rows.into_iter()
        .filter(|weapon| !jet || pass.is_some_and(|mode| pass_weapon(&weapon.def) == Some(mode)))
        .filter_map(|weapon| {
            let fire = weapon.def.against(AltitudeLayer::Ground)?;
            let (scatter, range) = if jet {
                (
                    fire.scatter_mrad * rules.physics.moving_scatter_multiplier,
                    firing_range_m(weapon, &arsenal.config, &rules.air.fixed_wing, speed)?,
                )
            } else {
                (fire.scatter_mrad, distance_m.min(fire.range_m))
            };
            let walk = weapon.def.magazine.map_or(0.0, |m| {
                f64::from(m.rounds.saturating_sub(1)) * m.shot_interval_s * speed
            });
            Some(
                crate::flight::scatter_reach_m(scatter, range)
                    + weapon.def.blast_radius_m
                    + walk / 2.0,
            )
        })
        .reduce(f64::max)
}
