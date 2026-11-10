//! What the effects read off this frame's aircraft as they are drawn: the
//! shadow beneath each high-air one, its own and the side's edge transits
//! alike, and each running jet's nozzles and wingtips for its exhaust.
use crate::effects::exhaust::DrawnJet;
use crate::effects::frame::EffectFrame;
use crate::effects::high_air_shadow::{high_air_ground_shadow, GroundShadow};
use crate::effects::transit::TransitPose;
use crate::static_world::ground::Ground;
use crate::units::drawn::DrawnModels;
use crate::units::pose::PoseFrame;
use crate::world::{Appearance, UnitsSection};

/// The shadow beneath every high-air aircraft drawn: the side's flying
/// units and its edge transits (own-side aircraft records reconstructed from
/// the publication, drawn as a live aircraft of their kind is) on `side`.
pub(super) fn high_air_shadows(
    effects: &EffectFrame,
    poses: &PoseFrame,
    transits: &[TransitPose],
    units: &UnitsSection,
    side: &str,
    ground: Option<&Ground>,
    out: &mut Vec<GroundShadow>,
) {
    let height = |x: f64, y: f64| ground.map_or(0.0, |g| g.source.grid.height(x, y));
    let flying = poses
        .vehicles
        .iter()
        .filter(|v| v.airborne)
        .map(|v| (&*v.kind, &*v.side, v.unit, v.position, v.yaw))
        .chain(
            transits
                .iter()
                .map(|t| (&*t.kind, side, 0.0, t.position, t.heading)),
        );
    let style = effects.presentation().high_air_shadow;
    out.clear();
    out.extend(
        flying
            .filter(|(kind, ..)| units.types.get(*kind).is_some_and(|t| t.high_air()))
            .filter_map(|(kind, on, unit, at, yaw)| {
                let name = units.resolve(kind, on, unit, 0, None, None)?;
                let Some(Appearance::Articulated { plan, .. }) = units.appearances.get(name) else {
                    return None;
                };
                Some(high_air_ground_shadow(at, yaw, *plan, height, &style))
            }),
    );
}

/// Every running jet as drawn at `time`, for its glow, flame and trails;
/// `jets` is scratch.
pub(super) fn observe_jets(
    effects: &mut EffectFrame,
    drawn: &mut DrawnModels,
    time: f64,
    jets: &mut Vec<DrawnJet>,
) {
    if let Some(style) = &effects.presentation().jet_exhaust {
        drawn.jets(&style.airframes, jets);
    }
    effects.observe_jets(time, jets);
}
