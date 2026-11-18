//! What a resolved catalog's entries must keep: each unit type's
//! components, ranges and body, each soldier kind and each prop type, and
//! the roster places and card labels they show.
use super::*;

/// The component rules one type must keep: known roles and soldier kinds,
/// mounts where its body carries them, optics on a turret, supply deployed.
pub(super) fn check(
    id: &str,
    t: &UnitType,
    roles: &BTreeMap<String, Role>,
    soldiers: &BTreeMap<String, SoldierKind>,
    props: &PropCatalog,
) -> Result<(), CatalogError> {
    let rule = |error: &str| {
        Err(CatalogError::Rule {
            id: id.to_string(),
            error: error.to_string(),
        })
    };
    if t.roles.is_empty() {
        return rule("a type needs at least one role");
    }
    if let Some(role) = t.roles.iter().find(|r| !roles.contains_key(*r)) {
        return Err(CatalogError::UnknownRole {
            id: id.to_string(),
            role: role.clone(),
        });
    }
    match &t.body {
        Body::Squad { slots } => {
            if slots.is_empty() {
                return rule("a squad needs at least one slot");
            }
            if let Some(s) = slots.iter().find(|s| !soldiers.contains_key(*s)) {
                return Err(CatalogError::UnknownSoldier {
                    id: id.to_string(),
                    soldier: s.clone(),
                });
            }
            if !t.mounts.is_empty() {
                return rule("a squad's mounts come from its soldiers");
            }
            if t.appearance.is_some() {
                return rule("a squad draws its soldiers' appearances, not its own");
            }
            if !matches!(t.mobility, Mobility::Foot { .. }) {
                return rule("a squad moves on foot");
            }
            if t.capabilities.carriage.is_some() {
                return rule("carriage requires a hull, not a squad");
            }
        }
        Body::Hull(h) => {
            if matches!(t.mobility, Mobility::Foot { .. }) {
                return rule("a hull does not move on foot");
            }
            let Some(wreck) = props.index(&h.wreck) else {
                return rule(&format!("its wreck {:?} is not a prop type", h.wreck));
            };
            if let Err(error) = props.check_placement(wreck, PropPlacement::Ordinary) {
                return rule(&format!("its wreck {:?}: {error}", h.wreck));
            }
            if t.appearance.is_none() {
                return rule("a hull needs an appearance");
            }
        }
    }
    if let Some(carriage) = t.capabilities.carriage {
        if t.hull().is_none() {
            return rule("carriage requires a hull");
        }
        if !matches!(carriage.seats, 3 | 8 | 16) {
            return rule("carriage seats must be 3, 8 or 16");
        }
    }
    if let Some(on) = &t.sensors.on {
        if !t.mounts.iter().any(|m| &m.id == on && m.turret) {
            return rule(&format!(
                "sensors.on names {on:?}, which is not a turret mount"
            ));
        }
    }
    if t.capabilities.supply.is_some() && t.capabilities.deploy.is_none() {
        return rule("supply serves only when deployed: it needs deploy");
    }
    // A hull's mounts: each on the hull or an earlier turret, firing from
    // its own muzzle, and neither a squad's weapon nor a soldier's.
    for (i, m) in t.mounts.iter().enumerate() {
        if let Some(arc) = m.arc {
            // Its arc is measured off the hull's heading, so the hull must
            // carry the turret that swings through it.
            if !m.turret || m.on.is_some() {
                return rule(&format!(
                    "mount {:?} has an arc but no turret on the hull to swing through it",
                    m.id
                ));
            }
            if !(arc.width_deg > 0.0 && arc.width_deg < 360.0 && arc.centre_deg.is_finite()) {
                return rule(&format!(
                    "mount {:?} arc width must lie between 0 and 360 degrees",
                    m.id
                ));
            }
        }
        if let Some(on) = &m.on {
            let Some(carrier) = t.mounts[..i].iter().find(|c| &c.id == on && c.turret) else {
                return rule(&format!(
                    "mount {:?} is on {on:?}, which is not an earlier turret mount",
                    m.id
                ));
            };
            // A carried mount swings round its carrier's ring, which turns
            // with the hull: one level of carriage, never a mount on a mount
            // on a turret.
            if carrier.on.is_some() {
                return rule(&format!(
                    "mount {:?} is on {on:?}, which is itself carried by a turret",
                    m.id
                ));
            }
        }
        if m.muzzle_m.is_none() {
            return rule(&format!("mount {:?} on a hull needs muzzle_m", m.id));
        }
        if m.operator_appearance.is_some() {
            return rule(&format!(
                "mount {:?} on a hull cannot name operator_appearance",
                m.id
            ));
        }
        if m.squad || m.special {
            return rule(&format!(
                "mount {:?} on a hull is neither squad nor special",
                m.id
            ));
        }
    }
    if let Body::Hull(h) = &t.body {
        // A hull shoves what is lighter than itself: none is so light it
        // shoves nothing, or so heavy nothing stops it.
        if matches!(
            h.weight_class,
            WeightClass::Trivial | WeightClass::Immovable
        ) {
            return rule(&format!(
                "a hull weighs light, medium or heavy, not {}",
                lower(&h.weight_class)
            ));
        }
        // A wreck keeps its live vehicle's cover (Q24).
        let row = props.by_id(&h.wreck).body.cover_tier;
        let want = h.weight_class.cover_tier();
        if row != want {
            let name = |t: Option<CoverTier>| t.map_or("no".into(), |t| lower(&t));
            return rule(&format!(
                "its wreck {:?} gives {} cover, not its {} weight's {} (Q24)",
                h.wreck,
                name(row),
                lower(&h.weight_class),
                name(want)
            ));
        }
        if !(h.half_extents_m.iter().all(|&e| positive(e)) && positive(h.hp)) {
            return rule("a hull needs finite positive extents and hp");
        }
        let armor = h.armor;
        if ![armor.front, armor.side, armor.rear, armor.roof]
            .into_iter()
            .all(at_least_zero)
        {
            return rule("body.hull.armor needs finite nonnegative protection on every face");
        }
        let chance = armor.ricochet;
        if ![chance.front, chance.side, chance.rear, chance.roof]
            .into_iter()
            .all(|p| p.is_finite() && (0.0..=1.0).contains(&p))
        {
            return rule("body.hull.armor.ricochet needs probabilities in [0, 1] on every face");
        }
    }
    let c = &t.capabilities;
    if let Some(Err(e)) = c.active_protection.as_ref().map(|p| p.validate(t.hull())) {
        return rule(e);
    }
    if let Some(air) = &t.capabilities.air {
        if !matches!(t.mobility, Mobility::Air { .. }) {
            return rule("capabilities.air requires air mobility");
        }
        if !positive(air.fuel_capacity_s) {
            return rule("capabilities.air.fuel_capacity_s must be positive");
        }
        if let Some(afterburner) = air.afterburner {
            if t.mobility.flight() != Some(FlightModel::FixedWing) {
                return rule(
                    "capabilities.air.afterburner needs fixed-wing flight: no rotor flight law flies it",
                );
            }
            if !(afterburner.speed_factor > 1.0
                && afterburner.fuel_factor > 1.0
                && afterburner.speed_factor.is_finite()
                && afterburner.fuel_factor.is_finite())
            {
                return rule(
                    "capabilities.air.afterburner multipliers must be finite and greater than one",
                );
            }
        }
        let refund = air.refund;
        if ![refund.health, refund.ammunition, refund.fuel]
            .into_iter()
            .all(at_least_zero)
            || (refund.health + refund.ammunition + refund.fuel - 1.0).abs() > 1e-6
        {
            return rule(
                "capabilities.air.refund weights must be finite, nonnegative and sum to one",
            );
        }
    }
    if let Some(Err(e)) = c.flares.map(|f| f.validate(t.hull())) {
        return rule(e);
    }
    if t.capabilities
        .radar_receiver
        .is_some_and(|receiver| !positive(receiver.range_m))
    {
        return rule("capabilities.radar_receiver range_m must be positive");
    }
    if let Some(Err(e)) = c.smoke.as_ref().map(SmokeCapability::validate) {
        return rule(&e);
    }
    ranges(t).map_or(Ok(()), rule)
}

/// A type's numbers in the ranges the simulation divides and eases by.
fn ranges(t: &UnitType) -> Option<&'static str> {
    let s = t.sensors.sight_shape;
    let (offroad_kmh, road_kmh) = t.mobility.speeds_kmh();
    if let Some(problem) = air_problem(&t.mobility) {
        return Some(problem);
    }
    if !matches!(t.mobility, Mobility::Air { .. })
        && !(offroad_kmh > 0.0 && road_kmh >= offroad_kmh && road_kmh <= MAX_SPEED_KMH)
    {
        return Some("mobility: speeds must satisfy 0 < offroad_kmh <= road_kmh <= 130");
    }
    let bad = match t.mobility {
        Mobility::Foot { .. } | Mobility::Air { .. } => None,
        Mobility::Tracked {
            turn_deg_s,
            reverse_speed_factor,
            ..
        }
        | Mobility::Wheeled {
            turn_deg_s,
            reverse_speed_factor,
            ..
        } => {
            if !positive(turn_deg_s) {
                Some("mobility: turn_deg_s must be positive")
            } else if !(reverse_speed_factor > 0.0 && reverse_speed_factor <= 1.0) {
                Some("mobility.reverse_speed_factor must lie in (0, 1]")
            } else {
                None
            }
        }
    };
    bad.or_else(|| match t.mobility {
        Mobility::Wheeled {
            turning_radius_m, ..
        } if !positive(turning_radius_m) => {
            Some("a wheeled vehicle needs a positive turning_radius_m")
        }
        _ => None,
    })
    .or_else(|| {
        (!at_least_zero(t.sound.loudness_m)).then_some("sound.loudness_m must not be negative")
    })
    .or_else(|| (!positive(t.sensors.ground_m)).then_some("sensors.ground_m must be positive"))
    .or_else(|| (!positive(t.concealment)).then_some("concealment must be finite and positive"))
    .or_else(|| {
        (!positive(t.sensors.spotting)).then_some("sensors.spotting must be finite and positive")
    })
    .or_else(|| {
        (![t.sensors.low_air_m, t.sensors.high_air_m]
            .into_iter()
            .all(at_least_zero))
        .then_some("sensors air ranges must be finite and nonnegative")
    })
    .or_else(|| {
        (!(positive(s.rear) && s.rear <= s.side && s.side <= s.front && s.front.is_finite()))
            .then_some("sensors.sight_shape must have 0 < rear <= side <= front")
    })
    .or_else(|| {
        t.capabilities
            .deploy
            .is_some_and(|d| !at_least_zero(d.seconds) || !at_least_zero(d.pack_seconds))
            .then_some("capabilities.deploy durations must not be negative")
    })
}

/// Infantry mounts follow one operator or are shared hand weapons, never
/// turrets or both a squad's and a special one.
pub(super) fn check_soldier(id: &str, s: &SoldierKind) -> Result<(), CatalogError> {
    let invalid = |error: String| {
        Err(CatalogError::Invalid {
            section: "soldiers",
            id: id.to_string(),
            error,
        })
    };
    if !positive(s.hp) {
        return invalid("hp must be positive".into());
    }
    for m in &s.mounts {
        if m.squad && m.operator_appearance.is_some() {
            return invalid(format!(
                "mount {:?}: operator_appearance needs a single operator",
                m.id
            ));
        }
        if let Some(appearance) = &m.operator_appearance {
            if appearance.active.is_empty()
                || appearance.carried.is_empty()
                || appearance.active.len() != appearance.carried.len()
            {
                return invalid(format!(
                    "mount {:?}: operator_appearance needs nonempty active and carried sets with equal variant counts",
                    m.id
                ));
            }
            if appearance.active_pose.as_ref().is_some_and(|p| {
                !matches!(p.clip.as_str(), "stand_aim" | "kneel_fire")
                    || !p.phase.is_finite()
                    || !(0.0..=1.0).contains(&p.phase)
            }) {
                return invalid(format!("mount {:?}: active_pose needs stand_aim or kneel_fire and a finite phase in [0, 1]", m.id));
            }
        }
        if m.squad && m.special {
            return invalid(format!(
                "mount {:?} is a squad weapon or a special one, not both",
                m.id
            ));
        }
        if m.turret
            || m.on.is_some()
            || (m.squad && (m.pivot_m != [0.0; 3] || m.muzzle_m.is_some()))
            || (m.muzzle_m.is_none() && m.pivot_m != [0.0; 3])
        {
            return invalid(format!(
                "mount {:?}: infantry cannot use turret/on, share bore offsets, or declare a pivot without a muzzle",
                m.id
            ));
        }
        if m.pivot_m
            .iter()
            .chain(m.muzzle_m.iter().flatten())
            .any(|x| !x.is_finite())
        {
            return invalid(format!(
                "mount {:?}: pivot_m and muzzle_m must be finite",
                m.id
            ));
        }
    }
    Ok(())
}

/// The rules one prop type must keep: `hp` and `destroyed` go together,
/// `hp` positive and `armor` in [0, 1]; only a toppling body's destroyed
/// state is cleared ground; and a destroyed state ends: each `into` names a
/// prop type, at a positive height, never back up its chain.
pub(super) fn check_prop(props: &PropCatalog, k: PropKind) -> Result<(), CatalogError> {
    let id = props.id(k);
    let t = props.get(k);
    let rule = |error: &str| {
        Err(CatalogError::Prop {
            id: id.to_string(),
            error: error.to_string(),
        })
    };
    let b = &t.body;
    if b.hp.is_some() != t.destroyed.is_some() {
        return rule("hp and destroyed go together");
    }
    if b.hp_scale == HpScale::BuildingFloorBands && b.hp.is_none() {
        return rule("building_floor_bands integrity requires hp");
    }
    if b.hp.is_some_and(|h| !h.is_finite() || h <= 0.0) || !(0.0..=1.0).contains(&b.armor) {
        return rule("hp must be positive and armor within [0, 1]");
    }
    if t.destroyed == Some(Destroyed::Cleared) && !b.topples {
        return rule("only a toppling body's destroyed state is cleared ground");
    }
    let mut state_type = t;
    let mut next = state_type.destroyed.as_ref();
    for _ in 0..=props.types.len() {
        let Some(Destroyed::Into {
            prop,
            height_m,
            building,
        }) = next
        else {
            return Ok(());
        };
        if !height_m.is_finite() || *height_m <= 0.0 {
            return rule("destroyed.into.height_m must be positive");
        }
        if let Some(building) = building {
            if !building.height_fraction.is_finite()
                || building.height_fraction <= 0.0
                || !building.max_height_m.is_finite()
                || building.max_height_m < *height_m
                || building.collapse_max_floors == 0
            {
                return rule("building remains require positive height fraction, bounded height, and floor threshold");
            }
            let Some(gutted) = props.index(&building.gutted_prop) else {
                return rule("building gutted_prop must name a prop type");
            };
            let shell = props.get(gutted);
            if shell.body.hp.is_some() || shell.body.garrison || shell.destroyed.is_some() {
                return rule("a gutted shell must be terminal and ungarrisonable");
            }
            if shell.body.blocks != state_type.body.blocks
                || shell.body.occludes != state_type.body.occludes
                || shell.body.stops_rounds != state_type.body.stops_rounds
                || shell.body.cover_tier != Some(CoverTier::Medium)
            {
                return rule("a gutted shell must keep blocking, sight and round stopping, with medium cover");
            }
        }
        let Some(into) = props.index(prop) else {
            return rule(&format!(
                "destroyed into {prop:?}, which is not a prop type"
            ));
        };
        state_type = props.get(into);
        next = state_type.destroyed.as_ref();
    }
    rule("its destroyed states loop")
}

pub(super) fn check_roster(id: &str, roster: &RosterMembership) -> Result<(), CatalogError> {
    let unique: std::collections::BTreeSet<_> = roster.factions.iter().collect();
    if unique.is_empty()
        || unique.len() != roster.factions.len()
        || roster.family_name.trim().is_empty()
        || roster.variant.trim().is_empty()
    {
        return Err(CatalogError::Invalid {
            section: "units",
            id: id.into(),
            error: "roster needs distinct factions, a family name and a variant name".into(),
        });
    }
    Ok(())
}

/// Each mount's label, by the field that holds it.
pub(super) fn mount_labels(mounts: &[MountDefinition]) -> Vec<(String, &str)> {
    mounts
        .iter()
        .map(|m| (format!("mounts.{}", m.id), m.name.as_str()))
        .collect()
}

/// What an entry's card shows: each label concise and each icon an icon id
/// (`crate::labels`), or the entry is refused naming the field (an empty
/// field is the entry's own `name`).
pub(super) fn check_card_labels(
    section: &'static str,
    id: &str,
    labels: &[(String, &str)],
    icons: &[(String, &str)],
) -> Result<(), CatalogError> {
    let refuse = |field: &str, error: String| CatalogError::Invalid {
        section,
        id: id.to_string(),
        error: if field.is_empty() {
            error
        } else {
            format!("{field}: {error}")
        },
    };
    for (field, label) in labels {
        crate::labels::check_label(label).map_err(|e| refuse(field, e))?;
    }
    for (field, icon) in icons {
        crate::labels::check_icon(icon).map_err(|e| refuse(field, e))?;
    }
    Ok(())
}
