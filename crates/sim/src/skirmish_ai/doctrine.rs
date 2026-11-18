//! Doctrine transitions: remembered threats, reaction delays and dwell hysteresis.

use contract::catalog::AltitudeLayer;
use contract::observation::ObservationFrame;
use contract::scenario::Rules;

use super::{frame_has_layer_threat, BalancedDoctrine, Tuning};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Doctrine {
    #[default]
    BalancedOpening,
    ReconScreen,
    InfantryPressure,
    TankSwarm,
    AirDenial,
    EmergencyDefense,
    Consolidate,
}

pub(super) fn update(
    ai: &mut BalancedDoctrine,
    frame: &ObservationFrame,
    rules: &Rules,
    draw: f64,
) {
    if !ai.started {
        ai.started = true;
        ai.doctrine_since = frame.tick;
    }
    // Reaction delay is a personality trait, not a licence to ignore a threat.
    if ai.response_delay_ticks.is_none() {
        ai.response_delay_ticks =
            Some(1 + Tuning::ticks(draw * ai.tuning.reaction_max_s, rules.tick_hz));
    }
    let low_air = frame_has_layer_threat(frame, rules, AltitudeLayer::LowAir);
    let own_count = frame.own.len();
    let lost_unit = ai
        .last_own_count
        .is_some_and(|previous| own_count < previous);
    ai.last_own_count = Some(own_count);
    if low_air {
        ai.last_air_evidence.get_or_insert(frame.tick);
    } else if ai.last_air_evidence.is_some_and(|seen| {
        frame.tick.saturating_sub(seen) > Tuning::ticks(ai.tuning.air_memory_s, rules.tick_hz)
    }) {
        ai.last_air_evidence = None;
    }
    let min_dwell = Tuning::ticks(ai.tuning.min_dwell_s, rules.tick_hz);
    let max_dwell = Tuning::ticks(ai.tuning.max_dwell_s, rules.tick_hz);
    let dwell = frame.tick.saturating_sub(ai.doctrine_since);
    let delayed_air = ai.last_air_evidence.is_some_and(|seen| {
        frame.tick.saturating_sub(seen)
            >= ai.response_delay_ticks.expect("reaction delay initialized")
    });
    let next = if lost_unit && dwell >= min_dwell {
        Some(Doctrine::EmergencyDefense)
    } else if delayed_air && dwell >= min_dwell {
        Some(Doctrine::AirDenial)
    } else if dwell >= max_dwell {
        Some(match ai.doctrine {
            Doctrine::BalancedOpening => Doctrine::ReconScreen,
            Doctrine::ReconScreen => Doctrine::InfantryPressure,
            Doctrine::InfantryPressure => Doctrine::TankSwarm,
            Doctrine::TankSwarm => Doctrine::Consolidate,
            Doctrine::AirDenial => Doctrine::Consolidate,
            Doctrine::Consolidate => Doctrine::ReconScreen,
            Doctrine::EmergencyDefense => Doctrine::ReconScreen,
        })
    } else {
        None
    };
    if let Some(next) = next.filter(|next| *next != ai.doctrine) {
        ai.doctrine = next;
        ai.doctrine_since = frame.tick;
        ai.pivot_count = ai.pivot_count.saturating_add(1);
    }
}
