//! Catalog admission bounds, independent of overridable battle rules.
use serde::{Deserialize, Serialize};

/// The largest hull each drive may have: what the map generator and the
/// route planner are proven against, by tests that drive hulls built at
/// these limits. A unit past them is refused where the rules load, so every
/// admitted unit is covered by those tests.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HullLimits {
    pub tracked: DriveLimits,
    pub wheeled: DriveLimits,
    /// A helicopter's hull, without its rotor disc.
    pub rotor: DriveLimits,
    /// A fixed-wing aircraft's hull.
    pub fixed_wing: DriveLimits,
}

impl HullLimits {
    pub const SHIPPED: Self = Self {
        tracked: DriveLimits {
            half_width_m: 2.1,
            half_length_m: 4.8,
            turning_radius_m: None,
        },
        wheeled: DriveLimits {
            half_width_m: 1.6,
            half_length_m: 6.2,
            turning_radius_m: Some(8.0),
        },
        rotor: DriveLimits {
            half_width_m: 3.7,
            half_length_m: 10.0,
            turning_radius_m: None,
        },
        fixed_wing: DriveLimits {
            half_width_m: 8.8,
            half_length_m: 11.7,
            turning_radius_m: None,
        },
    };
    /// The limits of an aircraft that flies by `flight`.
    pub fn flight(&self, flight: crate::catalog::FlightModel) -> &DriveLimits {
        match flight {
            crate::catalog::FlightModel::Rotor => &self.rotor,
            crate::catalog::FlightModel::FixedWing => &self.fixed_wing,
        }
    }
}

/// The fastest an aircraft of each flight model may cruise, km/h.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AirSpeedLimits {
    pub rotor: f64,
    pub fixed_wing: f64,
}

impl AirSpeedLimits {
    pub const SHIPPED: Self = Self {
        rotor: 320.0,
        fixed_wing: 1000.0,
    };
    /// The fastest an aircraft that flies by `flight` may cruise, km/h.
    pub fn flight(&self, flight: crate::catalog::FlightModel) -> f64 {
        match flight {
            crate::catalog::FlightModel::Rotor => self.rotor,
            crate::catalog::FlightModel::FixedWing => self.fixed_wing,
        }
    }

    /// The first aircraft of `catalog` cruising past its flight model's limit.
    fn check(&self, catalog: &crate::catalog::Catalog) -> Result<(), crate::catalog::CatalogError> {
        use crate::catalog::Mobility;
        for index in catalog.indices() {
            let Mobility::Air {
                cruise_kmh, flight, ..
            } = catalog.get(index).mobility
            else {
                continue;
            };
            let (name, limit) = (flight.name(), self.flight(flight));
            if cruise_kmh > limit {
                return Err(crate::catalog::CatalogError::Invalid {
                    section: "units",
                    id: catalog.id(index).into(),
                    error: format!(
                        "its {name} cruise_kmh {cruise_kmh} is past the catalog limit of {limit} km/h"
                    ),
                });
            }
        }
        Ok(())
    }
}

/// One drive's largest hull; a tracked hull pivots and has no turn limit.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DriveLimits {
    pub half_width_m: f64,
    pub half_length_m: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turning_radius_m: Option<f64>,
}

impl HullLimits {
    /// The first unit of `catalog` whose hull passes its drive's limits.
    fn check(&self, catalog: &crate::catalog::Catalog) -> Result<(), crate::catalog::CatalogError> {
        use crate::catalog::Mobility;
        for index in catalog.indices() {
            let unit = catalog.get(index);
            let Some(hull) = unit.hull() else { continue };
            let (drive, limits, radius) = match unit.mobility {
                Mobility::Tracked { .. } => ("tracked", &self.tracked, None),
                Mobility::Wheeled {
                    turning_radius_m, ..
                } => ("wheeled", &self.wheeled, Some(turning_radius_m)),
                Mobility::Air { flight, .. } => (flight.name(), self.flight(flight), None),
                // A squad has no hull; the catalog refuses a hull on foot.
                Mobility::Foot { .. } => continue,
            };
            let [length, width, _] = hull.half_extents_m;
            let past = [
                ("half width", width, Some(limits.half_width_m)),
                ("half length", length, Some(limits.half_length_m)),
            ]
            .into_iter()
            .chain(radius.map(|r| ("turning radius", r, limits.turning_radius_m)))
            .find(|(_, value, limit)| limit.is_some_and(|limit| *value > limit));
            if let Some((what, value, limit)) = past {
                return Err(crate::catalog::CatalogError::Invalid {
                    section: "units",
                    id: catalog.id(index).into(),
                    error: format!(
                        "its {drive} {what} {value} m is past the catalog limit of {} m",
                        limit.unwrap()
                    ),
                });
            }
        }
        Ok(())
    }
}

impl crate::catalog::Catalog {
    /// Refuse a hull or cruise speed beyond the caller's supported envelope.
    pub fn check_limits(
        &self,
        hull: HullLimits,
        air: AirSpeedLimits,
    ) -> Result<(), crate::catalog::CatalogError> {
        hull.check(self)?;
        air.check(self)
    }
}
