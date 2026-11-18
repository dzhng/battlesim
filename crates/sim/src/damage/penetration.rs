use contract::scenario::{Armor, Face, RicochetRules, Rules};
use contract::weapons::WeaponDefinition;

/// A round's remaining penetration after ricochets.
pub(super) fn reach(penetration: f64, bounces: u8, rules: &RicochetRules) -> f64 {
    penetration * rules.penetration_kept.powi(bounces as i32)
}

/// The share of a round's damage delivered to the face it hits.
pub fn pierce_share(
    def: &WeaponDefinition,
    bounces: u8,
    rules: &Rules,
    armor: &Armor,
    face: Face,
) -> f64 {
    let penetration = reach(def.penetration, bounces, &rules.ricochet);
    let scale = def
        .partial_damage_scale
        .map_or(1.0, |scale| scale.face(face));
    share(penetration, armor.face(face), def.partial_from, scale)
}

/// Whether a fresh round can deal damage on any face.
pub(crate) fn can_damage_hull(def: &WeaponDefinition, armor: &Armor, rules: &Rules) -> bool {
    if def.partial_damage_scale.is_none() {
        return share(def.penetration, armor.weakest(), def.partial_from, 1.0) > 0.0;
    }
    [Face::Front, Face::Side, Face::Rear, Face::Roof]
        .into_iter()
        .any(|face| pierce_share(def, 0, rules, armor, face) > 0.0)
}

fn share(penetration: f64, protection: f64, from: f64, partial_scale: f64) -> f64 {
    if penetration >= protection {
        return 1.0;
    }
    ((penetration / protection - from) / (1.0 - from)).max(0.0) * partial_scale
}
