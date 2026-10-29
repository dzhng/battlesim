//! Swamps in the world: standing shallow water in a dip each carves itself.
//! The ground lowers inside the outline, easing in over the edge, and the
//! water stands level with the land; a road across it is a causeway. The
//! water keeps one rule with a river's: it takes no marks, and rounds and
//! sight stop at its surface. Judged against the contract's own distance
//! into the outline, never against the bucket index under test.
use contract::ids::UnitId;
use contract::map::MapDefinition;
use serde_json::json;
use sim::battle::Battle;
use sim::math::{v2, v3};
use sim::world::WorldGeometry;

use crate::common;

/// How deep the test swamps lie: a test number, not the tuned one.
const DEPTH: f64 = 0.6;
/// The test swamp's outline: a 100 m square.
const SQUARE: [[f64; 2]; 4] = [
    [100.0, 100.0],
    [200.0, 100.0],
    [200.0, 200.0],
    [100.0, 200.0],
];

fn swamp(ring: &[[f64; 2]], depth: f64) -> String {
    format!(
        r#"{{"shape":{{"kind":"polygon","ring":{}}},"depth_m":{depth}}}"#,
        serde_json::to_string(ring).unwrap()
    )
}

/// A flat 400 × 300 m map holding the test swamp, plus extra map JSON
/// fields (leading comma).
fn map_json(extra: &str) -> String {
    format!(
        r#"{{"size":[400,300],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"swamps":[{}]{extra}}}"#,
        swamp(&SQUARE, DEPTH)
    )
}

fn map(extra: &str) -> MapDefinition {
    common::physical_map(&map_json(extra))
}

fn world(extra: &str) -> WorldGeometry {
    WorldGeometry::new(&map(extra), &common::rules())
}

/// How far the dip eases in from its outline on the test maps' 4 m grid.
fn edge() -> f64 {
    contract::swamp::edge_m(4.0)
}

#[test]
fn a_swamp_carves_its_dip_on_a_map_with_no_river() {
    let w = world("");
    assert!(w.rivers().is_empty());
    let h = |x: f64, y: f64| w.height_at(x, y).unwrap();
    // Past the edge the bed lies the whole depth under the land.
    for (x, y) in [(150.0, 150.0), (100.0 + edge(), 150.0), (137.3, 171.9)] {
        assert!(
            (h(x, y) + DEPTH).abs() < 1e-9,
            "bed {} at ({x}, {y})",
            h(x, y)
        );
    }
    // From the outline the ground falls steadily to the bed across the
    // edge, and outside the outline it is the land's.
    let mut last = h(100.0, 150.0);
    assert_eq!(last, 0.0);
    for k in 1..=12 {
        let at = h(100.0 + edge() * k as f64 / 12.0, 150.0);
        assert!(at <= last, "the edge rises {k}/12 of the way in");
        last = at;
    }
    assert!(h(104.0, 150.0) < 0.0 && h(104.0, 150.0) > -DEPTH);
    for (x, y) in [(96.0, 150.0), (150.0, 204.0), (50.0, 50.0), (300.0, 250.0)] {
        assert_eq!(h(x, y), 0.0, "uncut land at ({x}, {y})");
    }
    // Every point in and round the swamp can be stood on: the soft edge
    // is no wall.
    let mut y = 90.5;
    while y < 210.0 {
        let mut x = 90.5;
        while x < 210.0 {
            let s = w.surface_at(x, y).unwrap();
            assert!(s.traversable, "a {}° edge at ({x}, {y})", s.slope_deg);
            x += 1.3;
        }
        y += 1.1;
    }
    // Every point the carving moved lies in a region terrain and
    // navigation read.
    for k in 0..400 {
        let p = v2(90.0 + 0.3 * k as f64, 90.0 + 0.29 * k as f64);
        if h(p.x, p.y) != 0.0 {
            assert!(
                w.navigation_regions().iter().any(|r| p.x >= r[0]
                    && p.y >= r[1]
                    && p.x <= r[0] + r[2]
                    && p.y <= r[1] + r[3]),
                "carved ground at {p:?} is in no region"
            );
        }
    }
}

/// A swamp's water is its outline less any road across it, cell by cell
/// round its edge and either side of the road. (That planning reads the
/// same water as movement is `ground_speed`'s.)
#[test]
fn a_swamps_water_is_its_outline_off_the_road() {
    let w = world(ROAD);
    let outline = &map(ROAD).swamps[0];
    let (mut wet, mut dry) = (0, 0);
    for j in 40..110 {
        for i in 40..110 {
            let c = v2(i as f64 * 2.0 + 1.0, j as f64 * 2.0 + 1.0);
            let surface = w.surface_at(c.x, c.y).unwrap();
            let inside = outline.inside([c.x, c.y]) >= 0.0;
            assert_eq!(
                surface.swamp,
                inside && !surface.paved.is_some_and(|kind| kind.is_road()),
                "at {c:?}"
            );
            wet += surface.swamp as usize;
            dry += !surface.swamp as usize;
        }
    }
    assert!(wet > 2000 && dry > 2000, "{wet} wet cells, {dry} dry");
}

/// A road 8 m wide straight across the test swamp, west to east.
const ROAD: &str = r#","surfaces":[{"kind":"road","shape":{"kind":"stroke","points":[[0,150],[400,150]],"width_m":8}}]"#;

/// A road across a swamp is a causeway: the land's height all along it and
/// its whole width, dry and at road speed, with the swamp's water either
/// side down a slope a mover can climb.
#[test]
fn a_road_across_a_swamp_is_never_carved() {
    let w = world(ROAD);
    let mut x = 80.25;
    while x < 220.0 {
        for y in [146.0, 148.3, 150.0, 151.9, 154.0] {
            let s = w.surface_at(x, y).unwrap();
            assert_eq!(s.z, 0.0, "the road is cut at ({x}, {y})");
            assert!(s.paved.is_some_and(|kind| kind.is_road()) && !s.swamp && !s.standing_water());
        }
        // Off the road, inside the outline: the swamp's water.
        if (100.0..=200.0).contains(&x) {
            for y in [145.9, 154.1] {
                assert!(w.surface_at(x, y).unwrap().swamp, "dry at ({x}, {y})");
            }
            let bed = w.height_at(x, 150.0 + 4.0 + 4.0 * 1.5 + edge()).unwrap();
            if (100.0 + edge()..=200.0 - edge()).contains(&x) {
                assert!((bed + DEPTH).abs() < 1e-9, "bed {bed} by the road at {x}");
            }
        }
        x += 0.7;
    }
    let mut y = 140.5;
    while y < 160.0 {
        let s = w.surface_at(150.0, y).unwrap();
        assert!(s.traversable, "a {}° causeway side at y {y}", s.slope_deg);
        y += 0.5;
    }
}

/// Water keeps one rule for marks: a shell bursting in the swamp leaves no
/// crater or scorch and a tank wading it no tracks, while the dry rim round
/// it takes both.
#[test]
fn a_swamp_takes_no_crater_or_track_wear_but_its_dry_rim_does() {
    let burst = |at: [f64; 2]| json!({ "tick": 1, "burst": { "point": at, "weapon": "tank_he" } });
    let mut b = Battle::new(
        &common::scenario_with(
            &map_json(""),
            json!([{ "side": "blue", "kind": "test_tank", "position": [20, 150] }]),
            json!([
                burst([150.0, 120.0]),
                burst([170.0, 180.0]),
                burst([60.0, 250.0])
            ]),
            json!([{ "tick": 1, "side": "blue",
                "order": { "kind": "move", "units": [0], "gesture": 1, "goal": [380, 150], "route": "shortest" } }]),
        ),
        1,
    );
    for _ in 0..120 * 30 {
        b.step();
        if b.unit(UnitId(0)).unwrap().position.x >= 375.0 {
            break;
        }
    }
    assert!(
        b.unit(UnitId(0)).unwrap().position.x >= 375.0,
        "the tank waded the swamp"
    );
    let ground = b.ground();
    for at in [[150.0, 120.0], [170.0, 180.0]] {
        let cell = ground.cell(at[0], at[1]);
        assert!(
            cell.crater == 0 && cell.scorch == 0,
            "{cell:?} in the water at {at:?}"
        );
    }
    let rim = ground.cell(60.0, 250.0);
    assert!(rim.crater > 0 && rim.scorch > 0, "{rim:?} on dry ground");
    let tracked = |inside: &dyn Fn(f64) -> bool| {
        ground
            .cells()
            .any(|(x, y, c)| inside(x) && (y - 150.0).abs() <= 4.0 && c.tracks > 0)
    };
    assert!(tracked(&|x| x < 96.0), "tracks on the near rim");
    assert!(tracked(&|x| x > 204.0), "tracks on the far rim");
    assert!(
        !tracked(&|x| (101.0..198.0).contains(&x)),
        "tracks in the swamp"
    );
}

/// As on a river: a round falling into the swamp meets its water, not its
/// bed, and a shot skimming above the water flies on.
#[test]
fn a_round_falling_into_a_swamp_stops_at_its_surface() {
    use sim::world::Collider;
    let w = world(ROAD);
    assert!((w.height_at(150.0, 120.0).unwrap() + DEPTH).abs() < 1e-9);
    let hit = w
        .raycast(v3(150.0, 120.0, 30.0), v3(0.0, 0.0, -1.0), 100.0)
        .unwrap();
    assert_eq!(hit.collider, Collider::Terrain);
    assert!(hit.point.z.abs() < 1e-9, "at {}", hit.point.z);
    assert!((hit.normal - v3(0.0, 0.0, 1.0)).length() < 1e-12);
    // A plunging shot from the rim meets the water where its line crosses
    // the surface, and the yes/no line test agrees.
    let (from, to) = (v3(80.0, 130.0, 2.0), v3(150.0, 120.0, -0.5));
    let d = to - from;
    let hit = w.raycast(from, d * (1.0 / d.length()), d.length()).unwrap();
    assert!(hit.point.z.abs() < 1e-9);
    assert!((hit.point.x - (80.0 + 70.0 * 2.0 / 2.5)).abs() < 1e-9);
    assert!(!w.segment_clear(from, to));
    // Just above the water it flies across to the far rim.
    assert!(w.segment_clear(v3(60.0, 120.0, 0.3), v3(240.0, 120.0, 0.3)));
}

/// The water stops sight and rounds at its surface, but a soldier standing
/// in it stands above that: his eye, his aim point and the point he is
/// seen by all clear the water, so he is seen and hit as on dry ground.
#[test]
fn a_soldier_standing_in_a_swamp_is_seen_and_can_be_hit() {
    let mut b = Battle::new(
        &common::scenario(
            &map_json(""),
            json!([
                { "side": "blue", "kind": "test_rifle", "position": [150, 140], "engagement": "return_fire_only" },
                { "side": "red", "kind": "test_rifle", "position": [150, 40] }
            ]),
            json!([]),
        ),
        1,
    );
    let blue = b.unit(UnitId(0)).unwrap();
    assert!(blue.members.iter().all(|m| {
        b.world()
            .surface_at(m.position.x, m.position.y)
            .is_some_and(|s| s.swamp && (s.z + DEPTH).abs() < 1e-9)
    }));
    assert!(
        !b.observe(contract::ids::Side::Red).identified.is_empty(),
        "red sees the squad in the swamp"
    );
    let full = blue.members[0].hp;
    let mut hurt = false;
    for _ in 0..30 * 30 {
        b.step();
        hurt = b
            .unit(UnitId(0))
            .unwrap()
            .members
            .iter()
            .any(|m| m.hp < full);
        if hurt {
            break;
        }
    }
    assert!(hurt, "red's rifles hit the squad in the swamp");
}

/// No sight rule names a swamp: its dip shapes sight through the ordinary
/// terrain rule (decision 21), shown on a test dip far deeper than any
/// tuned swamp. The ground sight sweep marks a cell seen when the point a
/// target height above its ground clears the terrain between: from far
/// across flat ground the rim hides the dip's floor, while an eye at the
/// rim, or one high above the far ground, sees down into it.
#[test]
fn a_deep_dip_hides_its_floor_from_far_flat_ground_but_not_from_near_or_high() {
    use sim::visibility::{self, OcclusionGrid};
    let mut rules = common::rules();
    rules.sensors.fog_target_height_m = 1.0;
    let deep = 3.0;
    let ring = [[300.0, 100.0], [400.0, 200.0]];
    let ring = [
        ring[0],
        [ring[1][0], ring[0][1]],
        ring[1],
        [ring[0][0], ring[1][1]],
    ];
    let map = common::physical_map(&format!(
        r#"{{"size":[600,300],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"swamps":[{}]}}"#,
        swamp(&ring, deep)
    ));
    let w = WorldGeometry::new(&map, &rules);
    let floor = (350.0, 150.0);
    assert!((w.height_at(floor.0, floor.1).unwrap() + deep).abs() < 1e-9);
    let sight = sim::sight::Sight {
        forward: 0.0,
        shape: contract::scenario::SightShape {
            front: 1.0,
            side: 1.0,
            rear: 1.0,
        },
        range: 400.0,
    };
    let sees_floor = |eye: sim::math::V3| {
        let mut grid = OcclusionGrid::new(&w, 8.0);
        let mut field = grid.field();
        visibility::sweep(&w, &mut grid, &rules.sensors, eye, &sight, &mut field);
        field.visible(floor.0, floor.1)
    };
    let eye = rules.physics.infantry_eye_m;
    assert!(!sees_floor(v3(40.0, 150.0, eye)), "far, on flat ground");
    assert!(sees_floor(v3(296.0, 150.0, eye)), "near, at the rim");
    assert!(sees_floor(v3(40.0, 150.0, 30.0)), "far and high");
}

/// What the world's admission refuses, on a flat 600 × 300 m map holding
/// `swamps` (JSON objects) and extra fields (leading comma).
fn refusals(swamps: &[String], extra: &str) -> String {
    let map = common::physical_map(&format!(
        r#"{{"size":[600,300],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"swamps":[{}]{extra}}}"#,
        swamps.join(",")
    ));
    contract::maps::admit_world(&map, &common::rules())
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> [[f64; 2]; 4] {
    [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

#[test]
fn admission_takes_a_swamp_as_a_polygon_with_a_depth() {
    assert_eq!(refusals(&[swamp(&SQUARE, DEPTH)], ""), "");
    let stroke =
        r#"{"shape":{"kind":"stroke","points":[[100,150],[300,150]],"width_m":40},"depth_m":0.6}"#;
    assert!(refusals(&[stroke.into()], "").contains("polygon"));
    for depth in ["0", "-0.5"] {
        let refused = refusals(&[swamp(&SQUARE, 0.6).replace("0.6", depth)], "");
        assert!(refused.contains("positive depth"), "{depth}: {refused}");
    }
}

/// A swamp holding a disc three height cells across is carved wherever the
/// grid falls on it; one far narrower is refused.
#[test]
fn admission_refuses_a_swamp_too_small_to_carve() {
    for offset in [0.0, 1.3, 2.0, 2.9] {
        let (x, y) = (101.0 + offset, 103.0 - offset);
        assert_eq!(
            refusals(&[swamp(&rect(x, y, x + 12.0, y + 12.0), DEPTH)], ""),
            ""
        );
        let refused = refusals(&[swamp(&rect(x, y, x + 5.0, y + 5.0), DEPTH)], "");
        assert!(refused.contains("too small"), "{refused}");
    }
}

/// Whatever depth admission takes, a mover can climb every point of the
/// dip's edge; a dip too deep for its edge is refused, not drawn as a wall.
#[test]
fn admission_refuses_an_edge_steeper_than_the_slope_cutoff() {
    let (mut admitted, mut refused) = (0, 0);
    for depth in [0.6, 1.5, 3.0, 4.0, 6.0, 12.0] {
        let why = refusals(&[swamp(&SQUARE, depth)], "");
        if !why.is_empty() {
            assert!(why.contains("steeper"), "{depth} m: {why}");
            refused += 1;
            continue;
        }
        admitted += 1;
        let w = WorldGeometry::new(
            &common::physical_map(&format!(
                r#"{{"size":[400,300],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"swamps":[{}]}}"#,
                swamp(&SQUARE, depth)
            )),
            &common::rules(),
        );
        let mut x = 95.25;
        while x < 205.0 {
            for y in [100.5, 117.3, 150.0] {
                let s = w.surface_at(x, y).unwrap();
                assert!(s.traversable, "{depth} m: a {}° edge at {x}", s.slope_deg);
            }
            x += 0.5;
        }
    }
    assert!(
        admitted >= 3 && refused >= 2,
        "{admitted} admitted, {refused} refused"
    );
}

/// Its water lies flat at the land's height, so the land under it is level.
#[test]
fn admission_refuses_a_swamp_on_uneven_land() {
    let ridge = |x: f64| {
        format!(r#","relief":[{{"kind":"ridge","center":[{x},150],"peak_m":3,"radius_m":60}}]"#)
    };
    assert_eq!(refusals(&[swamp(&SQUARE, DEPTH)], &ridge(400.0)), "");
    let refused = refusals(&[swamp(&SQUARE, DEPTH)], &ridge(250.0));
    assert!(refused.contains("uneven"), "{refused}");
}

/// An edge's width of dry, uncut land lies between a swamp and any river.
#[test]
fn admission_keeps_a_swamp_clear_of_rivers() {
    let river = |x: f64| {
        format!(
            r#","rivers":[{{"surface_z":-0.5,"points":[{{"xy":[{x},0],"width_m":12,"depth_m":1.5}},{{"xy":[{x},300],"width_m":12,"depth_m":1.5}}]}}]"#
        )
    };
    // Its water's edge 24 m from the swamp's, its bank cut 2 m into the land.
    assert_eq!(refusals(&[swamp(&SQUARE, DEPTH)], &river(230.0)), "");
    for x in [216.0, 190.0] {
        let refused = refusals(&[swamp(&SQUARE, DEPTH)], &river(x));
        assert!(refused.contains("river"), "{x}: {refused}");
    }
}

/// No building stands on ground a swamp cuts.
#[test]
fn admission_keeps_buildings_off_a_swamp() {
    const HOUSE: &str = "china-home-12x9-2f";
    let house = |y: f64| {
        format!(
            r#","buildings":[{}]"#,
            common::building(0, HOUSE, [150.0, y], 0.0)
        )
    };
    assert_eq!(refusals(&[swamp(&SQUARE, DEPTH)], &house(240.0)), "");
    for y in [196.0, 150.0] {
        let refused = refusals(&[swamp(&SQUARE, DEPTH)], &house(y));
        assert!(refused.contains("building"), "{y}: {refused}");
    }
}

/// A bridge is come onto over uncut land: a swamp keeps off its deck and
/// an edge's width past each end.
#[test]
fn admission_keeps_bridge_approaches_off_a_swamp() {
    let bridged = r#","rivers":[{"surface_z":-1.5,"points":[{"xy":[300,0],"width_m":24,"depth_m":2},{"xy":[300,300],"width_m":24,"depth_m":2}]}],"bridges":[{"deck":"bridge_deck","center":[300,150],"half_extents":[30,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}]"#;
    let beside = |east: f64| swamp(&rect(180.0, 120.0, east, 180.0), DEPTH);
    assert_eq!(refusals(&[beside(240.0)], bridged), "");
    let refused = refusals(&[beside(262.0)], bridged);
    assert!(refused.contains("bridges[0]"), "{refused}");
}

/// Presentation draws each swamp from the export: its outline's edges, each
/// with the swamp's index (what its look is seeded by), its water's height
/// and its depth, laid out as the published layout says.
#[test]
fn the_export_gives_each_swamp_its_outline_and_index() {
    use sim::world::export;
    let triangle = [[260.0, 40.0], [330.0, 40.0], [300.0, 110.0]];
    let w = WorldGeometry::new(
        &common::physical_map(&format!(
            r#"{{"size":[400,300],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"swamps":[{},{}]}}"#,
            swamp(&SQUARE, DEPTH),
            swamp(&triangle, 0.4)
        )),
        &common::rules(),
    );
    let layout: serde_json::Value =
        serde_json::from_str(&export::layout_json(&common::rules().catalog)).unwrap();
    let fields: Vec<&str> = layout["swampFields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert_eq!(
        layout["swampStride"].as_u64().unwrap() as usize,
        fields.len()
    );
    let field = |name: &str| fields.iter().position(|f| *f == name).unwrap();
    let exported = w.export_swamps();
    let mut edges = [Vec::new(), Vec::new()];
    for r in exported.chunks(fields.len()) {
        let swamp = r[field("swamp")] as usize;
        edges[swamp].push([
            r[field("ax")],
            r[field("ay")],
            r[field("bx")],
            r[field("by")],
        ]);
        assert_eq!(r[field("surfaceZ")], 0.0);
        assert_eq!(r[field("depth")], [DEPTH, 0.4][swamp] as f32);
    }
    for (swamp, ring) in [&SQUARE[..], &triangle[..]].into_iter().enumerate() {
        let expected: Vec<[f32; 4]> = (0..ring.len())
            .map(|i| {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                [a[0], a[1], b[0], b[1]].map(|v| v as f32)
            })
            .collect();
        assert_eq!(edges[swamp], expected, "swamp {swamp}");
    }
}

/// Map analysis counts a swamp's water as no open ground, as it does a
/// river's: no approach or sight sample stands in it. A causeway across it
/// and the dry rim round it still count.
#[test]
fn map_analysis_counts_no_swamp_water_as_open_ground() {
    let w = world(ROAD);
    let sites = contract::encounter::EncounterSites::default();
    let open = |x: f64, y: f64| sim::map_analysis::open_ground(&w, &sites, v2(x, y));
    for (x, y) in [(150.0, 120.0), (101.0, 180.0), (199.0, 101.0)] {
        assert!(!open(x, y), "swamp water at ({x}, {y})");
    }
    for (x, y) in [(150.0, 150.0), (96.0, 120.0), (300.0, 250.0)] {
        assert!(open(x, y), "dry ground at ({x}, {y})");
    }
}
