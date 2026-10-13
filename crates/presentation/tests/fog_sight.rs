//! Fog of war's sight is the browser's, word for word: over the paris-corner
//! reel's whole publication stream, market-town's capture window and the
//! fog labs' street (blue's encounter, six seconds of ticks), every frame's
//! uploads and passes equal the recorded browser vectors
//! (`fixtures/fog-sight-vectors.json`: FNV-1a over each upload's bytes, the
//! dispatches and the clears, a frame after each publication and a second
//! after every third).
use presentation::fog::sight::{FogUpload, Sensors};
use presentation::presentation::{FogSetup, Presentation};
use presentation::records::Camera;
use presentation::static_world::ground::biome::Biome;
use presentation::static_world::ground::{self, Tuning};
use serde_json::Value;
use sim::world::WorldGeometry;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn read(path: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{path}")).unwrap()
}

fn fnv(bytes: &[u8]) -> String {
    let mut h: u32 = 0x811c9dc5;
    for &b in bytes {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x01000193);
    }
    format!("{h:08x}")
}

/// An upload as the browser wrote it: its words and their hash (a buffer
/// is never empty: none writes one zeroed record).
fn upload<T: bytemuck::Pod + Default>(items: &[T]) -> Value {
    let one = [T::default()];
    let items = if items.is_empty() { &one[..] } else { items };
    let bytes: &[u8] = bytemuck::cast_slice(items);
    serde_json::json!([bytes.len() / 4, fnv(bytes)])
}

/// The side's presentation over `map`'s ground under the test rules, with
/// fog shown.
fn presentation(map: &str, layout: &str) -> Presentation {
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_game()).unwrap();
    let resolved = sim::maps::Catalogue::shipped().load(map).unwrap();
    let def = resolved.definition;
    let world = WorldGeometry::new(&def, &rules);
    let source = ground::source(
        &world,
        &def.extents(),
        &sim::world::export::layout_json(&rules.catalog),
    )
    .unwrap();
    let meta: Value =
        serde_json::from_str(&read(&format!("fixtures/maps/{map}/meta.json"))).unwrap();
    let biome = read(&format!(
        "fixtures/biomes/{}.json",
        meta["biome"].as_str().unwrap()
    ));
    let biome = Biome::load(&biome, def.regional_family.as_deref()).unwrap();
    let game: Value = serde_json::from_str(&read("fixtures/game.json")).unwrap();
    let tuning: Tuning = serde_json::from_value(game["presentation"].clone()).unwrap();
    let sensors: Sensors = serde_json::from_value(game["sensors"].clone()).unwrap();
    let mut p = Presentation::new(Some(layout), 30.0).unwrap();
    p.set_ground(source, biome, tuning).unwrap();
    p.set_fog(Some(FogSetup {
        sensors: Some(sensors),
        ..FogSetup::default()
    }))
    .unwrap();
    p
}

/// What one frame uploaded and ran, in the vectors' terms.
fn frame(up: &FogUpload, publication: usize) -> Value {
    let mut f = serde_json::json!({
        "publication": publication,
        "params": fnv(bytemuck::bytes_of(&up.params)),
    });
    let mut dispatch = vec![];
    let mut clears = vec![];
    if let Some(foliage) = &up.foliage {
        f["foliage"] = upload(foliage);
    }
    if let Some(occluders) = &up.occluders {
        f["occluders"] = upload(occluders);
    }
    if let Some(words) = &up.wholes {
        f["wholes"] = upload(words);
    }
    if up.active {
        f["eyes"] = upload(&up.eyes);
        if !up.rebuild.is_empty() {
            f["rebuild"] = upload(&up.rebuild);
            dispatch.push(serde_json::json!(["fogTerrain", up.rebuild_dispatch[0], 1]));
            dispatch.push(serde_json::json!(["fogMerge", up.rebuild_dispatch[1], 1]));
        }
        if let Some(whole) = &up.whole {
            clears.push("fog-whole-flags");
            if whole.count > 0 {
                f["items"] = upload(&whole.items);
                dispatch.push(serde_json::json!([
                    "fogWhole",
                    whole.dispatch[0],
                    whole.dispatch[1]
                ]));
            }
            clears.push("copy-flags");
        }
        clears.push("fog-tile-counts");
        dispatch.push(serde_json::json!(["fogCull", up.tiles[0], up.tiles[1]]));
    }
    f["dispatch"] = dispatch.into();
    f["clears"] = clears.into();
    f
}

fn publications(source: &str) -> (String, Vec<Vec<u32>>) {
    if source == "street" {
        let bytes = std::fs::read(format!(
            "{}/tests/fixtures/fog-street-publications.u32",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let words: Vec<u32> = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect();
        let n = words[0] as usize;
        let mut at = 1 + n;
        let all = words[1..=n]
            .iter()
            .map(|&len| {
                let p = words[at..at + len as usize].to_vec();
                at += len as usize;
                p
            })
            .collect();
        return (String::new(), all);
    }
    let capture: Value = serde_json::from_str(&read(source)).unwrap();
    let layout = match &capture["layout"] {
        Value::String(s) => s.clone(),
        v => v.to_string(),
    };
    let all = capture["samples"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            s["publication"]
                .as_array()
                .unwrap()
                .iter()
                .map(|w| w.as_u64().unwrap() as u32)
                .collect()
        })
        .collect();
    (layout, all)
}

#[test]
fn every_fog_upload_is_the_browsers() {
    let vectors: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/fog-sight-vectors.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let c: Vec<f32> = vectors["camera"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap() as f32)
        .collect();
    let camera = Camera {
        inv_view_proj: c[16..32].try_into().unwrap(),
        eye: c[32..35].try_into().unwrap(),
        ..Camera::default()
    };
    let [width, height] = [1280, 720];
    for run in vectors["runs"].as_array().unwrap() {
        let name = run["name"].as_str().unwrap();
        let source = run["source"].as_str().unwrap();
        let (mut layout, publications) = publications(source);
        let map = if source == "street" {
            layout = vectors["streetLayout"].as_str().unwrap().to_owned();
            "street".to_owned()
        } else {
            let capture: Value = serde_json::from_str(&read(source)).unwrap();
            capture["workload"]["map"].as_str().unwrap().to_owned()
        };
        let mut p = presentation(&map, &layout);
        let want = run["frames"].as_array().unwrap();
        let mut got = vec![];
        for (i, words) in publications.iter().enumerate() {
            assert!(p.push_publication(words, 0.0).unwrap(), "{name} {i}");
            let draws = if i % 3 == 2 { 2 } else { 1 };
            for _ in 0..draws {
                let up = p
                    .prepare_fog(Some(&camera), width, height, 0)
                    .unwrap()
                    .unwrap();
                got.push(frame(&up, i));
            }
        }
        assert_eq!(got.len(), want.len(), "{name}: frames");
        for (k, (g, w)) in got.iter().zip(want).enumerate() {
            let mut w = w.clone();
            let words = w.as_object_mut().unwrap().remove("paramWords");
            if g != &w {
                if let Some(words) = words {
                    let ours: Vec<u32> = bytemuck::cast_slice(bytemuck::bytes_of(
                        &p.prepare_fog(Some(&camera), width, height, 0)
                            .unwrap()
                            .unwrap()
                            .params,
                    ))
                    .to_vec();
                    eprintln!("{name}: browser params {words}\n ours (a later frame) {ours:?}");
                }
                panic!("{name}: frame {k} differs\n ours    {g}\n browser {w}");
            }
        }
        let stats = p.fog_stats().unwrap();
        assert_eq!(
            stats.eyes as u64,
            run["stats"]["eyes"].as_u64().unwrap(),
            "{name}"
        );
        assert_eq!(
            stats.rebuilt_total,
            run["stats"]["rebuiltTotal"].as_u64().unwrap(),
            "{name}"
        );
    }
}
