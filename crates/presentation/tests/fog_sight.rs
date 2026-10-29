//! Fog of war's sight is the browser's, word for word: over the paris-corner
//! reel's whole publication stream, market-town's capture window and the
//! fog labs' street (blue's encounter, six seconds of ticks), every frame's
//! uploads and passes equal the recorded browser vectors
//! (`fixtures/fog-sight-vectors.json`: FNV-1a over each upload's bytes, the
//! dispatches and the clears, a frame after each publication and a second
//! after every third); `BLESS_PARITY=1` re-records the street stream and
//! layout from that encounter and the native publisher, and every run's
//! frames and stats from its stream. The browser's TypeScript sight that
//! first wrote these vectors is deleted, so they now pin Rust's own sight:
//! re-bless only after a stream or capture was refreshed, and only once the
//! old stream and captures still reproduce the old vectors.
use contract::ids::Side;
use presentation::fog::sight::{FogUpload, Sensors};
use presentation::presentation::{FogSetup, Presentation};
use presentation::records::Camera;
use presentation::static_world::ground::biome::Biome;
use presentation::static_world::ground::{self, Tuning};
use serde_json::Value;
use sim::battle::Battle;
use sim::publication::{self, Publisher};
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
    let game: Value = serde_json::from_str(&read("fixtures/rules.json")).unwrap();
    let tuning: Tuning = serde_json::from_str(&read("fixtures/presentation.json")).unwrap();
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

fn street_fixture() -> (usize, Vec<Vec<u32>>) {
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
    (n, all)
}

fn replace_json_string(text: &mut String, key: &str, value: &str) {
    let key_start = text.find(key).unwrap() + key.len();
    let start = text[key_start..].find('"').map(|i| key_start + i).unwrap();
    let bytes = text.as_bytes();
    let mut end = start + 1;
    let mut escaped = false;
    while end < bytes.len() {
        match (bytes[end], escaped) {
            (b'"', false) => break,
            (b'\\', false) => escaped = true,
            (_, _) => escaped = false,
        }
        end += 1;
    }
    text.replace_range(start..=end, &serde_json::to_string(value).unwrap());
}

/// Re-record the native street stream and its layout.
fn bless_street(vectors: &mut Value) {
    if std::env::var("BLESS_PARITY").as_deref() != Ok("1") {
        return;
    }

    let (ticks, _) = street_fixture();
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_game()).unwrap();
    let scenario = sim::maps::scenario("street", "street", rules).unwrap();
    // Matches STREET_SEED in apps/battle-lab/src/streetScenario.ts.
    let mut battle = Battle::new(&scenario, 20_260_925);
    let layout = publication::layout_json(&battle);
    let mut publisher = Publisher::new();
    let mut records = Vec::with_capacity(ticks);
    for _ in 0..ticks {
        battle.step();
        records.push(
            publisher
                .publish(&battle, Side::Blue)
                .unwrap()
                .iter()
                .map(|word| word.to_bits())
                .collect::<Vec<_>>(),
        );
    }

    let mut words = Vec::with_capacity(1 + ticks + records.iter().map(Vec::len).sum::<usize>());
    words.push(ticks as u32);
    words.extend(records.iter().map(|record| record.len() as u32));
    words.extend(records.into_iter().flatten());
    let bytes: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
    std::fs::write(
        format!(
            "{}/tests/fixtures/fog-street-publications.u32",
            env!("CARGO_MANIFEST_DIR")
        ),
        bytes,
    )
    .unwrap();

    let vectors_path = format!(
        "{}/tests/fixtures/fog-sight-vectors.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut text = std::fs::read_to_string(&vectors_path).unwrap();
    replace_json_string(&mut text, "\"streetLayout\":", &layout);
    std::fs::write(vectors_path, text).unwrap();
    vectors["streetLayout"] = Value::String(layout);
}

fn publications(source: &str) -> (String, Vec<Vec<u32>>) {
    if source == "street" {
        return (String::new(), street_fixture().1);
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
    let mut vectors: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/fog-sight-vectors.json"
        ))
        .unwrap(),
    )
    .unwrap();
    bless_street(&mut vectors);
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
    let bless = std::env::var("BLESS_PARITY").as_deref() == Ok("1");
    let mut blessed = vec![];
    for (index, run) in vectors["runs"].as_array().unwrap().iter().enumerate() {
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
        if bless {
            let stats = serde_json::to_value(p.fog_stats().unwrap()).unwrap();
            blessed.push((index, got, stats));
            continue;
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
    if !blessed.is_empty() {
        for (index, frames, stats) in blessed {
            let run = &mut vectors["runs"][index];
            run["frames"] = frames.into();
            for (key, value) in stats.as_object().unwrap() {
                run["stats"][key] = value.clone();
            }
        }
        std::fs::write(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/fog-sight-vectors.json"
            ),
            serde_json::to_string(&vectors).unwrap(),
        )
        .unwrap();
    }
}
