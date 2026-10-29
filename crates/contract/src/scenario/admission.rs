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
    type Error = RulesError;
    fn try_from(r: UncheckedRules) -> Result<Self, Self::Error> {
        let rules = Rules {
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
        };
        rules.check()?;
        Ok(rules)
    }
}

/// A refused rule, located relative to the battle rules document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulesError {
    pub path: String,
    pub message: String,
}

impl RulesError {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for RulesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for RulesError {}

impl Rules {
    /// Contract-owned section and cross-section checks, also used by simulation admission.
    pub fn check(&self) -> Result<(), RulesError> {
        self.infantry_movement.check()?;
        self.cover.check()?;
        self.ground.check()?;
        self.movement.check()?;
        self.ricochet.check()?;
        self.guided.check()?;
        self.physics.check()?;
        self.garrison.check()?;
        self.pushing.check()?;
        self.launch.check()?;
        self.catalog
            .check_weapons(&self.weapons)
            .map_err(|e| RulesError::new("catalog", e.to_string()))?;
        self.hull_limits
            .check(&self.catalog)
            .map_err(|e| RulesError::new("hull_limits", e.to_string()))?;
        self.air_speed_limits_kmh
            .check(&self.catalog)
            .map_err(|e| RulesError::new("air_speed_limits_kmh", e.to_string()))?;
        for (id, checked) in [
            ("suppression", self.suppression.check()),
            ("navigation", self.navigation.check()),
            ("sensors", self.sensors.check()),
            ("forests", self.forests.check(&self.catalog)),
            ("formation", self.formation.check()),
            ("commands", self.commands.check()),
            ("carriage", self.carriage.check()),
            ("air", self.air.check()),
            ("shouts", self.shouts.check()),
        ] {
            checked.map_err(|error| RulesError::new(id, error))?;
        }
        // Every host a passenger can be thrown out of has its row, so a
        // bail-out never finds one missing mid-battle.
        for (host, name) in [
            (BailOutHost::Building, "building"),
            (BailOutHost::Ground, "ground"),
            (BailOutHost::Air, "air"),
        ] {
            self.bail_out
                .get(&host)
                .ok_or_else(|| RulesError::new(format!("bail_out.{name}"), "missing host row"))?
                .check(name)?;
        }
        Ok(())
    }
}
