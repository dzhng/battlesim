//! The `effects` section a battle fields: the effect tables with each
//! weapon's rows resolved, each fielded hull and its muzzles, what smokes
//! once known, and how each installed hull dies.
use super::*;

/// `presentation.effects` with a row for every weapon of `weapons` that has
/// none in the tables each weapon's round reads: its nearest ancestor's.
fn weapon_effects(presentation: &PresentationSettings, weapons: &Value) -> Result<Value, String> {
    let mut effects = serde_json::to_value(&presentation.effects).map_err(|e| e.to_string())?;
    let parents: BTreeMap<String, Option<String>> = weapons
        .as_object()
        .ok_or("rules.json has no weapons")?
        .iter()
        .map(|(k, w)| (k.clone(), w["extends"].as_str().map(str::to_owned)))
        .collect();
    for table in ["tracers", "flashes", "impact_scale", "blast_scale"] {
        let mut rows: HashMap<String, Value> = effects[table]
            .as_object()
            .ok_or(format!("presentation.effects.{table} is not a table"))?
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        inherit(&mut rows, &parents);
        effects[table] = Value::Object(rows.into_iter().collect());
    }
    Ok(effects)
}

/// The `effects` section: the effects' tuning, each
/// fielded type's hull and mount muzzles, the prop kinds that smoke once
/// known, and each installed hull's wreck: what a cook-off throws, how it is
/// fitted and how hard the dead hull stops.
pub(super) fn section(
    presentation: &PresentationSettings,
    weapons: &Value,
    rules: &Value,
    catalog: &Catalog,
    art: &Art,
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
        let bytes = art.bytes(art.bundle(name)?)?;
        let bundle = Container::<Value>::parse(&bytes, BUNDLE, name)?;
        let states: &[Value] = match bundle.content["kind"].as_str() {
            Some("static") => bundle.content["states"]
                .as_array()
                .map_or(&[], Vec::as_slice),
            _ => &[],
        };
        let state = |n: &str| states.iter().find(|s| s["name"] == n);
        // A wreck with no whole state makes no transition.
        let (Some(whole), false) = (state("default"), footprint.is_null()) else {
            continue;
        };
        let turret = state(WRECK_TURRET).filter(|_| state(WRECK_HULL).is_some());
        let corner = |b: &Value, c: usize| b[c].as_f64().unwrap_or(0.0);
        wrecks.insert(
            name.to_owned(),
            json!({
                "lies": turret.map(|t| {
                    let (min, max) = (&t["bounds"]["min"], &t["bounds"]["max"]);
                    [0, 1, 2].map(|c| (corner(min, c) + corner(max, c)) / 2.0)
                }),
                "underside": turret.map_or(0.0, |t| corner(&t["bounds"]["min"], 2)),
                "debrisTop": state(WRECK_DEBRIS).map(|d| corner(&d["bounds"]["max"], 2)),
            }),
        );
        looks.insert(
            unit.to_owned(),
            json!({
                "appearance": name,
                "footprint": vec3(footprint),
                "bounds": [vec3(&whole["bounds"]["min"]), vec3(&whole["bounds"]["max"])],
                "braking": top_speed_kmh(catalog.get(&unit)?) / 3.6 / wreck_stop_s,
            }),
        );
    }
    Ok(json!({
        "presentation": weapon_effects(presentation, weapons)?,
        "types": types,
        "smokes": smokes,
        "wrecks": wrecks,
        "wreckLooks": looks,
    }))
}
