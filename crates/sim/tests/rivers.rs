//! Rivers in the world: one distance says what is water, the terrain is
//! carved to the same cross-section, and a bridge end is reached up a ramp.
//! Judged against the analytic cross-section and the contract's own
//! all-stretches distance, never against the bucket index under test.
use contract::map::MapDefinition;
use sim::math::{v2, v3};
use sim::world::{Collider, SurfaceKind, WorldGeometry};

const RIVER_LAB: &str = include_str!("../../../fixtures/river-lab.json");

fn lab_map() -> MapDefinition {
    serde_json::from_str(RIVER_LAB).unwrap()
}

fn lab() -> WorldGeometry {
    WorldGeometry::new(&lab_map(), &crate::common::rules())
}

fn river(points: &[(f64, f64, f64, f64)], surface_z: f64) -> String {
    let points: Vec<String> = points
        .iter()
        .map(|(x, y, width, depth)| {
            format!(r#"{{"xy":[{x},{y}],"width_m":{width},"depth_m":{depth}}}"#)
        })
        .collect();
    format!(
        r#"{{"points":[{}],"surface_z":{surface_z}}}"#,
        points.join(",")
    )
}

fn kind(w: &WorldGeometry, x: f64, y: f64) -> SurfaceKind {
    w.ground_surface_at(x, y).unwrap().kind
}

#[test]
fn water_is_the_closed_set_within_half_the_width_of_the_line() {
    let w = crate::common::flat(
        [400.0, 300.0],
        &format!(
            r#","rivers":[{},{}]"#,
            river(&[(100.0, 0.0, 12.0, 1.5), (100.0, 300.0, 12.0, 1.5)], -0.5),
            river(&[(300.0, 0.0, 12.0, 1.5), (300.0, 150.0, 30.0, 3.0)], -0.5)
        ),
    );
    // A constant width: the edge itself is water, the next number past it is not.
    for y in [0.0, 37.3, 128.0, 299.0] {
        for (edge, outward) in [(106.0f64, 1.0), (94.0, -1.0)] {
            assert_eq!(kind(&w, edge, y), SurfaceKind::Water, "edge at {edge},{y}");
            let past = edge + outward * edge * f64::EPSILON;
            assert_eq!(kind(&w, past, y), SurfaceKind::Ground, "past {past},{y}");
        }
    }
    // A widening river: half the width runs from 6 m to 15 m along 150 m.
    for y in [10.0, 61.5, 128.0, 149.0] {
        let half = 6.0 + 9.0 * y / 150.0;
        for side in [-1.0, 1.0] {
            assert_eq!(
                kind(&w, 300.0 + side * (half - 1e-9), y),
                SurfaceKind::Water
            );
            assert_eq!(
                kind(&w, 300.0 + side * (half + 1e-9), y),
                SurfaceKind::Ground
            );
        }
    }
    // It ends in a round cap of its last half width.
    let (s, c) = 0.7f64.sin_cos();
    assert_eq!(
        kind(&w, 300.0 + 14.999 * c, 150.0 + 14.999 * s),
        SurfaceKind::Water
    );
    assert_eq!(
        kind(&w, 300.0 + 15.001 * c, 150.0 + 15.001 * s),
        SurfaceKind::Ground
    );
}

#[test]
fn the_bucketed_lookup_is_the_all_stretches_distance() {
    let w = lab();
    let river = &w.rivers()[0];
    let (mut wet, mut dry) = (0, 0);
    let mut y = 0.3;
    while y < 480.0 {
        let mut x = 0.7;
        while x < 640.0 {
            let inside = river.inside([x, y]);
            assert_eq!(
                kind(&w, x, y) == SurfaceKind::Water,
                inside >= 0.0,
                "at ({x}, {y}), {inside} m inside"
            );
            wet += (inside >= 0.0) as usize;
            dry += (inside < 0.0) as usize;
            x += 1.7;
        }
        y += 1.3;
    }
    assert!(wet > 3000 && dry > 50_000, "{wet} wet, {dry} dry");
}

#[test]
fn the_bed_and_bank_are_one_v_through_the_waterline() {
    let w = crate::common::flat(
        [400.0, 300.0],
        &format!(
            r#","rivers":[{}]"#,
            river(&[(200.0, 0.0, 24.0, 2.0), (200.0, 300.0, 24.0, 2.0)], -0.5)
        ),
    );
    let grade = 2.0 / 12.0;
    // The middle, the waterline and the bank's top at 3 m out; flat beyond.
    for (offset, expected) in [
        (0.0, -2.5),
        (4.0, -0.5 - grade * 8.0),
        (10.3, -0.5 - grade * 1.7),
        (12.0, -0.5),
        (20.0, 0.0),
        (60.0, 0.0),
    ] {
        for side in [-1.0, 1.0] {
            let h = w.height_at(200.0 + side * offset, 77.7).unwrap();
            assert!(
                (h - expected).abs() < 1e-9,
                "height {h} at {offset} m, wanted {expected}"
            );
        }
    }
    // The surface the water lies at meets the ground exactly on the edge.
    assert!(w.height_at(212.0, 150.0).unwrap().abs() - 0.5 < 1e-12);
}

#[test]
fn every_dry_point_of_the_river_lab_can_be_stood_on() {
    let w = lab();
    let mut steepest: f64 = 0.0;
    let mut y = 0.5;
    while y < 480.0 {
        let mut x = 0.5;
        while x < 640.0 {
            let s = w.ground_surface_at(x, y).unwrap();
            if s.kind != SurfaceKind::Water {
                assert!(
                    s.traversable && s.slope_deg < w.slope_cutoff_deg(),
                    "a {}° bank at ({x}, {y})",
                    s.slope_deg
                );
                steepest = steepest.max(s.slope_deg);
            } else {
                assert!(!s.traversable && s.road_factor == 0.0 && !s.forest);
            }
            x += 1.1;
        }
        y += 0.9;
    }
    assert!(steepest > 10.0, "the lab has no bank: {steepest}°");
}

/// SG2's measure. The water is drawn where the distance says and hidden
/// where the ground stands above its surface, so the drawn waterline strays
/// from the exact edge by how far inside the edge the grid's triangles still
/// stand above the surface. Outside the edge, ground under the surface
/// shows as bank with no water on it. The first stays under SG2's half
/// metre; the second within the wet band.
#[test]
fn the_grids_waterline_stays_within_half_a_metre_of_the_exact_edge() {
    let w = lab();
    let river = &w.rivers()[0];
    let (mut dry_inside, mut low_outside): (f64, f64) = (0.0, 0.0);
    let mut y = 0.25;
    while y < 480.0 {
        let mut x = 0.25;
        while x < 640.0 {
            let inside = river.inside([x, y]);
            if inside.abs() < 4.0 {
                let above = w.height_at(x, y).unwrap() - river.surface_z();
                if inside >= 0.0 && above > 0.0 {
                    dry_inside = dry_inside.max(inside);
                } else if inside < 0.0 && above < 0.0 {
                    low_outside = low_outside.max(-inside);
                }
            }
            x += 0.5;
        }
        y += 0.5;
    }
    println!("waterline: dry {dry_inside} m inside the edge, low {low_outside} m outside it");
    assert!(
        dry_inside < 0.5,
        "dry ground {dry_inside} m inside the edge"
    );
    // A bank that tops out within a sample of the edge is drawn a little low
    // by the triangles that span its top: under the wet band, never water.
    assert!(
        low_outside < 1.0,
        "sunk ground {low_outside} m outside the edge"
    );
}

/// A river whose bank runs 9 m, bridged by a deck that ends 6 m past the
/// water: without a ramp its ends would hang half a metre over the bank.
fn ramp_world() -> WorldGeometry {
    crate::common::flat(
        [400.0, 300.0],
        &format!(
            r#","rivers":[{}],"bridges":[{{"deck":"bridge_deck","center":[200,150],"half_extents":[18,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}}]"#,
            river(&[(200.0, 0.0, 24.0, 2.0), (200.0, 300.0, 24.0, 2.0)], -1.5)
        ),
    )
}

#[test]
fn a_bridge_end_is_reached_up_a_ramp() {
    let w = ramp_world();
    for side in [-1.0, 1.0] {
        // Along the deck's axis from the open land to its end, and along
        // both of its edges: ground a mover can stand on all the way.
        for across in [-4.9, 0.0, 4.9] {
            let mut out = 60.0;
            while out > 18.0 {
                let s = w.surface_at(200.0 + side * out, 150.0 + across).unwrap();
                assert_eq!(s.kind, SurfaceKind::Ground);
                assert!(s.traversable, "the approach is blocked {out} m out");
                out -= 0.25;
            }
            // The deck is stepped onto from the land's own height, as on
            // any bridge: 0.1 m here, not from half a metre down the bank.
            let before = w.height_at(200.0 + side * 18.001, 150.0 + across).unwrap();
            let deck = w.surface_at(200.0 + side * 17.999, 150.0 + across).unwrap();
            assert_eq!(deck.kind, SurfaceKind::Bridge);
            assert!(
                (deck.z - before - 0.1).abs() < 1e-9,
                "a {} m step onto the deck",
                deck.z - before
            );
        }
    }
    // Away from the bridge the bank keeps its own gentle fall.
    let bank = w.height_at(200.0 + 18.0, 60.0).unwrap();
    assert!((bank - (-1.5 + 6.0 / 6.0)).abs() < 1e-9, "bank {bank}");
}

#[test]
fn a_ramp_never_stands_in_the_water_or_blocks_its_bank() {
    let w = ramp_world();
    let mut y = 120.0;
    while y < 180.0 {
        let mut x = 160.0;
        while x < 240.0 {
            let s = w.ground_surface_at(x, y).unwrap();
            if s.kind == SurfaceKind::Water {
                assert!(s.z <= -1.5 + 1e-9, "ground above the water at ({x}, {y})");
            } else {
                assert!(s.traversable, "a {}° ramp at ({x}, {y})", s.slope_deg);
            }
            x += 0.37;
        }
        y += 0.41;
    }
}

#[test]
fn water_is_neither_forest_nor_road_and_no_trunk_stands_in_it() {
    let w = lab();
    let rules = crate::common::rules();
    let river = &w.rivers()[0];
    // The lab's wood is authored over the river's north bank and its water.
    let in_wood_and_water = w.ground_surface_at(500.0, 237.0).unwrap();
    assert_eq!(in_wood_and_water.kind, SurfaceKind::Water);
    assert!(!in_wood_and_water.forest);
    assert_eq!(w.forest_concealment(true, 500.0, 237.0), 1.0);
    assert_eq!(w.forest_concealment(false, 500.0, 237.0), 1.0);
    assert!(w.ground_surface_at(500.0, 300.0).unwrap().forest);
    let trunks: Vec<_> = w
        .props()
        .filter(|p| p.kind == crate::common::kind("trunk"))
        .collect();
    assert!(trunks.len() > 20);
    let clearance = rules.forests.rule.trunk_clearance_m;
    for trunk in trunks {
        let inside = river.inside([trunk.center.x, trunk.center.y]);
        assert!(inside < -clearance, "a trunk {inside} m inside the water");
    }
    // The country road is carried over the water by the deck; the ground
    // under the deck is still water, and no road.
    let under = w.ground_surface_at(60.0, 240.0).unwrap();
    assert_eq!(under.kind, SurfaceKind::Water);
    assert_eq!(under.road_factor, 0.0);
    assert_eq!(w.surface_at(60.0, 240.0).unwrap().kind, SurfaceKind::Bridge);
    assert_eq!(w.surface_at(60.0, 200.0).unwrap().kind, SurfaceKind::Road);
}

#[test]
fn a_body_shoved_along_a_deck_stays_on_it_and_drops_to_the_bed_off_its_side() {
    let mut w = lab();
    let deck_z = lab_map().bridges[0].deck_z;
    let crate_on_the_road = w.add_prop(&contract::map::PropDefinition {
        kind: "crate".into(),
        center: [60.0, 220.0],
        yaw: 0.0,
        half_extents: [0.8, 0.8, 0.6],
        base_z: None,
    });
    let base = |w: &WorldGeometry| w.prop(crate_on_the_road).unwrap().base_z;
    assert_eq!(base(&w), 0.0);
    // Onto the deck, over the middle of the water: it rides the deck.
    w.move_prop(crate_on_the_road, v2(60.0, 240.0), 0.0, 1);
    assert!((base(&w) - deck_z).abs() < 1e-12, "at {}", base(&w));
    // Off the deck's side: it lies on the bed, under the surface.
    w.move_prop(crate_on_the_road, v2(70.0, 240.0), 0.0, 2);
    assert_eq!(base(&w), w.height_at(70.0, 240.0).unwrap());
    assert!(base(&w) < w.rivers()[0].surface_z() - 1.0);
}

#[test]
fn a_falling_round_stops_at_the_waters_surface() {
    let w = lab();
    let surface = w.rivers()[0].surface_z();
    // Over the 30 m section the bed lies metres under the surface.
    assert!(w.height_at(500.0, 225.5).unwrap() < surface - 3.0);
    let hit = w
        .raycast(v3(500.0, 225.5, 30.0), v3(0.0, 0.0, -1.0), 100.0)
        .unwrap();
    assert_eq!(hit.collider, Collider::Terrain);
    assert!((hit.point.z - surface).abs() < 1e-9);
    assert!((hit.normal - v3(0.0, 0.0, 1.0)).length() < 1e-12);
    // A flat shot across the river above the water flies on to the far bank.
    let across = w.raycast(v3(100.0, 200.0, 1.0), v3(0.0, 1.0, 0.0), 80.0);
    assert!(across.is_none(), "{across:?}");
    assert!(w.segment_clear(v3(100.0, 200.0, 1.0), v3(100.0, 280.0, 1.0)));
    // A plunging shot from the bank meets the water where its line crosses
    // the surface, and the yes/no line test agrees.
    let from = v3(500.0, 195.5, 2.0);
    let to = v3(500.0, 225.5, -3.0);
    let d = to - from;
    let hit = w.raycast(from, d * (1.0 / d.length()), d.length()).unwrap();
    assert!((hit.point.z - surface).abs() < 1e-9);
    assert!((hit.point.y - (195.5 + 30.0 * (2.0 - surface) / 5.0)).abs() < 1e-9);
    assert!(!w.segment_clear(from, to));
    // On the deck a round still lands on the deck.
    let deck = w
        .raycast(v3(60.0, 240.0, 30.0), v3(0.0, 0.0, -1.0), 100.0)
        .unwrap();
    assert!(matches!(deck.collider, Collider::Prop(_)));
}

#[test]
fn a_long_river_keeps_the_terrain_sparse() {
    // 8 km corner to corner: the stored height pages and the regions
    // navigation reads follow the river, not the box round it.
    let size = 8000.0;
    let w = crate::common::flat(
        [size, size],
        &format!(
            r#","rivers":[{}]"#,
            river(&[(0.0, 0.0, 30.0, 3.0), (size, size, 30.0, 3.0)], -1.0)
        ),
    );
    assert_eq!(kind(&w, 4000.0, 4000.0), SurfaceKind::Water);
    let pages = w.export_terrain_page_ids().len();
    let all = (size / (4.0 * 16.0)).powi(2) as usize;
    assert!(
        pages > 100 && pages * 20 < all,
        "{pages} of {all} height pages stored"
    );
    let covered: f64 = w.navigation_regions().iter().map(|r| r[2] * r[3]).sum();
    assert!(
        covered < size * size * 0.05,
        "navigation regions cover {} of the map",
        covered / (size * size)
    );
    // Every point the carving moved lies inside a region.
    for k in 0..2000 {
        let along = size * (k as f64 + 0.5) / 2000.0;
        for offset in [-24.0, -17.0, 0.0, 16.0, 23.0] {
            let p = v2(along + offset, along - offset);
            if w.height_at(p.x, p.y).is_some_and(|h| h != 0.0) {
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
}

#[test]
#[should_panic(expected = "invalid rivers or bridges")]
fn a_world_refuses_a_river_its_terrain_cannot_carry() {
    crate::common::flat(
        [400.0, 300.0],
        &format!(
            r#","rivers":[{}]"#,
            river(&[(200.0, 0.0, 8.0, 1.0), (200.0, 300.0, 8.0, 1.0)], -0.5)
        ),
    );
}
