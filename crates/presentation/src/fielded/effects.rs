//! The `effects` section a battle fields: the effect tables with each
//! weapon's rows resolved, each fielded hull and its muzzles, what smokes
//! once known, and how each installed hull dies.
use super::*;

/// The `effects` section: the effects' tuning, each
/// fielded type's hull and mount muzzles, the prop kinds that smoke once
/// known, and each installed hull's wreck: what a cook-off throws, how it is
/// fitted and how hard the dead hull stops.
pub(super) fn section(
    presentation: &PresentationSettings,
    weapon_looks: &crate::looks::WeaponLooks,
    rules: &Value,
    catalog: &Catalog,
    art: &Art,
    bundle_facts: &mut BundleFacts,
    kinds: &[String],
) -> Result<Value, String> {
    let vec3 = |v: &Value| -> Value {
        match v.as_array() {
            Some(a) if a.len() >= 3 => json!([a[0], a[1], a[2]]),
            _ => Value::Null,
        }
    };
    let mut types = Map::new();
    for kind in kinds.iter().collect::<BTreeSet<_>>() {
        let rows: &[Value] = catalog.get(kind)?["mounts"]
            .as_array()
            .map_or(&[], Vec::as_slice);
        let muzzles: Vec<Value> = rows
            .iter()
            .map(|row| {
                if row["muzzle_m"].is_null() {
                    return Value::Null;
                }
                let on = rows.iter().position(|c| !row["on"].is_null() && c["id"] == row["on"]);
                let ring = on.map_or(json!([0, 0, 0]), |c| {
                    json!([rows[c]["pivot_m"][0], rows[c]["pivot_m"][1], 0])
                });
                json!({ "on": on, "pivot": vec3(&row["pivot_m"]), "ring": ring, "muzzle": vec3(&row["muzzle_m"]) })
            })
            .collect();
        let half = catalog
            .hull(kind)?
            .map_or(Value::Null, |h| vec3(&h["half_extents_m"]));
        types.insert(kind.clone(), json!({ "half": half, "muzzles": muzzles }));
    }
    let mut smokes = Map::new();
    let (remains, shells) = building_remains(catalog.view);
    for kind in remains {
        smokes.insert(kind, json!("ruin"));
    }
    for kind in shells {
        smokes.insert(kind, json!("gutted"));
    }
    for id in catalog.ids() {
        if let Some(wreck) = catalog.hull(id)?.and_then(|h| h["wreck"].as_str()) {
            smokes.insert(wreck.to_owned(), json!("wreck"));
        }
    }
    let wreck_stop_s = number(rules, "movement.drive", "wreck_stop_s")?;
    let (mut wrecks, mut looks) = (Map::new(), Map::new());
    for (unit, name) in hull_wrecks(catalog, art) {
        let name = name.as_str();
        let footprint = &art.entry(name)["footprint_half_m"];
        let wreck = wreck_facts(bundle_facts.facts, art, bundle_facts.bytes, name)?;
        // A wreck with no whole state makes no transition.
        let (Some(bounds), false) = (&wreck.bounds, footprint.is_null()) else {
            continue;
        };
        wrecks.insert(
            name.to_owned(),
            json!({
                "lies": wreck.lies,
                "underside": wreck.underside,
                "debrisTop": wreck.debris_top,
            }),
        );
        looks.insert(
            unit.to_owned(),
            json!({
                "appearance": name,
                "footprint": vec3(footprint),
                "bounds": [vec3(&bounds[0]), vec3(&bounds[1])],
                "braking": top_speed_kmh(catalog.get(&unit)?) / 3.6 / wreck_stop_s,
            }),
        );
    }
    let effect_settings = {
        let mut effects = serde_json::to_value(&presentation.effects).map_err(|e| e.to_string())?;
        effects["looks"] = serde_json::to_value(weapon_looks).map_err(|e| e.to_string())?;
        effects["smoke_screen_looks"] =
            serde_json::to_value(&presentation.smoke_screen_looks).map_err(|e| e.to_string())?;
        effects
    };
    Ok(json!({
        "presentation": effect_settings,
        "types": types,
        "smokes": smokes,
        "wrecks": wrecks,
        "wreckLooks": looks,
    }))
}

fn wreck_facts(
    facts: &mut Facts,
    art: &Art,
    bytes: &HashMap<String, Vec<u8>>,
    name: &str,
) -> Result<WreckFacts, String> {
    let hash = art.bundle(name)?.to_owned();
    if let Some(facts) = facts.wrecks.get(&hash) {
        return Ok(facts.clone());
    }
    let bytes = bytes
        .get(&hash)
        .ok_or(format!("content {hash} was not read"))?;
    let bundle = Container::<Value>::parse(bytes, BUNDLE, name)?;
    let states: &[Value] = match bundle.content["kind"].as_str() {
        Some("static") => bundle.content["states"]
            .as_array()
            .map_or(&[], Vec::as_slice),
        _ => &[],
    };
    let state = |n: &str| states.iter().find(|s| s["name"] == n);
    let corner = |b: &Value, c: usize| b[c].as_f64().unwrap_or(0.0);
    let whole = state("default").map(|state| {
        [
            state["bounds"]["min"].clone(),
            state["bounds"]["max"].clone(),
        ]
    });
    let turret = state(WRECK_TURRET).filter(|_| state(WRECK_HULL).is_some());
    let derived = WreckFacts {
        bounds: whole,
        lies: turret.map(|t| {
            [0, 1, 2]
                .map(|c| (corner(&t["bounds"]["min"], c) + corner(&t["bounds"]["max"], c)) / 2.0)
        }),
        underside: turret.map_or(0.0, |t| corner(&t["bounds"]["min"], 2)),
        debris_top: state(WRECK_DEBRIS).map(|d| corner(&d["bounds"]["max"], 2)),
    };
    facts.wrecks.insert(hash, derived.clone());
    Ok(derived)
}

/// Every fielded type that carries smoke names a `presentation.smoke_screen_looks`
/// row for its cloud.
pub(super) fn check_smoke_screen_looks(
    presentation: &PresentationSettings,
    catalog: &Catalog,
    kinds: &[String],
) -> Result<(), String> {
    for kind in kinds {
        let look = &catalog.get(kind)?["capabilities"]["smoke"]["cloud"]["look"];
        if let Some(look) = look.as_str() {
            if !presentation.smoke_screen_looks.contains_key(look) {
                return Err(format!(
                    "unit type {kind}: its smoke cloud's look {look:?} has no presentation.smoke_screen_looks row"
                ));
            }
        }
    }
    Ok(())
}
