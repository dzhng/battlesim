//! Puddles are look only: each lies where its cell alone puts it (decision
//! 34), on the surface its biome allows, in that surface's look (decision 28),
//! and the one standing-water owner holds it.
use contract::map::MapDefinition;
use presentation::static_world::ground::biome::{Biome, AREA_KINDS};
use presentation::static_world::ground::puddles::PuddleGround;
use presentation::static_world::ground::water::StandingWater;
use presentation::static_world::ground::{self, Ground, Tuning};
use serde_json::Value;
use sim::world::WorldGeometry;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn read(path: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{path}")).unwrap()
}

/// Saved map `id`'s file.
fn map(id: &str) -> Value {
    serde_json::from_str(&read(&format!("fixtures/maps/{id}/map.json"))).unwrap()
}

/// The river lab's map with a metalled road across its north, so every
/// surface a puddle lies on is there.
fn river_with_road() -> Value {
    let mut m = map("river");
    m["surfaces"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "kind": "road",
            "shape": { "kind": "stroke", "points": [[100, 440], [700, 440]], "width_m": 8 },
        }));
    m
}

/// The ground of map `map` under the summer biome, seeded `seed`.
fn ground_of(map: Value, seed: u32) -> Ground {
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_game()).unwrap();
    let def: MapDefinition = serde_json::from_value(map).unwrap();
    let world = WorldGeometry::new(&def, &rules);
    let source = ground::source(
        &world,
        &def.extents(),
        &sim::world::export::layout_json(&rules.catalog),
    )
    .unwrap();
    let game: Value = serde_json::from_str(&read("fixtures/presentation.json")).unwrap();
    let tuning: Tuning = serde_json::from_value(game.clone()).unwrap();
    let mut biome = Biome::load(&read("fixtures/biomes/summer.json"), None).unwrap();
    biome.seed = seed;
    Ground::new(source, biome, tuning).unwrap()
}

/// The ground of saved map `id`, seeded `seed`.
fn ground(id: &str, seed: u32) -> Ground {
    ground_of(map(id), seed)
}

#[test]
fn a_puddle_takes_the_look_of_the_surface_under_its_centre() {
    let mut looks = std::collections::BTreeSet::new();
    for seed in 0..6 {
        for (id, m) in [
            ("river and road", river_with_road()),
            ("wetlands", map("wetlands")),
        ] {
            let g = ground_of(m, seed);
            let rules = g.biome.water.puddles.as_ref().unwrap();
            for p in &g.puddles {
                // The surface under its centre, as the paving is drawn.
                let [x, y] = p.centre;
                let strokes = g
                    .drawn
                    .as_deref()
                    .unwrap_or(&g.source.surface_strokes)
                    .chunks(7);
                let under = strokes
                    .map(|r| {
                        let (ax, ay, bx, by, half) = (
                            r[0] as f64,
                            r[1] as f64,
                            r[2] as f64,
                            r[3] as f64,
                            r[4] as f64,
                        );
                        let (dx, dy) = (bx - ax, by - ay);
                        let t = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy).max(1e-9))
                            .clamp(0.0, 1.0);
                        (
                            half - ((x - ax - dx * t).hypot(y - ay - dy * t)),
                            r[5] as usize,
                        )
                    })
                    .filter(|&(inside, _)| inside >= 0.0)
                    // Where kinds overlap the earliest wins, as the
                    // simulation resolves them.
                    .map(|(_, tag)| tag)
                    .min()
                    .map(|tag| AREA_KINDS[tag]);
                let want = under.map(|kind| &rules.on[kind].look);
                assert_eq!(
                    Some(&rules.looks[p.look].name),
                    want,
                    "{id} seed {seed}: the puddle at {x:.1}, {y:.1}"
                );
                looks.insert(rules.looks[p.look].name.clone());
            }
        }
    }
    assert!(looks.len() > 1, "both looks are drawn somewhere: {looks:?}");
}

#[test]
fn where_a_puddle_lies_is_its_cell_alone() {
    let g = ground("wetlands", 3);
    let (s, rules) = (&g.source, g.biome.water.puddles.as_ref().unwrap());
    assert!(!g.puddles.is_empty(), "the wetlands map holds puddles");
    // The other water, as the ground asked it, and the strokes as drawn.
    let deep = StandingWater::new(&s.rivers, &s.swamps, &[], &s.grid, &g.biome);
    let in_water = |x, y| deep.at(x, y).is_some();
    let ground = PuddleGround {
        source: s,
        strokes: g.drawn.as_deref().unwrap_or(&s.surface_strokes),
        forests: &g.forests,
        biome: &g.biome,
        in_water: &in_water,
    };
    // Each puddle's cell, asked for alone and in reverse order, gives the
    // puddle the whole map's walk laid; and the walk again gives them all.
    for p in g.puddles.iter().rev() {
        let (i, j) = (
            (p.centre[0] / rules.cell_m).floor() as i64,
            (p.centre[1] / rules.cell_m).floor() as i64,
        );
        assert_eq!(
            ground.candidate(rules, i, j).as_ref(),
            Some(p),
            "cell {i}, {j}"
        );
    }
    assert_eq!(ground.puddles(), g.puddles);
}

#[test]
fn standing_water_holds_a_puddle_shoaling_to_its_rim() {
    let g = ground("wetlands", 3);
    let p = g.puddles[0];
    let water = g.standing_water();
    let middle = water.at(p.centre[0], p.centre[1]).unwrap();
    assert!(
        (middle.depth_m - p.depth_m).abs() < 1e-9,
        "deepest at its middle"
    );
    let near_rim = water
        .at(p.centre[0] + 0.95 * p.radius_m, p.centre[1])
        .unwrap();
    assert!(
        near_rim.depth_m < 0.15 * p.depth_m,
        "shoaling to its rim: {}",
        near_rim.depth_m
    );
    assert_eq!(
        water
            .at(p.centre[0] + 1.05 * p.radius_m, p.centre[1])
            .filter(|w| w.look == middle.look),
        None
    );
}

#[test]
fn no_puddle_lies_in_other_water() {
    for seed in 0..6 {
        for (id, m) in [
            ("river and road", river_with_road()),
            ("wetlands", map("wetlands")),
        ] {
            let g = ground_of(m, seed);
            let s = &g.source;
            let deep = StandingWater::new(&s.rivers, &s.swamps, &[], &s.grid, &g.biome);
            for p in &g.puddles {
                for k in 0..16 {
                    let a = k as f64 * std::f64::consts::PI / 8.0;
                    let (x, y) = (
                        p.centre[0] + p.radius_m * a.cos(),
                        p.centre[1] + p.radius_m * a.sin(),
                    );
                    assert_eq!(
                        deep.at(x, y),
                        None,
                        "{id} seed {seed}: the puddle at {:?} reaches water",
                        p.centre
                    );
                }
            }
        }
    }
}
