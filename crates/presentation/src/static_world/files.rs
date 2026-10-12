//! The static world from the repository's own files, as a client with no
//! page reads it: the saved map, the fixture's presentation numbers, and the
//! runtime catalog's template library and kit bundles, each stored gzipped
//! under its transport hash (`packages/scene-assets/src/gzip.ts`).
use crate::static_world::buildings::{Buildings, Placed, Style};
use crate::static_world::library::{kit_bounds, Bounds, Library};
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

/// `presentation.buildings`' tier rule: a chunk `chunk_m` square takes tier
/// t while a metre at its nearest point covers more than `lod_px_per_m[t]`
/// pixels (else the coarsest), and the residency pool holds `pool_records`.
pub struct Tiers {
    pub lod_px_per_m: [f64; 3],
    pub chunk_m: f64,
    pub pool_records: usize,
}

/// The buildings of saved map `map` (`fixtures/maps/<map>/map.json`), drawn
/// from the runtime catalog's template art under `fixtures/game.json`'s
/// `presentation.buildings`, and its tier rule. Only the kits the map's
/// templates place are read.
pub fn buildings(root: &Path, map: &str) -> Result<(Buildings, Tiers), String> {
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
    let presentation = &json(&root.join("fixtures/game.json"))?["presentation"]["buildings"];
    let number = |key: &str| {
        presentation[key]
            .as_f64()
            .ok_or_else(|| format!("presentation.buildings.{key} is not a number"))
    };
    let lod = |t: usize| {
        presentation["lod_px_per_m"][t]
            .as_f64()
            .ok_or("presentation.buildings.lod_px_per_m needs three numbers")
    };
    let style = Style {
        chunk_m: number("chunk_m")?,
        tint_jitter: number("tint_jitter")?,
    };
    let tiers = Tiers {
        lod_px_per_m: [lod(0)?, lod(1)?, lod(2)?],
        chunk_m: style.chunk_m,
        pool_records: number("pool_records")? as usize,
    };
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
    Ok((Buildings::new(library, &bounds, placed, &style)?, tiers))
}
