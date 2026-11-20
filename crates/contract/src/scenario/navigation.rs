//! Route planning rules.
use serde::{Deserialize, Serialize};

/// How routes are planned. A unit holds where it is while its route is
/// worked out, a share of each tick's allowance at a time. A long ordered
/// leg goes by road where a road journey beats driving straight there.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationRules {
    /// Planning work every side's units share each tick: one unit is about
    /// one grid cell searched.
    pub work_per_tick: u32,
    /// How far a search looks before it gives the route up as blocked: this
    /// many cells, plus `search_cells_per_m` for every metre between its
    /// two ends.
    pub search_cells_base: u32,
    pub search_cells_per_m: u32,
    /// An ordered move leg strictly longer than this, as the crow flies
    /// when the leg starts, is driven by the fastest policy: by road where
    /// that is quicker.
    pub road_leg_m: f64,
    /// How far from each end of a leg a road is looked for.
    pub road_access_m: f64,
    /// A unit sent where it has no room to stand stops at the nearest room
    /// it can reach, or short of it on its way there: it drives a detour to
    /// the nearer only while each metre of ground gained costs at most this
    /// many metres of extra driving.
    pub stop_short_detour_ratio: f64,
    /// A vehicle on the move sent somewhere within this many degrees of the
    /// way it travels drives on, braking, while the new route is worked
    /// out; sent farther round than that, it stops for the route.
    pub drive_on_deg: f64,
    /// The room a vehicle's route keeps each side of its hull from
    /// whatever it passes: a gap narrower than the hull and twice this is
    /// no way through.
    pub hull_margin_m: f64,
}

impl NavigationRules {
    /// The cells a search between two points `distance_m` apart may expand.
    pub fn search_limit(&self, distance_m: f64) -> usize {
        self.search_cells_base as usize + (self.search_cells_per_m as f64 * distance_m) as usize
    }

    pub(super) fn check(&self) -> Result<(), String> {
        if self.work_per_tick == 0 {
            return Err("work_per_tick must be positive: no route would ever finish".into());
        }
        if self.search_cells_base == 0 {
            return Err("search_cells_base must be positive: no search would look anywhere".into());
        }
        if !self.stop_short_detour_ratio.is_finite() || self.stop_short_detour_ratio < 0.0 {
            return Err("stop_short_detour_ratio must be finite and not negative".into());
        }
        if !(0.0..=180.0).contains(&self.drive_on_deg) {
            return Err("drive_on_deg must be an angle between 0 and 180".into());
        }
        for (name, v) in [
            ("road_leg_m", self.road_leg_m),
            ("road_access_m", self.road_access_m),
            ("hull_margin_m", self.hull_margin_m),
        ] {
            if !v.is_finite() || v < 0.0 {
                return Err(format!(
                    "{name} must be a distance: finite and not negative"
                ));
            }
        }
        Ok(())
    }
}
