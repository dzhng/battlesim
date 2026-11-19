//! Validated physical flight bounds and weapon launch profiles.

use super::*;

/// Validated flight bounds at one tick rate.
#[derive(Clone, Debug)]
pub struct FlightConfig {
    pub(super) gravity: V3,
    pub(super) miss_fall_max_s: f64,
    pub(super) ricochet_lifetime_s: f64,
    pub(super) miss_gravity_multiplier: f64,
    pub(super) tick_s: f64,
    pub(super) subsegments: u32,
    pub(super) chord_error_m: f64,
    pub(super) max_unguided_lifetime_s: f64,
    pub(super) min_spread_at_max_range_m: f64,
    /// Intercepts and launches already solved ([`solve::Solved`]).
    pub(super) solved: solve::Solved,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FlightConfigError {
    NotPositive(&'static str),
    /// Keeping chords within the chord error would need more chords per tick
    /// than declared; contacts would otherwise be skipped silently.
    TooManySubsegments {
        required: u32,
        max: u32,
    },
    LifetimeExceedsBound {
        lifetime_s: f64,
        max_s: f64,
    },
    /// `accel_mps2` and `top_speed_mps` come together, on a guided row (a
    /// motor round flies without gravity), with a top speed above launch.
    Motor(&'static str),
    /// `top_attack` belongs on a guided row, with a positive loft and a dive
    /// between level and vertical.
    TopAttack(&'static str),
    ArcHeight(&'static str),
    /// A launch elevation belongs on a guided row and lies between level and
    /// vertical.
    LaunchElevation(&'static str),
    /// An unguided layer more accurate at its reach than
    /// `physics.min_spread_at_max_range_m` allows.
    TighterThanCeiling {
        spread_m: f64,
        min_m: f64,
    },
}

impl std::fmt::Display for FlightConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotPositive(field) => write!(f, "{field} is outside its finite physical range"),
            Self::TooManySubsegments { required, max } => write!(
                f,
                "Trajectory needs {required} collision segments per tick; the limit is {max}. Reduce gravity strength"
            ),
            Self::LifetimeExceedsBound { lifetime_s, max_s } => write!(
                f,
                "Projectile lifetime must be at most {max_s} s; entered {lifetime_s} s"
            ),
            Self::Motor(reason) => write!(f, "Invalid missile motor: {reason}"),
            Self::TopAttack(reason) => write!(f, "Invalid top attack: {reason}"),
            Self::ArcHeight(reason) => write!(f, "Invalid arc height: {reason}"),
            Self::LaunchElevation(reason) => write!(f, "Invalid launch elevation: {reason}"),
            Self::TighterThanCeiling { spread_m, min_m } => write!(
                f,
                "Landing spread must be at least {min_m} m at maximum range; entered {spread_m} m"
            ),
        }
    }
}

/// Chords per tick keeping each within `chord_error` of an arc under constant
/// acceleration `accel`: a chord of duration h strays at most accel·h²/8.
pub(super) fn subsegments_for(accel: f64, tick_s: f64, chord_error: f64) -> u32 {
    ((accel * tick_s * tick_s / (8.0 * chord_error))
        .sqrt()
        .ceil() as u32)
        .max(1)
}

impl FlightConfig {
    pub fn new(rules: &FlightRules, tick_hz: u32) -> Result<Self, FlightConfigError> {
        let positive = |name, v: f64| (v > 0.0 && v.is_finite()).then_some(()).ok_or(name);
        positive("tick_hz", tick_hz as f64)
            .and(positive("curve_chord_error_m", rules.curve_chord_error_m))
            .and(positive(
                "max_subsegments_per_tick",
                rules.max_subsegments_per_tick as f64,
            ))
            .and(positive(
                "max_unguided_lifetime_s",
                rules.max_unguided_lifetime_s,
            ))
            .and(positive("gravity_mps2", rules.gravity_mps2))
            .and(positive(
                "miss_gravity_multiplier",
                rules.miss_gravity_multiplier,
            ))
            .and(
                (rules.ricochet_lifetime_s >= 0.0 && rules.ricochet_lifetime_s.is_finite())
                    .then_some(())
                    .ok_or("ricochet_lifetime_s"),
            )
            .and(
                (rules.miss_fall_max_s >= 0.0 && rules.miss_fall_max_s.is_finite())
                    .then_some(())
                    .ok_or("miss_fall_max_s"),
            )
            .and(
                (rules.min_spread_at_max_range_m >= 0.0
                    && rules.min_spread_at_max_range_m.is_finite())
                .then_some(())
                .ok_or("min_spread_at_max_range_m"),
            )
            .map_err(FlightConfigError::NotPositive)?;
        let tick_s = 1.0 / tick_hz as f64;
        let required = subsegments_for(rules.gravity_mps2, tick_s, rules.curve_chord_error_m);
        if required > rules.max_subsegments_per_tick {
            return Err(FlightConfigError::TooManySubsegments {
                required,
                max: rules.max_subsegments_per_tick,
            });
        }
        Ok(FlightConfig {
            solved: solve::Solved::default(),
            gravity: v3(0.0, 0.0, -rules.gravity_mps2),
            miss_fall_max_s: rules.miss_fall_max_s,
            ricochet_lifetime_s: rules.ricochet_lifetime_s,
            miss_gravity_multiplier: rules.miss_gravity_multiplier,
            tick_s,
            subsegments: required,
            chord_error_m: rules.curve_chord_error_m,
            max_unguided_lifetime_s: rules.max_unguided_lifetime_s,
            min_spread_at_max_range_m: rules.min_spread_at_max_range_m,
        })
    }

    pub fn gravity(&self) -> V3 {
        self.gravity
    }

    pub fn tick_s(&self) -> f64 {
        self.tick_s
    }

    pub fn subsegments_per_tick(&self) -> u32 {
        self.subsegments
    }

    /// A weapon row's launch profile; its lifetime is bounded.
    pub fn profile(&self, weapon: &WeaponBallistics) -> Result<LaunchProfile, FlightConfigError> {
        let lifetime_s = weapon.lifetime_s.unwrap_or(self.max_unguided_lifetime_s);
        for (name, v) in [
            ("speed_mps", weapon.speed_mps),
            ("range_m", weapon.range_m),
            ("lifetime_s", lifetime_s),
            ("gravity_scale", weapon.gravity_scale),
        ] {
            if weapon.delivery == Delivery::Unpowered
                && (name == "speed_mps" || name == "range_m")
                && v == 0.0
            {
                continue;
            }
            if !(v > 0.0 && v.is_finite()) {
                return Err(FlightConfigError::NotPositive(name));
            }
        }
        if lifetime_s > self.max_unguided_lifetime_s {
            return Err(FlightConfigError::LifetimeExceedsBound {
                lifetime_s,
                max_s: self.max_unguided_lifetime_s,
            });
        }
        if let Some(height) = weapon.arc_height_m {
            if !(height.is_finite() && height > 0.0) {
                return Err(FlightConfigError::NotPositive("arc_height_m"));
            }
            if weapon.trajectory != Trajectory::Indirect
                || weapon.delivery != Delivery::Powered
                || weapon.turn_deg_s.is_some()
            {
                return Err(FlightConfigError::ArcHeight(
                    "it needs a powered, unguided indirect row",
                ));
            }
            let gravity = -self.gravity.z * weapon.gravity_scale;
            let rise_s = libm::sqrt(2.0 * height / gravity);
            if 2.0 * rise_s > lifetime_s
                || libm::hypot(weapon.range_m / (2.0 * rise_s), gravity * rise_s) > weapon.speed_mps
            {
                return Err(FlightConfigError::ArcHeight(
                    "the arc cannot reach maximum range on level ground within launch speed and lifetime",
                ));
            }
        }
        let motor = match (weapon.accel_mps2, weapon.top_speed_mps) {
            (None, None) => None,
            (Some(accel_mps2), Some(top_speed_mps)) => {
                if weapon.turn_deg_s.is_none() {
                    return Err(FlightConfigError::Motor("a motor needs a guided row"));
                }
                if !(accel_mps2 > 0.0 && accel_mps2.is_finite()) {
                    return Err(FlightConfigError::NotPositive("accel_mps2"));
                }
                if !(top_speed_mps > weapon.speed_mps && top_speed_mps.is_finite()) {
                    return Err(FlightConfigError::Motor(
                        "top_speed_mps must exceed speed_mps",
                    ));
                }
                Some(Motor {
                    accel_mps2,
                    top_speed_mps,
                })
            }
            _ => {
                return Err(FlightConfigError::Motor(
                    "accel_mps2 and top_speed_mps come together",
                ))
            }
        };
        let loft = match weapon.top_attack {
            None => None,
            Some(_) if weapon.turn_deg_s.is_none() => {
                return Err(FlightConfigError::TopAttack("it needs a guided row"))
            }
            Some(t) if !(t.loft_m > 0.0 && t.loft_m.is_finite()) => {
                return Err(FlightConfigError::TopAttack("loft_m must be positive"))
            }
            Some(t) if !(t.dive_deg > 0.0 && t.dive_deg < 90.0) => {
                return Err(FlightConfigError::TopAttack(
                    "dive_deg must lie between 0 and 90",
                ))
            }
            Some(t) => Some(Loft {
                height_m: t.loft_m,
                slope: libm::tan(t.dive_deg.to_radians()),
            }),
        };
        let launch_elevation_rad = match weapon.launch_elevation_deg {
            None => None,
            Some(deg) if !(deg.is_finite() && (0.0..=90.0).contains(&deg)) => {
                return Err(FlightConfigError::LaunchElevation(
                    "must lie between 0 and 90 degrees",
                ))
            }
            Some(_) if weapon.turn_deg_s.is_none() => {
                return Err(FlightConfigError::LaunchElevation("it needs a guided row"))
            }
            Some(deg) => Some(deg.to_radians()),
        };
        // A heavier share of gravity bends each chord more: it must still
        // keep within the chord error at the declared chords per tick.
        let accel = self.gravity.length() * weapon.gravity_scale;
        let required = subsegments_for(accel, self.tick_s, self.chord_error_m);
        if required > self.subsegments {
            return Err(FlightConfigError::TooManySubsegments {
                required,
                max: self.subsegments,
            });
        }
        Ok(LaunchProfile {
            speed_mps: weapon.speed_mps,
            gravity_scale: weapon.gravity_scale,
            lifetime_s,
            trajectory: weapon.trajectory,
            arc_height_m: weapon.arc_height_m,
            suppression_radius_m: weapon.suppression_radius_m,
            turn_rad_s: weapon.turn_deg_s.map(f64::to_radians),
            launch_elevation_rad,
            motor,
            loft,
        })
    }

    /// One target layer's spread at its reach, held to the accuracy ceiling.
    pub fn check_layer(
        &self,
        weapon: &WeaponBallistics,
        range_m: f64,
        scatter_mrad: f64,
    ) -> Result<(), FlightConfigError> {
        let spread_m = scatter_mrad * 1e-3 * range_m;
        if weapon.delivery == Delivery::Powered
            && weapon.speed_mps > 0.0
            && range_m > 0.0
            && weapon.turn_deg_s.is_none()
            && spread_m < self.min_spread_at_max_range_m
        {
            return Err(FlightConfigError::TighterThanCeiling {
                spread_m,
                min_m: self.min_spread_at_max_range_m,
            });
        }
        Ok(())
    }
}

/// What a weapon's launch needs from its row, validated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaunchProfile {
    pub speed_mps: f64,
    /// The share of the world's gravity an unguided round falls under.
    pub gravity_scale: f64,
    pub lifetime_s: f64,
    pub trajectory: Trajectory,
    pub arc_height_m: Option<f64>,
    pub suppression_radius_m: f64,
    /// Guided rounds' turn limit; `None` flies ballistically.
    pub turn_rad_s: Option<f64>,
    /// A guided round's elevation as it leaves its launcher.
    pub launch_elevation_rad: Option<f64>,
    /// A rocket motor speeding the round up along its heading.
    pub motor: Option<Motor>,
    /// A guided round's top-attack path.
    pub loft: Option<Loft>,
}

/// A top-attack path, validated: climb at the dive angle to `height_m`
/// above the commanded point and hold it until the point lies below the
/// round by at least `slope` (the tangent of the dive angle) times their
/// horizontal distance, then dive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Loft {
    pub height_m: f64,
    pub slope: f64,
}

/// A rocket motor: from its launch speed the round speeds up at
/// `accel_mps2` along its heading, then holds `top_speed_mps`. The thrust
/// is along the velocity, so a tick's flight stays a straight line and
/// its chords are exact.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motor {
    pub accel_mps2: f64,
    pub top_speed_mps: f64,
}

impl Motor {
    /// The thrust over the next `span` seconds for a round at `velocity`:
    /// the full rate, or what reaches top speed exactly at the span's end.
    pub fn thrust(&self, velocity: V3, span: f64) -> V3 {
        let speed = velocity.length();
        if speed <= 0.0 || span <= 0.0 {
            return v3(0.0, 0.0, 0.0);
        }
        let rate = self
            .accel_mps2
            .min((self.top_speed_mps - speed).max(0.0) / span);
        velocity * (rate / speed)
    }

    /// Distance flown `t` seconds after a launch at `launch_mps`, in a
    /// straight line.
    pub fn distance(&self, launch_mps: f64, t: f64) -> f64 {
        let burn = ((self.top_speed_mps - launch_mps) / self.accel_mps2).max(0.0);
        let tb = t.min(burn);
        launch_mps * tb + 0.5 * self.accel_mps2 * tb * tb + self.top_speed_mps * (t - tb)
    }
}

impl LaunchProfile {
    /// Keep the solved launch speed while using an authored launcher elevation.
    pub fn initial_velocity(&self, solved: V3, toward: V3) -> V3 {
        let Some(elevation) = self.launch_elevation_rad else {
            return solved;
        };
        let horizontal = if solved.xy().length() > 1e-9 {
            solved.xy().normalized()
        } else {
            toward.xy().normalized()
        };
        let speed = solved.length();
        let (sin, cos) = libm::sincos(elevation);
        v3(
            horizontal.x * speed * cos,
            horizontal.y * speed * cos,
            speed * sin,
        )
    }

    /// The acceleration the round flies under: its share of gravity, or
    /// none for a motor-sustained guided round.
    pub fn gravity(&self, config: &FlightConfig) -> V3 {
        match self.turn_rad_s {
            Some(_) => v3(0.0, 0.0, 0.0),
            None => config.gravity * self.gravity_scale,
        }
    }
}
