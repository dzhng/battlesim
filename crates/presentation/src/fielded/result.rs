//! Immutable resolved descriptions, grown from new kinds rather than rebuilt.
use super::*;

pub(super) struct Resolved {
    settings: PresentationSettings,
    looks: crate::looks::WeaponLooks,
    pub(super) kinds: BTreeSet<String>,
    physical: BTreeMap<String, Arc<crate::world::UnitType>>,
    fielded: Fielded,
}

pub(super) fn resolve(
    request: &Fielding<'_>,
    facts: &mut Facts,
    read: &Read<'_>,
) -> Result<Fielded, String> {
    let mut state = match facts.resolved.take() {
        Some(state) => state,
        None => initial(request)?,
    };
    // Failed growth never publishes a partial result. Content facts can remain:
    // they are immutable and named by content, independent of a candidate.
    let result = grow(request, facts, read, &mut state);
    facts.resolved = Some(state);
    result
}

fn initial(request: &Fielding<'_>) -> Result<Resolved, String> {
    let settings = PresentationSettings::from_value(request.presentation)?;
    let catalog = Catalog::new(request.catalog)?;
    let looks =
        crate::looks::WeaponLooks::new(settings.weapon_looks.clone(), &request.rules["weapons"])?;
    looks.check_mounts(request.catalog)?;
    let art = Art::new(request.runtime, &catalog, &[], request.factions)?;
    let (mut types, _) = unit_types(&catalog, &art, &[], request.factions, catalog.ids())?;
    resolve_weapons(&mut types, &looks);
    let body_half: BTreeMap<_, _> = catalog
        .ids()
        .map(|kind| {
            let half = catalog
                .hull(kind)?
                .map(|h| h["half_extents_m"].clone())
                .unwrap_or(Value::Null);
            Ok((kind, half))
        })
        .collect::<Result<_, String>>()?;
    let sides = &request.runtime["sides"];
    let units: UnitsSection = serde_json::from_value(json!({
        "pose": settings.pose, "transit": settings.transit,
        "leanHold": number(request.rules, "cover", "lean_hold_s")?,
        "detail": settings.models, "tints": {"blue": sides["blue"], "red": sides["red"]},
        "bodyHalf": body_half, "types": types, "appearances": {},
    }))
    .map_err(|e| format!("units: {e}"))?;
    let effects = effects::initial(&settings, &looks, &catalog)?;
    Ok(Resolved {
        settings,
        looks,
        kinds: BTreeSet::new(),
        physical: units.types.clone(),
        fielded: Fielded {
            units,
            effects,
            tables: ClipTables::new(),
            work: FieldingWork::default(),
        },
    })
}

fn resolve_weapons(types: &mut Map<String, Value>, looks: &crate::looks::WeaponLooks) {
    for unit in types.values_mut() {
        for mount in unit["mounts"].as_array_mut().unwrap() {
            mount["weapons"] = Value::Array(mount["weapons"].as_array().unwrap().iter().map(|id| {
                let weapon = id.as_str().unwrap();
                json!({"kind": weapon, "kick": looks.get(weapon).kick.expect("fielded firing kick")})
            }).collect());
        }
    }
}

fn grow(
    request: &Fielding<'_>,
    facts: &mut Facts,
    read: &Read<'_>,
    state: &mut Resolved,
) -> Result<Fielded, String> {
    let added: Vec<_> = request
        .kinds
        .iter()
        .filter(|k| !state.kinds.contains(*k))
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let catalog = Catalog::new(request.catalog)?;
    effects::check_smoke_screen_looks(&state.settings, &catalog, &added)?;
    let art = Art::new(request.runtime, &catalog, &added, request.factions)?;
    let growth = Fielding {
        kinds: &added,
        ..*request
    };
    let bytes = growth
        .contents(facts)?
        .into_iter()
        .map(|hash| {
            let bytes = read(&hash)?;
            Ok((hash, bytes))
        })
        .collect::<Result<HashMap<_, _>, String>>()?;
    let mut candidate = state.fielded.clone();
    candidate.work = FieldingWork {
        kinds: added.len(),
        appearances: 0,
    };
    let (mut types, drawn) = unit_types(
        &catalog,
        &art,
        &added,
        request.factions,
        added.iter().map(String::as_str),
    )?;
    resolve_weapons(&mut types, &state.looks);
    for (kind, value) in types {
        candidate.units.types.insert(
            kind,
            Arc::new(serde_json::from_value(value).map_err(|e| format!("unit type: {e}"))?),
        );
    }
    for name in drawn {
        if candidate.units.appearances.contains_key(&name) {
            continue;
        }
        let appearance = appearance_facts(facts, &art, &bytes, &name)?;
        if let Some((key, table)) = appearance.table {
            candidate.tables.entry(key).or_insert(table);
        }
        candidate.units.appearances.insert(name, appearance.record);
        candidate.work.appearances += 1;
    }
    let mut bundle_facts = BundleFacts {
        facts,
        bytes: &bytes,
    };
    effects::extend(
        &mut candidate.effects,
        request.rules,
        &catalog,
        &art,
        &mut bundle_facts,
        &added,
    )?;
    state.kinds.extend(added);
    state.fielded = candidate.clone();
    // A caller may narrow its kinds again. Retain the descriptions in this owner
    // for reuse, but give the snapshot only the art and effects it requested.
    let wanted: BTreeSet<_> = request.kinds.iter().map(String::as_str).collect();
    if wanted.len() == state.kinds.len() && wanted.iter().all(|k| state.kinds.contains(*k)) {
        return Ok(candidate);
    }
    for (kind, physical) in &state.physical {
        if !wanted.contains(kind.as_str()) {
            candidate.units.types.insert(kind.clone(), physical.clone());
        }
    }
    candidate
        .effects
        .types
        .retain(|k, _| wanted.contains(k.as_str()));
    candidate
        .effects
        .wreck_looks
        .retain(|k, _| wanted.contains(k.as_str()));
    // Collect routing names from the retained rows, without rebuilding them.
    let mut drawn = BTreeSet::new();
    for kind in request.kinds {
        for sets in candidate.units.types[kind].draws.values() {
            drawn.extend(sets.hull.iter().cloned());
            drawn.extend(sets.slots.iter().flatten().cloned());
            for kit in sets.operated.iter().flatten() {
                drawn.extend(kit.active.iter().chain(&kit.carried).cloned());
            }
        }
    }
    candidate
        .units
        .appearances
        .retain(|name, _| drawn.contains(name));
    candidate.tables.retain(|key, _| {
        candidate.units.appearances.values().any(
            |a| matches!(a.as_ref(), crate::world::Appearance::Skinned {table,..} if table == key),
        )
    });
    let wrecks: BTreeSet<_> = candidate
        .effects
        .wreck_looks
        .values()
        .map(|l| l.appearance.as_str())
        .collect();
    candidate
        .effects
        .wrecks
        .retain(|name, _| wrecks.contains(name.as_str()));
    Ok(candidate)
}
