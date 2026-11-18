use super::*;

/// The descent and clearance a rotor aircraft authors to set down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Landing {
    pub drop_seconds: f64,
    pub rotor_radius_m: f64,
}

impl UnitType {
    /// Its landing geometry and timing, if it sets up by landing.
    pub fn landing(&self) -> Option<Landing> {
        let Mobility::Air {
            rotor_radius_m: Some(rotor_radius_m),
            ..
        } = self.mobility
        else {
            return None;
        };
        Some(Landing {
            drop_seconds: self.capabilities.deploy?.drop_seconds?,
            rotor_radius_m,
        })
    }

    /// What it is to a weapon choosing its targets: an aircraft by how it
    /// flies, else a hull on the ground, else soldiers.
    pub fn target_class(&self) -> crate::weapons::TargetClass {
        use crate::weapons::TargetClass;
        match (self.mobility.flight(), &self.body) {
            (Some(FlightModel::Rotor), _) => TargetClass::AirRotor,
            (Some(FlightModel::FixedWing), _) => TargetClass::AirFixedWing,
            (None, Body::Hull(_)) => TargetClass::Vehicle,
            (None, Body::Squad { .. }) => TargetClass::Infantry,
        }
    }

    /// A vehicle: it has a hull.
    pub fn hull(&self) -> Option<&Hull> {
        match &self.body {
            Body::Hull(h) => Some(h),
            Body::Squad { .. } => None,
        }
    }

    /// A vehicle that drives on the ground: its hull. None for a squad and
    /// for an aircraft, which has a hull but never drives a road.
    pub fn ground_hull(&self) -> Option<&Hull> {
        match self.mobility {
            Mobility::Air { .. } => None,
            _ => self.hull(),
        }
    }

    /// Infantry: it has soldier slots.
    pub fn slots(&self) -> Option<&[String]> {
        match &self.body {
            Body::Squad { slots } => Some(slots),
            Body::Hull(_) => None,
        }
    }

    /// Soldiers at full strength (0 for a hull).
    pub fn squad_size(&self) -> usize {
        self.slots().map_or(0, <[String]>::len)
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// A vehicle's presentation class, from its physics: how it moves
    /// (tracked, wheeled, `air`, or `high_air` for an aircraft in high air),
    /// its hull's weight class, plus `_logistics` for a supply carrier.
    /// None for a squad.
    pub fn vehicle_class(&self) -> Option<String> {
        let hull = self.hull()?;
        let moves = match self.mobility {
            Mobility::Foot { .. } => return None,
            Mobility::Tracked { .. } => "tracked",
            Mobility::Wheeled { .. } => "wheeled",
            Mobility::Air {
                layer: AltitudeLayer::HighAir,
                ..
            } => "high_air",
            Mobility::Air { .. } => "air",
        };
        let weight = match hull.weight_class {
            WeightClass::Trivial => "trivial",
            WeightClass::Light => "light",
            WeightClass::Medium => "medium",
            WeightClass::Heavy => "heavy",
            WeightClass::Immovable => "immovable",
        };
        let hauls = if self.capabilities.supply.is_some() {
            "_logistics"
        } else {
            ""
        };
        Some(format!("{moves}_{weight}{hauls}"))
    }
}
