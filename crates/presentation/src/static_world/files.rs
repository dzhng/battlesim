//! The static world from the repository's own files, as a client with no
//! page reads it: the saved map, the fixture's presentation numbers, and the
//! runtime catalog's template library and kit bundles, each stored gzipped
//! under its transport hash (`packages/scene-assets/src/gzip.ts`).
use crate::static_world::buildings::{Placed, PlacedBuildings};
use crate::static_world::ground::biome::Biome;
use crate::static_world::ground::tables::Grid;
use crate::static_world::ground::{self, Ground, Source, Tuning};
use crate::static_world::library::{kit_bounds, Bounds, Library};
use crate::world::BuildingStyle;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::io::Read;
use std::path::Path;

fn json(path: &Path) -> Result<Value, String> {
    let text = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_slice(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The runtime file the content `hash` travels in, inflated.
fn runtime_file(root: &Path, catalog: &Value, hash: &str, file: &str) -> Result<Vec<u8>, String> {
    let transport = catalog["gzip"][hash]["hash"]
        .as_str()
        .ok_or_else(|| format!("the runtime catalog has no transport for {hash}"))?;
    let path = root.join("assets/runtime").join(transport).join(file);
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = vec![];
    flate2::read::GzDecoder::new(&bytes[..])
        .read_to_end(&mut out)
        .map_err(|e| format!("{}: {e}; is it a Git LFS pointer?", path.display()))?;
    Ok(out)
}

/// A saved map's ground as a client with no page resolves it: the ground
/// (its tables, grass and scenery) and the simulation's terrain triangles.
pub struct MapGround {
    pub ground: Ground,
    /// The terrain's vertices (x, y, z) and triangles, as the simulation
    /// exports them.
    pub positions: Vec<f32>,
    pub indices: Vec<u32>,
}

/// The ground of saved map `map` (`fixtures/maps/<map>`), built by the
/// simulation's own world code under the game's rules, in the map's biome
/// (`meta.json`) and regional finish, with `fixtures/game.json`'s
/// `presentation` tunings: what the page resolves through `WorldView` and
/// `Ground` (`terrainSurface.ts`).
pub fn ground(root: &Path, map: &str) -> Result<MapGround, String> {
    let folder = root.join("fixtures/maps").join(map);
    let resolved = sim::maps::load_folder(&folder).map_err(|e| e.to_string())?;
    let game = json(&root.join("fixtures/game.json"))?;
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::game()).map_err(|e| format!("rules: {e}"))?;
    let map_def = &resolved.definition;
    let world = sim::world::WorldGeometry::new(map_def, &rules);
    let extents = map_def.extents();
    let header: Value = serde_json::from_str(&world.export_terrain_grid())
        .map_err(|e| format!("terrain grid: {e}"))?;
    let n = |k: &str| header[k].as_f64().unwrap_or(0.0);
    let source = Source {
        map: extents.physical,
        // The page hands the rendered surroundings only where they reach
        // past the map (`resolveGround`).
        rendered: (extents.rendered[0] < 0.0).then_some(extents.rendered),
        grid: Grid {
            nx: n("nx") as usize,
            ny: n("ny") as usize,
            spacing: n("spacing"),
            page_size: n("pageSize") as usize,
            min_height: n("minHeight"),
            page_ids: world.export_terrain_page_ids(),
            heights: world.export_terrain_heights(),
        },
        props: world.export_props(),
        drawn_by: ground::drawn_by(&sim::world::export::layout_json(&rules.catalog))?,
        foliage: world.export_foliage(),
        surface_strokes: world.export_surface_strokes(),
        surface_runs: world.export_surface_runs(),
        surface_triangles: world.export_surface_triangles(),
        surface_boundaries: world.export_surface_boundaries(),
        rivers: world.export_rivers(),
        river_runs: world.export_river_runs(),
        forests: world.export_forests(),
        forest_rect_ids: world.export_forest_rect_ids(),
        forest_metadata: world.export_forest_metadata(),
        forest_strokes: world.export_forest_strokes(),
        forest_triangles: world.export_forest_triangles(),
        forest_boundaries: world.export_forest_boundaries(),
        forest_trunk_ranges: world.export_forest_trunk_ranges(),
    };
    let meta = json(&folder.join("meta.json"))?;
    let biome_name = meta["biome"].as_str().ok_or("meta.json has no biome")?;
    let biome_path = root
        .join("fixtures/biomes")
        .join(format!("{biome_name}.json"));
    let biome_text = std::fs::read_to_string(&biome_path)
        .map_err(|e| format!("{}: {e}", biome_path.display()))?;
    let biome = Biome::load(&biome_text, map_def.regional_family.as_deref())?;
    let tuning: Tuning = serde_json::from_value(game["presentation"].clone())
        .map_err(|e| format!("presentation: {e}"))?;
    Ok(MapGround {
        ground: Ground::new(source, biome, tuning)?,
        positions: world.export_terrain_positions(),
        indices: world.export_terrain_indices(),
    })
}

/// The buildings of saved map `map` as `Presentation::load_world` takes
/// them, each building's part props `parts` (in the map's building order)
/// and the prop kinds a gutted building's parts become (`shells`), and the
/// style that places them.
pub fn placed_buildings(
    root: &Path,
    map: &str,
    parts: Vec<Vec<u32>>,
    shells: Vec<String>,
) -> Result<(PlacedBuildings, BuildingStyle), String> {
    let saved = json(&root.join("fixtures/maps").join(map).join("map.json"))?;
    let placed: Vec<Placed> = saved["buildings"]
        .as_array()
        .map(|all| {
            all.iter()
                .map(|b| {
                    let t = &b["frame"]["translation"];
                    let n = |v: &Value| v.as_f64().unwrap_or(0.0);
                    Placed {
                        owner: b["owner"].as_u64().unwrap_or(0) as u32,
                        template: b["template_id"].as_str().unwrap_or("").to_string(),
                        frame: [n(&t[0]), n(&t[1]), n(&t[2]), n(&b["frame"]["yaw"])],
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let style: BuildingStyle = serde_json::from_value(
        json(&root.join("fixtures/game.json"))?["presentation"]["buildings"].clone(),
    )
    .map_err(|e| format!("presentation.buildings: {e}"))?;
    let catalog = json(&root.join("assets/runtime/catalog.json"))?;
    let hash = catalog["templates"]["library"]
        .as_str()
        .ok_or("the runtime catalog has no template library")?;
    let library = Library::decode(&runtime_file(root, &catalog, hash, "templates.bin")?)?;
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
        let bundle = catalog["appearances"][&kit]["bundle"]
            .as_str()
            .ok_or_else(|| {
                format!("kit.missing: kit \"{kit}\" is not in the appearance catalog")
            })?;
        installed.insert(
            kit,
            kit_bounds(&runtime_file(root, &catalog, bundle, "bundle.bin")?)?,
        );
    }
    let bounds: Vec<Option<Bounds>> = library
        .modules
        .iter()
        .map(|m| installed.get(&m.kit).and_then(|k| k.get(&m.name).copied()))
        .collect();
    let parts = parts
        .into_iter()
        .enumerate()
        .flat_map(|(b, props)| props.into_iter().map(move |p| (p, b)))
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
