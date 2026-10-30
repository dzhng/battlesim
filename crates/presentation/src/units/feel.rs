//! Authored pose settings, shared by fielding and the pose driver.
use super::driver::StanceFeel;
use contract::catalog::FlightModel;
use serde::Deserialize;

/// `presentation.pose`: how poses move between what the simulation
/// publishes. Presentation feel, not rules.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct PoseFeel {
    pub gait: Gait,
    pub rest: Rest,
    pub stance: StanceFeel,
    pub lean: Lean,
    pub mount: MountFeel,
    /// How a shot kicks the hull on the ground that fires it; none: never.
    pub hull_kick: Option<HullKick>,
    /// How each of his own shots kicks a soldier; none: never.
    pub soldier_kick: Option<SoldierKick>,
    pub rotor: RotorFeel,
    pub attitude: AttitudeFeel,
    pub corpses: CorpseCap,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Gait {
    pub walk_mps: f64,
    pub run_mps: f64,
    pub crawl_mps: f64,
    pub fade_s: f64,
    pub facing_mps: f64,
    pub turn_rad_s: f64,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rest {
    pub turn_rad: f64,
    pub tempo: [f64; 2],
    pub watch_every: f64,
    pub settle_s: [f64; 2],
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Lean {
    pub out_s: f64,
    pub back_s: f64,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct MountFeel {
    pub gun_elevation_rad_s: f64,
    pub hmg_elevation_rad_s: f64,
    pub recoil_m: f64,
    pub recoil_return_s: f64,
}

/// A shot's recoil throws the hull back on its springs, away from where the
/// round went, and it rocks back past rest and settles, never moving off its
/// spot. At a look's hull kick of 1 the
/// hull's roof swings back at most `rock_deg` about its base while it slides
/// `slide_m` back; at 0 it never moves. Its springs ring at `hz`, damped by
/// `damping` (a share of critical, in (0, 1)).
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct HullKick {
    pub rock_deg: f64,
    pub slide_m: f64,
    pub hz: f64,
    pub damping: f64,
}

/// Each of his own shots snaps a soldier's upper body back about his waist
/// ([`KICK_JOINT`](super::skin::KICK_JOINT)), his muzzle climbing
/// `pitch_deg` and his shoulders thrown `back_m` back, and he recovers over
/// `return_s`. The weapon look supplies its share of that motion.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct SoldierKick {
    pub pitch_deg: f64,
    pub back_m: f64,
    pub return_s: f64,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RotorFeel {
    pub tip_mps: f64,
}

/// How each flight model leans an aircraft.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct AttitudeFeel {
    pub rotor: TiltFeel,
    pub fixed_wing: TiltFeel,
}

impl AttitudeFeel {
    /// The tilt of an aircraft that flies by `flight`.
    pub fn flight(&self, flight: FlightModel) -> &TiltFeel {
        match flight {
            FlightModel::Rotor => &self.rotor,
            FlightModel::FixedWing => &self.fixed_wing,
        }
    }
}

/// How far and how quickly an aircraft tilts: `gain` scales the angle its
/// flight asks for, clamped to `max_deg` and eased over `smoothing_s`.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct TiltFeel {
    pub gain: f64,
    pub max_deg: f64,
    pub smoothing_s: f64,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct CorpseCap {
    pub max: usize,
    pub fade_s: f64,
    pub sink_m: f64,
}

impl PoseFeel {
    pub fn from_value(value: &serde_json::Value) -> Result<Self, String> {
        let feel: Self = crate::settings::read(value, "presentation.pose")?;
        feel.check()?;
        Ok(feel)
    }

    pub fn check(&self) -> Result<(), String> {
        use crate::settings::validate::{numbers, range};
        numbers(self, "presentation.pose", &[])?;
        for (name, v) in [
            ("gait.fade_s", self.gait.fade_s),
            ("rest.settle_s[1]", self.rest.settle_s[1]),
            ("lean.out_s", self.lean.out_s),
            ("lean.back_s", self.lean.back_s),
            ("mount.recoil_return_s", self.mount.recoil_return_s),
            ("corpses.fade_s", self.corpses.fade_s),
            (
                "attitude.rotor.smoothing_s",
                self.attitude.rotor.smoothing_s,
            ),
            (
                "attitude.fixed_wing.smoothing_s",
                self.attitude.fixed_wing.smoothing_s,
            ),
        ] {
            range(
                &format!("presentation.pose.{name}"),
                v,
                f64::MIN_POSITIVE,
                f64::MAX,
            )?;
        }
        for (name, v) in [
            ("fire_prone", self.stance.fire_prone),
            ("fire_stand", self.stance.fire_stand),
            ("pinned_kneel", self.stance.pinned_kneel),
            ("fall_back", self.stance.fall_back),
        ] {
            range(&format!("presentation.pose.stance.{name}"), v, 0.0, 1.0)?;
        }
        if self.stance.fire_prone + self.stance.fire_stand > 1.0 {
            return Err("presentation.pose.stance: firing shares must sum to at most 1".into());
        }
        if let Some(k) = &self.hull_kick {
            range(
                "presentation.pose.hull_kick.damping",
                k.damping,
                f64::MIN_POSITIVE,
                1.0 - f64::EPSILON,
            )?;
            range(
                "presentation.pose.hull_kick.hz",
                k.hz,
                f64::MIN_POSITIVE,
                f64::MAX,
            )?;
        }
        if let Some(k) = &self.soldier_kick {
            range(
                "presentation.pose.soldier_kick.return_s",
                k.return_s,
                f64::MIN_POSITIVE,
                f64::MAX,
            )?;
        }
        Ok(())
    }
}
