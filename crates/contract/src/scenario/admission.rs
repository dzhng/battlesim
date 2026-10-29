//! Deserialization and cross-section admission of battle rules.
use super::*;

/// [`Rules`] as read, before the checks across its sections.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UncheckedRules {
    tick_hz: u32,
    commands: CommandRules,
    movement: MovementRules,
    navigation: NavigationRules,
    formation: FormationRules,
    infantry_movement: InfantryMovementRules,
    physics: BodyRules,
    catalog: crate::catalog::Catalog,
    pushing: PushingRules,
    ricochet: RicochetRules,
    guided: crate::ballistics::GuidedRules,
    launch: LaunchRules,
    sensors: SensorRules,
    forests: ForestRules,
    #[serde(deserialize_with = "crate::weapons::resolve_weapons")]
    weapons: crate::weapons::WeaponRules,
    service: ServiceRules,
    suppression: SuppressionRules,
    cover: CoverRules,
    ground: GroundRules,
    buildings: BuildingRules,
    garrison: GarrisonRules,
    carriage: CarriageRules,
    bail_out: std::collections::BTreeMap<BailOutHost, BailOutRow>,
    hull_limits: HullLimits,
    air_speed_limits_kmh: AirSpeedLimits,
    air: AirRules,
    shouts: ShoutRules,
}

impl TryFrom<UncheckedRules> for Rules {
    type Error = crate::catalog::CatalogError;
    fn try_from(r: UncheckedRules) -> Result<Self, Self::Error> {
        r.catalog.check_weapons(&r.weapons)?;
        r.hull_limits.check(&r.catalog)?;
        r.air_speed_limits_kmh.check(&r.catalog)?;
        for (id, checked) in [
            ("suppression", r.suppression.check()),
            ("navigation", r.navigation.check()),
            ("movement", r.movement.speed_factors.check()),
            ("sensors", r.sensors.check()),
            ("forests", r.forests.check(&r.catalog)),
            ("formation", r.formation.check()),
            ("commands", r.commands.check()),
            ("carriage", r.carriage.check()),
            ("air", r.air.check()),
            ("shouts", r.shouts.check()),
        ] {
            checked.map_err(|error| crate::catalog::CatalogError::Invalid {
                section: "rules",
                id: id.into(),
                error,
            })?;
        }
        // Every host a passenger can be thrown out of has its row, so a
        // bail-out never finds one missing mid-battle.
        for (host, name) in [
            (BailOutHost::Building, "building"),
            (BailOutHost::Ground, "ground"),
            (BailOutHost::Air, "air"),
        ] {
            if !r.bail_out.contains_key(&host) {
                return Err(crate::catalog::CatalogError::Invalid {
                    section: "rules",
                    id: "bail_out".into(),
                    error: format!("no row for {name} hosts"),
                });
            }
        }
        Ok(Rules {
            tick_hz: r.tick_hz,
            commands: r.commands,
            movement: r.movement,
            navigation: r.navigation,
            formation: r.formation,
            infantry_movement: r.infantry_movement,
            physics: r.physics,
            catalog: r.catalog,
            pushing: r.pushing,
            ricochet: r.ricochet,
            guided: r.guided,
            launch: r.launch,
            sensors: r.sensors,
            forests: r.forests,
            weapons: r.weapons,
            service: r.service,
            suppression: r.suppression,
            cover: r.cover,
            ground: r.ground,
            buildings: r.buildings,
            garrison: r.garrison,
            carriage: r.carriage,
            bail_out: r.bail_out,
            hull_limits: r.hull_limits,
            air_speed_limits_kmh: r.air_speed_limits_kmh,
            air: r.air,
            shouts: r.shouts,
        })
    }
}
