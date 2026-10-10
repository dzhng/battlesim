//! Authored flight data: the fixture's `physics` flight bounds, its `guided`
//! release rule and the flight fields of a weapon row. Other fields in those sections belong to other
//! owners and are ignored here.
use serde::{Deserialize, Serialize};

/// Flight bounds from the fixture's `physics` section.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlightRules {
    pub gravity_mps2: f64,
    /// Maximum fall time after a direct round passes its original aim point; zero disables it.
    #[serde(default)]
    pub miss_fall_max_s: f64,
    /// Flight time left after the first ricochet; zero keeps the original lifetime.
    #[serde(default)]
    pub ricochet_lifetime_s: f64,
    /// Downward acceleration after passing the aim point, in world gravities.
    #[serde(default = "full_gravity")]
    pub miss_gravity_multiplier: f64,
    /// Largest allowed distance between a flown chord and the true curve.
    pub curve_chord_error_m: f64,
    /// Declared bound on chords per tick; authoring that needs more is invalid.
    pub max_subsegments_per_tick: u32,
    /// Lifetime of an unguided round, and the bound on any authored one.
    pub max_unguided_lifetime_s: f64,
    /// The accuracy ceiling: no unguided weapon's spread at its own
    /// `range_m` (`scatter_mrad` × range, the one-axis standard deviation of
    /// where its rounds land about the aim point) is tighter than this, in
    /// metres. A long gun is no sniper at its limit, and every gun grows
    /// deadlier as the distance shrinks (Hollywood realism: close in for
    /// the kill). A row may be less accurate; a guided row steers out its
    /// spread and is exempt.
    pub min_spread_at_max_range_m: f64,
}

/// What a guided missile does once its launcher stops supporting it (the
/// fixture's `guided` section).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuidedRules {
    /// A released missile flies straight on for this long, then goes to
    /// ground: its commanded point becomes where it would be after this many
    /// seconds, dropped to the ground beneath.
    pub release_coast_s: f64,
}

/// Which ballistic arcs a weapon may choose (P03).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trajectory {
    /// Normal low arc only.
    #[default]
    Direct,
    /// A dedicated indirect-fire weapon: prefers the high arc.
    Indirect,
}

/// The flight fields of a fixture weapon row.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeaponBallistics {
    /// Launch speed: a gun's muzzle speed, a missile's speed off the rail.
    pub speed_mps: f64,
    /// Farthest the weapon is fired; its spread there is held to at least
    /// `physics.min_spread_at_max_range_m`.
    pub range_m: f64,
    /// A rocket motor: the round speeds up along its heading at this rate
    /// until `top_speed_mps` (absent: none). Only a guided round, which flies
    /// without gravity, has one.
    #[serde(default)]
    pub accel_mps2: Option<f64>,
    /// The speed its motor holds once reached; given with `accel_mps2`.
    #[serde(default)]
    pub top_speed_mps: Option<f64>,
    /// The share of the world's gravity an unguided round falls under
    /// (default 1). Hollywood realism: a gun round flies at a speed chosen
    /// for the screen, not the real round's, and falls under
    /// (speed / real speed)², so it flies the real round's line (the same
    /// drop over the same distance) in its own time.
    #[serde(default = "full_gravity")]
    pub gravity_scale: f64,
    /// One-axis angular standard deviation of launch spread, milliradians.
    pub scatter_mrad: f64,
    /// Path distance within which a passing round reports a near miss.
    pub suppression_radius_m: f64,
    /// Authored lifetime; unguided rounds without one use the physics bound.
    #[serde(default)]
    pub lifetime_s: Option<f64>,
    #[serde(default)]
    pub trajectory: Trajectory,
    /// A guided round: it flies without gravity, steering toward its
    /// commanded point no faster than this (P05, P06).
    #[serde(default)]
    pub turn_deg_s: Option<f64>,
    /// How a guided round's launcher steers it (D38); every guided row says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guidance: Option<Guidance>,
    /// A guided round that climbs above its point, then dives onto it from
    /// above (absent: it flies straight at its point).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_attack: Option<TopAttack>,
}

/// When a guided round's launcher can steer it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Guidance {
    /// Only while the launcher holds still (a ground crew's wire-guided missile).
    Stationary,
    /// While the launcher moves too (a helicopter's missile).
    OnTheMove,
}

/// A top-attack path: while its launcher supports it, a guided round steers
/// at a point `loft_m` above its commanded point until that point lies at
/// least `dive_deg` below its horizon, then dives onto it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopAttack {
    pub loft_m: f64,
    pub dive_deg: f64,
}

fn full_gravity() -> f64 {
    1.0
}
