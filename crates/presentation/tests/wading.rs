//! A soldier standing in water takes no posture that would put him under
//! it: in a swamp deeper than a lying man he never lies down, and in a river
//! deeper than a kneeling man's chest he stands. On dry ground the same men
//! lie and kneel as they always would. The water is a real ground's (the
//! river lab with a swamp beside its river), the feed synthetic.
use contract::map::MapDefinition;
use presentation::static_world::ground::biome::Biome;
use presentation::static_world::ground::{self, Ground, Tuning};
use presentation::units::driver::PoseDriver;
use presentation::world::UnitsSection;
use serde_json::{json, Value};
use sim::world::WorldGeometry;

mod common;
use common::*;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn read(path: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{path}")).unwrap()
}

/// The river lab's ground with a swamp 0.6 m deep beside its river.
fn wet_ground() -> Ground {
    let mut map: Value = serde_json::from_str(&read("fixtures/maps/river/map.json")).unwrap();
    map["swamps"] = json!([{
        "shape": { "kind": "polygon", "ring": [[440, 380], [560, 380], [560, 450], [440, 450]] },
        "depth_m": 0.6
    }]);
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_game()).unwrap();
    let def: MapDefinition = serde_json::from_value(map).unwrap();
    let world = WorldGeometry::new(&def, &rules);
    let layout = sim::world::export::layout_json(&rules.catalog);
    let source = ground::source(&world, &def.extents(), &layout).unwrap();
    let biome = Biome::load(&read("fixtures/biomes/summer.json"), None).unwrap();
    let game: Value = serde_json::from_str(&read("fixtures/game.json")).unwrap();
    let tuning: Tuning = serde_json::from_value(game["presentation"].clone()).unwrap();
    Ground::new(source, biome, tuning).unwrap()
}

/// How high each posture (standing, kneeling, lying) holds a man's chest.
fn chest(units: &UnitsSection) -> [f64; 3] {
    serde_json::from_value(units.pose["stance"]["chest_m"].clone()).unwrap()
}

/// Every clip twelve men at (x, y) play, first pinned, then firing, in
/// `ground`.
fn clips(units: &UnitsSection, ground: Option<&Ground>, x: f64, y: f64) -> Vec<String> {
    let none = fallen(&[]);
    let mut seen = vec![];
    for pinned in [true, false] {
        let mut driver = PoseDriver::new(units).unwrap();
        for (time, shots) in [(0.0, 0.0), (0.5, 1.0)] {
            let men = (0..12).map(|i| man(f64::from(i) + 1.0, x + f64::from(i % 4), y, shots));
            let poses = driver.update(
                &frame(time, vec![squad(men.collect(), pinned)], &none),
                units,
                ground,
            );
            seen.extend(poses.soldiers.iter().map(|s| s.clip.to_string()));
        }
    }
    seen
}

fn lying(clip: &str) -> bool {
    clip == "prone_pinned" || clip.starts_with("crawl_")
}

fn kneeling(clip: &str) -> bool {
    clip == "kneel_fire" || clip.starts_with("crouch_")
}

#[test]
fn a_soldier_standing_in_water_deeper_than_a_lying_soldier_is_never_posed_prone() {
    let units = units();
    let ground = wet_ground();
    let water = ground.standing_water();
    let [_, kneel, prone] = chest(&units);
    let (x, y) = (496.0, 415.0);
    let depth = water.at(x, y).expect("the swamp holds water there").depth_m;
    assert!(
        prone < depth && depth < kneel,
        "a swamp a lying man drowns in: {depth} m"
    );

    let dry = clips(&units, None, x, y);
    assert!(
        dry.iter().any(|c| lying(c)),
        "on dry ground some lie down: {dry:?}"
    );
    let wet = clips(&units, Some(&ground), x, y);
    assert!(
        !wet.iter().any(|c| lying(c)),
        "in the swamp nobody lies: {wet:?}"
    );
    assert!(
        wet.iter().any(|c| kneeling(c)),
        "they kneel instead: {wet:?}"
    );
}

#[test]
fn in_a_river_deeper_than_a_kneeling_soldier_s_chest_he_stands() {
    let units = units();
    let ground = wet_ground();
    let water = ground.standing_water();
    let [_, kneel, _] = chest(&units);
    let (x, y) = (20.0, 240.0);
    let depth = water.at(x, y).expect("the river runs there").depth_m;
    assert!(kneel < depth, "a river a kneeling man drowns in: {depth} m");

    let dry = clips(&units, None, x, y);
    assert!(
        dry.iter().any(|c| kneeling(c)),
        "on dry ground some kneel: {dry:?}"
    );
    let wet = clips(&units, Some(&ground), x, y);
    assert!(
        !wet.iter().any(|c| lying(c) || kneeling(c)),
        "in the river everyone stands: {wet:?}"
    );
}
