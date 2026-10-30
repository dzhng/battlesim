//! Finite physical ranges of the contract's rule sections.
use super::*;

pub(super) fn positive(prefix: &str, fields: &[(&str, f64)]) -> Result<(), RulesError> {
    for &(field, value) in fields {
        if !value.is_finite() || value <= 0.0 {
            return Err(RulesError::new(
                format!("{prefix}.{field}"),
                "must be finite and positive",
            ));
        }
    }
    Ok(())
}

pub(super) fn nonnegative(prefix: &str, fields: &[(&str, f64)]) -> Result<(), RulesError> {
    for &(field, value) in fields {
        if !value.is_finite() || value < 0.0 {
            return Err(RulesError::new(
                format!("{prefix}.{field}"),
                "must be finite and nonnegative",
            ));
        }
    }
    Ok(())
}

pub(super) fn share(path: &str, value: f64, include_one: bool) -> Result<(), RulesError> {
    if (0.0..1.0).contains(&value) || (include_one && value == 1.0) {
        Ok(())
    } else {
        Err(RulesError::new(
            path,
            if include_one {
                "must lie in [0, 1]"
            } else {
                "must lie in [0, 1)"
            },
        ))
    }
}

impl CoverRules {
    pub(super) fn check(&self) -> Result<(), RulesError> {
        positive(
            "cover",
            &[
                ("reach_m", self.reach_m),
                ("search_m", self.search_m),
                ("step_out_m", self.step_out_m),
                ("reresolve_s", self.reresolve_s),
                ("vehicle_moved_m", self.vehicle_moved_m),
                ("lean_burst_s", self.lean_burst_s),
                ("lean_tuck_s", self.lean_tuck_s),
                ("crater_min_fill", self.crater_min_fill),
                ("lean_hold_s", self.lean_hold_s),
                ("lean_max_m", self.lean_max_m),
                ("lean_apart_m", self.lean_apart_m),
            ],
        )?;
        nonnegative(
            "cover",
            &[
                ("lean_clear_m", self.lean_clear_m),
                ("standoff_m", self.standoff_m),
                ("search_slack_m", self.search_slack_m),
            ],
        )?;
        if !(-1.0..=0.0).contains(&self.away_cos) {
            return Err(RulesError::new("cover.away_cos", "must lie in [-1, 0]"));
        }
        if !self.building_spread_multiplier.is_finite() || self.building_spread_multiplier < 1.0 {
            return Err(RulesError::new(
                "cover.building_spread_multiplier",
                "must be finite and at least 1",
            ));
        }
        share(
            "cover.building_fragment_probability_multiplier",
            self.building_fragment_probability_multiplier,
            true,
        )?;
        let mut previous = 1.0;
        for (name, value) in [
            ("light", self.tiers.light),
            ("medium", self.tiers.medium),
            ("heavy", self.tiers.heavy),
        ] {
            if !value.is_finite() || value < previous {
                return Err(RulesError::new(
                    format!("cover.tiers.{name}"),
                    "must widen spread by at least 1, heavier tiers no less",
                ));
            }
            previous = value;
        }
        Ok(())
    }
}

impl GroundRules {
    pub(super) fn check(&self) -> Result<(), RulesError> {
        positive(
            "ground",
            &[
                ("cell_m", self.cell_m),
                ("mark_reference_blast_m", self.mark_reference_blast_m),
            ],
        )?;
        if !(self.crater_full_depth > 0.0 && self.crater_full_depth <= u8::MAX as f64) {
            return Err(RulesError::new(
                "ground.crater_full_depth",
                "must lie in (0, 255]",
            ));
        }
        share("ground.track_gauge", self.track_gauge, true)?;
        nonnegative("ground", &[("lane_margin_m", self.lane_margin_m)])
    }
}

impl MovementRules {
    pub(super) fn check(&self) -> Result<(), RulesError> {
        self.speed_factors
            .check()
            .map_err(|e| RulesError::new("movement", e))?;
        let d = self.drive;
        positive(
            "movement.drive",
            &[
                ("acceleration_s", d.acceleration_s),
                ("braking_s", d.braking_s),
                ("wreck_stop_s", d.wreck_stop_s),
                ("turn_in_place_deg", d.turn_in_place_deg),
                ("abeam_m", d.abeam_m),
                ("abeam_deg", d.abeam_deg),
                ("turning_deg", d.turning_deg),
                ("turn_speed_factor", d.turn_speed_factor),
            ],
        )?;
        share(
            "movement.drive.turn_speed_factor",
            d.turn_speed_factor,
            true,
        )?;
        nonnegative(
            "movement.drive",
            &[
                ("min_leg_m", d.min_leg_m),
                ("circle_margin_m", d.circle_margin_m),
            ],
        )
    }
}

impl RicochetRules {
    pub(super) fn check(&self) -> Result<(), RulesError> {
        positive("ricochet", &[("speed_kept", self.speed_kept)])?;
        share("ricochet.speed_kept", self.speed_kept, true)?;
        share("ricochet.penetration_kept", self.penetration_kept, true)?;
        if !(0.0..90.0).contains(&self.scatter_deg) {
            return Err(RulesError::new(
                "ricochet.scatter_deg",
                "must lie in [0, 90)",
            ));
        }
        Ok(())
    }
}

impl BodyRules {
    pub(super) fn check(&self) -> Result<(), RulesError> {
        positive(
            "physics",
            &[(
                "vehicle_aim_height_fraction",
                self.vehicle_aim_height_fraction,
            )],
        )?;
        share(
            "physics.vehicle_aim_height_fraction",
            self.vehicle_aim_height_fraction,
            true,
        )
    }
}

impl crate::ballistics::GuidedRules {
    pub(super) fn check(&self) -> Result<(), RulesError> {
        positive("guided", &[("release_coast_s", self.release_coast_s)])?;
        nonnegative("guided", &[("flare_meet_m", self.flare_meet_m)])
    }
}

impl BailOutRow {
    pub(super) fn check(&self, host: &str) -> Result<(), RulesError> {
        let prefix = format!("bail_out.{host}");
        share(&format!("{prefix}.death"), self.death, true)?;
        if let Some(chance) = self.squad_loss {
            share(&format!("{prefix}.squad_loss"), chance, true)?;
        }
        nonnegative(
            &prefix,
            &[
                ("hp_lost[0]", self.hp_lost[0]),
                ("hp_lost[1]", self.hp_lost[1]),
                ("search_m", self.search_m),
            ],
        )?;
        if self.hp_lost[0] > self.hp_lost[1] {
            return Err(RulesError::new(
                format!("{prefix}.hp_lost"),
                "must be an ordered injury interval",
            ));
        }
        Ok(())
    }
}

impl GarrisonRules {
    pub(super) fn check(&self) -> Result<(), RulesError> {
        positive("garrison", &[("enter_exit_s", self.enter_exit_s)])?;
        nonnegative(
            "garrison",
            &[
                ("entry_distance_m", self.entry_distance_m),
                ("slot_standoff_m", self.slot_standoff_m),
                ("window_swap_s", self.window_swap_s),
                ("window_hold_s", self.window_hold_s),
            ],
        )?;
        if !(0.0..90.0).contains(&self.slot_facing_min_deg) {
            return Err(RulesError::new(
                "garrison.slot_facing_min_deg",
                "must lie in [0, 90)",
            ));
        }
        Ok(())
    }
}

impl PushingRules {
    pub(super) fn check(&self) -> Result<(), RulesError> {
        nonnegative(
            "pushing",
            &[
                ("turn_deg_per_m", self.turn_deg_per_m),
                ("relearn_m", self.relearn_m),
            ],
        )
    }
}

impl LaunchRules {
    pub(super) fn check(&self) -> Result<(), RulesError> {
        nonnegative("launch", &[("clearance_m", self.clearance_m)])
    }
}

impl InfantryMovementRules {
    pub(super) fn check(&self) -> Result<(), RulesError> {
        let prefix = "infantry_movement";
        positive(
            prefix,
            &[
                ("wander_period_s", self.wander_period_s),
                ("spread_squad_size", self.spread_squad_size as f64),
                ("steer_ahead_m", self.steer_ahead_m),
                ("lane_lookahead_m", self.lane_lookahead_m),
                ("window_m", self.window_m),
            ],
        )?;
        nonnegative(
            prefix,
            &[
                ("path_clearance_m", self.path_clearance_m),
                ("spread_m", self.spread_m),
                ("spacing_m", self.spacing_m),
                ("personal_space_m", self.personal_space_m),
                ("lane_shift_mps", self.lane_shift_mps),
                ("wander_m", self.wander_m),
                ("stagger_s", self.stagger_s),
                ("final_leg_m", self.final_leg_m),
                ("yield_horizon_s", self.yield_horizon_s),
                ("yield_margin_m", self.yield_margin_m),
            ],
        )?;
        share(
            "infantry_movement.pace_variation",
            self.pace_variation,
            false,
        )
    }
}
