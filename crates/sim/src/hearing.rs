//! Sound as evidence: every living unit makes its own sound (its body row's
//! `sound` and `loudness_m`), louder when
//! moving; shots are their own category. Only a friendly listener within the
//! category's hearing range turns an unseen enemy's sound into a cue, and the
//! cue carries a broad direction and distance band, never a position.
use std::collections::BTreeSet;

use contract::ids::{Side, UnitId};
use contract::observation::{MoveState, SoundBand, SoundCategory, SoundCue};
use contract::scenario::Rules;

use crate::knowledge::SideKnowledge;
use crate::units::Unit;

/// Cues `side` hears this bucket, one per unseen source and category.
pub fn hear(
    side: Side,
    tick: u64,
    units: &[Unit],
    knowledge: &SideKnowledge,
    fired: &BTreeSet<UnitId>,
    rules: &Rules,
) -> Vec<SoundCue> {
    let s = &rules.sensors;
    let mut cues = Vec::new();
    let listeners: Vec<&Unit> = units
        .iter()
        .filter(|u| u.side == side && u.alive())
        .collect();
    for source in units.iter().filter(|u| u.side != side && u.alive()) {
        if knowledge.identifies(source.id, tick) {
            continue; // seen units need no ears
        }
        // Each mover's own loudness (its body row), not whether it has a hull.
        let body = crate::units::body(source.kind, rules);
        let base = (body.sound, body.loudness_m);
        let shot = fired
            .contains(&source.id)
            .then_some((SoundCategory::Shot, s.hearing_shot_m));
        for (category, range) in std::iter::once(base).chain(shot) {
            let nearest = listeners
                .iter()
                .map(|l| (l, (source.position.xy() - l.position.xy()).length()))
                .filter(|(_, d)| *d <= range)
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.id.cmp(&b.0.id)));
            let Some((listener, distance)) = nearest else {
                continue;
            };
            let to = source.position.xy() - listener.position.xy();
            let angle = to.y.atan2(to.x).rem_euclid(std::f64::consts::TAU);
            cues.push(SoundCue {
                listener: listener.id,
                category,
                sector: ((angle / std::f64::consts::FRAC_PI_4).round() as u8) % 8,
                band: if distance <= range / 2.0 {
                    SoundBand::Near
                } else {
                    SoundBand::Far
                },
                moving: category != SoundCategory::Shot && source.state == MoveState::Moving,
            });
        }
    }
    cues
}
