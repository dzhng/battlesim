//! The static world from the repository's own files, as a client with no
//! page reads it: the saved map, the fixture's presentation numbers, and the
//! runtime catalog's template library and kit bundles ([`Runtime`]).
use crate::assets::Runtime;
use crate::static_world::buildings::{Placed, PlacedBuildings};
use crate::static_world::ground::biome::Biome;
use crate::static_world::ground::{self, Source, Tuning};
use crate::static_world::library::{kit_bounds, state_corners, Bounds, Library};
use crate::static_world::props::{Art, PropInput, STAND_IN_KIT, STAND_IN_MODULE};
use crate::world::BuildingStyle;
use contract::maps::MapCategory;
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

fn json(path: &Path) -> Result<Value, String> {
    let text = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_slice(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// A saved map's ground as a client with no page reads it: what
/// `Presentation::set_ground` resolves the ground from, and the simulation's
/// terrain triangles.
pub struct MapGround {
    pub source: Source,
    pub biome: Biome,
    pub tuning: Tuning,
    /// The sensor numbers sight shares with the simulation (`rules.sensors`).
    pub sensors: crate::fog::sight::Sensors,
    /// The terrain's vertices (x, y, z) and triangles, as the simulation
    /// exports them.
    pub positions: Vec<f32>,
    pub indices: Vec<u32>,
    /// The simulation's world, for what a side's knowledge changes of it
    /// (the foliage left on ground it has seen cleared).
    pub world: World,
    /// The world layout (`sim::world::export::layout_json`) its props are
    /// placed under (`Presentation::place_props`).
    pub layout: String,
    /// The map's regional family, if it has one.
    pub family: Option<String>,
}

/// The ground of saved map `map` (`fixtures/maps/<map>`), built by the
/// simulation's own world code under the battle's resolved rules, in the
/// map's biome (`meta.json`) and
/// regional finish, with `fixtures/presentation.json`'s tunings: what
/// the page reads from `WorldView` and hands `set_ground`.
pub fn ground(
    root: &Path,
    map: &str,
    rules: &contract::scenario::Rules,
) -> Result<MapGround, String> {
    let folder = root.join("fixtures/maps").join(map);
    let resolved = sim::maps::load_folder(&folder).map_err(|e| e.to_string())?;
    let presentation = json(&root.join("fixtures/presentation.json"))?;
    let meta = json(&folder.join("meta.json"))?;
    let map_def = &resolved.definition;
    let world = sim::world::WorldGeometry::new(map_def, rules);
    let layout = sim::world::export::layout_json(&rules.catalog);
    let source = ground::source(&world, &map_def.extents(), &layout)?;
    let biome_name = meta["biome"].as_str().ok_or("meta.json has no biome")?;
    let biome_path = root
        .join("fixtures/biomes")
        .join(format!("{biome_name}.json"));
    let biome_text = std::fs::read_to_string(&biome_path)
        .map_err(|e| format!("{}: {e}", biome_path.display()))?;
    let biome = Biome::load(&biome_text, map_def.regional_family.as_deref())?;
    let tuning = crate::settings::PresentationSettings::from_value(&presentation)?.tuning();
    let sensors =
        serde_json::from_value(serde_json::to_value(&rules.sensors).map_err(|e| e.to_string())?)
            .map_err(|e| format!("sensors: {e}"))?;
    Ok(MapGround {
        source,
        biome,
        tuning,
        sensors,
        positions: world.export_terrain_positions(),
        indices: world.export_terrain_indices(),
        world,
        layout,
        family: map_def.regional_family.clone(),
    })
}

/// The rules of a battle fought on saved map `map`: the game's, with the
/// units of the map's own category (a capture map's the capture set's, a
/// test map's the tests').
pub fn rules(root: &Path, map: &str) -> Result<contract::scenario::Rules, String> {
    let meta = json(&root.join("fixtures/maps").join(map).join("meta.json"))?;
    let category: MapCategory = serde_json::from_value(meta["category"].clone())
        .map_err(|e| format!("{map}/meta.json category: {e}"))?;
    let set = match category {
        MapCategory::Capture => sim::fixtures::CatalogSet::Capture,
        MapCategory::Test => sim::fixtures::CatalogSet::Test,
    };
    let mut fixture = sim::fixtures::rules();
    fixture["catalog"] = Value::Array(sim::fixtures::catalog_documents(set));
    let rules = serde_json::from_value(fixture).map_err(|e| format!("rules: {e}"))?;
    sim::admit(&rules).map_err(|e| e.to_string())?;
    Ok(rules)
}

/// The simulation's world a saved map's ground was built by.
pub use sim::world::WorldGeometry as World;

/// The foliage a side knows of `world`: less the trees on ground its
/// `learned` ground says it has seen cleared (`WorldView.foliage_cleared`).
pub fn side_foliage(world: &World, learned: &crate::scars::LearnedGround) -> Vec<f32> {
    let (cols, rows, cell) = (learned.cols, learned.rows, learned.cell_m);
    world.export_foliage_cleared(|x, y| {
        let (i, j) = ((x / cell).floor(), (y / cell).floor());
        i >= 0.0
            && j >= 0.0
            && (i as usize) < cols
            && (j as usize) < rows
            && learned.is_cleared(i as usize, j as usize)
    })
}

/// What a map in regional family `family` draws its props from
/// (`Presentation::place_props`), as a battle page whose units wear the
/// appearances `wearing` installs it (`propArt` over what the page loads):
/// every static appearance with a footprint the catalog load takes (scenery
/// of no region, and of the wrecks only the worn vehicles' own) or the map's
/// family fetches, each read for its first state's bounds; each worn
/// vehicle's wreck; the stand-in kit's box where the catalog has its module;
/// and the fixture's model tiers. Prop bindings in the layout own stand-in tints.
pub fn prop_art(root: &Path, family: Option<&str>, wearing: &[&str]) -> Result<PropInput, String> {
    let runtime = Runtime::open(&root.join("assets/runtime"))?;
    let presentation = crate::settings::PresentationSettings::from_value(&json(
        &root.join("fixtures/presentation.json"),
    )?)?;
    let none = serde_json::Map::new();
    let entries = runtime.catalog["appearances"].as_object().unwrap_or(&none);
    let worn = |name: &str| wearing.contains(&name);
    let wreck_of = |name: &str| entries.get(name).and_then(|e| e["wreck"].as_str());
    let wrecked: BTreeSet<&str> = entries.keys().filter_map(|n| wreck_of(n)).collect();
    let wrecks: serde_json::Map<String, Value> = wearing
        .iter()
        .filter_map(|&n| wreck_of(n).map(|w| (n.to_owned(), json!(w))))
        .collect();
    let states = |name: &str, entry: &Value| -> Result<Vec<_>, String> {
        let hash = entry["bundle"]
            .as_str()
            .ok_or(format!("{name}: no bundle"))?;
        state_corners(
            runtime.reader(hash, "bundle.bin")?,
            &format!("bundle {name}"),
        )
    };
    let mut appearances = Vec::new();
    for (name, e) in entries {
        let region = e["regional_family"].as_str();
        // A kit, or a look of another region, is fetched only on request.
        let loaded = match (e["unit"].as_str(), region) {
            (Some("kit"), _) => false,
            (_, Some(r)) => Some(r) == family,
            _ if wrecked.contains(name.as_str()) => wrecks.values().any(|w| w == name),
            (unit, None) => unit == Some("scenery") || worn(name),
        };
        let footprint = &e["footprint_half_m"];
        if !loaded || e["kind"] != "static" || footprint.is_null() {
            continue;
        }
        let Some((_, bounds)) = states(name, e)?.into_iter().next() else {
            return Err(format!("bundle {name}: no states"));
        };
        appearances.push(Art {
            name: name.clone(),
            scenery: e["scenery"].as_str().map(str::to_owned),
            family: region.map(str::to_owned),
            footprint: serde_json::from_value(footprint.clone())
                .map_err(|e| format!("{name}: footprint_half_m: {e}"))?,
            paints: serde_json::from_value(e["paints"].clone())
                .map_err(|e| format!("{name}: paints: {e}"))?,
            bounds,
        });
    }
    let stand_in = match entries.get(STAND_IN_KIT) {
        Some(kit) if kit["kind"] == "static" => {
            let states = states(STAND_IN_KIT, kit)?;
            let has_module = states.iter().any(|(n, _)| n == STAND_IN_MODULE);
            states.first().filter(|_| has_module).map(|(_, b)| *b)
        }
        _ => None,
    };
    Ok(PropInput {
        appearances,
        wrecks: serde_json::from_value(Value::Object(wrecks)).map_err(|e| e.to_string())?,
        stand_in,
        family: family.map(str::to_owned),
        detail: presentation.models,
        dynamic: true,
    })
}

/// The buildings of saved map `map`, built by the simulation's `world`, as
/// `Presentation::load_world` takes them: each placed from its template, its
/// part props (`export_buildings`, what the page reads), the prop kinds a
/// gutted building's parts become under the map's catalog, and the style
/// that places them.
pub fn placed_buildings(
    root: &Path,
    rules: &contract::scenario::Rules,
    world: &World,
) -> Result<(PlacedBuildings, BuildingStyle), String> {
    let exported: Value =
        serde_json::from_str(&world.export_buildings()).map_err(|e| format!("buildings: {e}"))?;
    let all: &[Value] = exported["buildings"].as_array().map_or(&[], Vec::as_slice);
    let n = |v: &Value| v.as_f64().unwrap_or(0.0);
    let placed: Vec<Placed> = all
        .iter()
        .map(|b| {
            let t = &b["frame"]["translation"];
            Placed {
                owner: b["owner"].as_u64().unwrap_or(0) as u32,
                template: b["templateId"].as_str().unwrap_or("").to_string(),
                frame: [n(&t[0]), n(&t[1]), n(&t[2]), n(&b["frame"]["yaw"])],
            }
        })
        .collect();
    let parts: Vec<(u32, usize)> = all
        .iter()
        .enumerate()
        .flat_map(|(i, b)| {
            let props = b["parts"].as_array().map_or(&[][..], Vec::as_slice);
            props
                .iter()
                .filter_map(move |p| Some((p["prop"].as_u64()? as u32, i)))
        })
        .collect();
    let (_, shells) = crate::fielded::building_remains(&rules.catalog.view());
    let shells: Vec<String> = shells.into_iter().collect();
    let style = crate::settings::PresentationSettings::from_value(&json(
        &root.join("fixtures/presentation.json"),
    )?)?
    .buildings;
    let runtime = Runtime::open(&root.join("assets/runtime"))?;
    let hash = runtime.catalog["templates"]["library"]
        .as_str()
        .ok_or("the runtime catalog has no template library")?;
    let library = Library::decode(&runtime.read(hash, "templates.bin")?)?;
    // The kits the placed templates' rows place, in any state.
    let mut kits = BTreeSet::new();
    for p in &placed {
        for r in library
            .states(&p.template)?
            .iter()
            .flatten()
            .flat_map(|r| r.clone())
        {
            kits.insert(library.modules[library.module[r] as usize].kit.clone());
        }
    }
    let mut installed: HashMap<String, HashMap<String, Bounds>> = HashMap::new();
    for kit in kits {
        let bundle = runtime.catalog["appearances"][&kit]["bundle"]
            .as_str()
            .ok_or_else(|| {
                format!("kit.missing: kit \"{kit}\" is not in the appearance catalog")
            })?;
        installed.insert(kit, kit_bounds(&runtime.read(bundle, "bundle.bin")?)?);
    }
    let bounds: Vec<Option<Bounds>> = library
        .modules
        .iter()
        .map(|m| installed.get(&m.kit).and_then(|k| k.get(&m.name).copied()))
        .collect();
    let placed = PlacedBuildings {
        library,
        bounds,
        placed,
        parts,
        shells,
    };
    Ok((placed, style))
}
