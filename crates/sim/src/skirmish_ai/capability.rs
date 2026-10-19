//! What a unit's weapons can engage, by target class and altitude layer,
//! from weapon data: the AI's one owner for a unit's specialization.

use contract::catalog::{AltitudeLayer, TypeIndex};
use contract::observation::ObservationFrame;
use contract::scenario::Rules;
use contract::weapons::{TargetClass, WeaponDefinition};

pub(crate) fn unit_can_engage_target(rules: &Rules, kind: TypeIndex, target: TypeIndex) -> bool {
    let target = rules.catalog.get(target);
    let (layer, class) = (target.mobility.layer(), target.target_class());
    unit_weapons(rules, kind).any(|weapon| weapon.engages(layer, Some(class)).is_some())
}
/// Whether a unit has at least one authored weapon that can engage a ground
/// target. The target class comes from the weapon row; unit roles never decide
/// whether a detachment can answer a lane.
pub(crate) fn unit_can_engage_ground(rules: &Rules, kind: TypeIndex) -> bool {
    unit_weapons(rules, kind).any(|weapon| {
        [TargetClass::Infantry, TargetClass::Vehicle]
            .into_iter()
            .any(|class| weapon.engages(AltitudeLayer::Ground, Some(class)).is_some())
    })
}
/// The weapon rows `kind`'s mounts carry.
pub(super) fn unit_weapons(
    rules: &Rules,
    kind: TypeIndex,
) -> impl Iterator<Item = &WeaponDefinition> {
    rules
        .catalog
        .mounts(kind)
        .iter()
        .flat_map(|mount| &mount.def.weapons)
        .filter_map(|weapon| rules.weapons.get(weapon))
}
/// Whether `weapon` engages an aircraft in `layer`, of either flight.
pub(super) fn engages_aircraft(weapon: &WeaponDefinition, layer: AltitudeLayer) -> bool {
    [TargetClass::AirRotor, TargetClass::AirFixedWing]
        .into_iter()
        .any(|class| weapon.engages(layer, Some(class)).is_some())
}
/// Whether one of `kind`'s weapons engages an aircraft, of either flight,
/// in `layer`; a row that only shoots down missiles there does not.
pub fn unit_engages_aircraft_in(rules: &Rules, kind: TypeIndex, layer: AltitudeLayer) -> bool {
    unit_weapons(rules, kind).any(|weapon| engages_aircraft(weapon, layer))
}
pub(super) fn frame_has_layer_threat(
    frame: &ObservationFrame,
    rules: &Rules,
    layer: AltitudeLayer,
) -> bool {
    frame
        .identified
        .iter()
        .any(|unit| rules.catalog.get(unit.kind).mobility.layer() == layer)
        || frame.contacts.iter().any(|contact| contact.layer == layer)
}
