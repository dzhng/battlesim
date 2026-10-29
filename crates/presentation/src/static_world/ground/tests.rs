//! The ground's behaviour on one small synthetic world, through `Ground`'s
//! own calls: a square wood with a road through it, a tree line, a lone
//! street tree, a house and a rock, a river and a court, under the summer
//! biome.
use super::biome::tests::SUMMER;
use super::biome::{linear, Biome};
use super::scenery::{Scenery, Size, DRESSING_CELL_M, FLOATS};
use super::shapes::{closest, distance, stroke_inside, triangle_contains};
use super::tables::Grid;
use super::{DrawnBy, Ground, Source, Tuning, PROP};

const WOOD: [f32; 4] = [20.0, 20.0, 60.0, 60.0];
const CANOPY: f32 = 12.0;
/// The tree line: along x = 120 from y = 30 to 170, 12 m wide.
const LINE: [f32; 7] = [120.0, 30.0, 120.0, 170.0, 6.0, 1.0, 3.0];
/// A country road across the wood along y = 50, 6 m wide.
const ROAD: [f32; 7] = [0.0, 50.0, 200.0, 50.0, 3.0, 1.0, 3.0];
const LONE: [f32; 2] = [160.0, 160.0];
const ROCK: [f32; 2] = [60.0, 70.0];
/// A river along y = 185, 4 m wide, and a court of paving at the far corner.
const RIVER: [f32; 12] = [
    0.0, 185.0, 200.0, 185.0, 2.0, 2.0, 0.5, 0.5, 1.0, 1.0, -1.0, 0.0,
];
const COURT: [f32; 4] = [170.0, 100.0, 190.0, 120.0];

fn prop(id: u32, kind: f32, [x, y]: [f32; 2], half: f32) -> [f32; PROP] {
    [id as f32, 0.0, kind, x, y, 0.3, half, half, 4.0, 0.0, -1.0]
}

/// The synthetic world: trunks are props of kind 0 (drawn by their forest),
/// the house kind 1 (a building), the rock kind 2.
fn world() -> Source {
    let mut props = vec![];
    let mut id = 0;
    for x in (24..80).step_by(8) {
        for y in (24..80).step_by(8) {
            props.extend(prop(id, 0.0, [x as f32, y as f32], 0.3));
            id += 1;
        }
    }
    let wood = id;
    for y in (34..170).step_by(9) {
        props.extend(prop(id, 0.0, [120.0, y as f32], 0.3));
        id += 1;
    }
    let line = id;
    props.extend(prop(id, 0.0, LONE, 0.3));
    props.extend(prop(id + 1, 1.0, [150.0, 60.0], 5.0));
    props.extend(prop(id + 2, 2.0, ROCK, 1.5));
    let [x0, y0, x1, y1] = COURT;
    let court = [[x0, y0, x1, y0, x1, y1], [x0, y0, x1, y1, x0, y1]];
    let edges = [
        [x0, y0, x1, y0],
        [x1, y0, x1, y1],
        [x1, y1, x0, y1],
        [x0, y1, x0, y0],
    ];
    // Foliage in every 8 m fog cell the wood and the line cover.
    let mut foliage = vec![25.0, 25.0, 8.0];
    for j in 0..25 {
        for i in 0..25 {
            let (x, y) = (i as f32 * 8.0 + 4.0, j as f32 * 8.0 + 4.0);
            let in_wood =
                x > WOOD[0] && x < WOOD[0] + WOOD[2] && y > WOOD[1] && y < WOOD[1] + WOOD[3];
            let in_line = (x - 120.0).abs() < 8.0 && (28.0..172.0).contains(&y);
            if in_wood || in_line {
                foliage.extend([i as f32, j as f32, CANOPY, 1.0]);
            }
        }
    }
    Source {
        map: [0.0, 0.0, 200.0, 200.0],
        grid: Grid {
            nx: 51,
            ny: 51,
            spacing: 4.0,
            page_size: 16,
            ..Grid::default()
        },
        props,
        kinds: super::PropKinds {
            drawn_by: vec![Some(DrawnBy::Forest), Some(DrawnBy::Building), None],
            ..Default::default()
        },
        foliage,
        surface_strokes: ROAD.to_vec(),
        surface_runs: ROAD[..4].to_vec(),
        surface_triangles: court
            .iter()
            .flat_map(|t| [&t[..], &[3.0]].concat())
            .collect(),
        surface_boundaries: edges
            .iter()
            .flat_map(|e| [&e[..], &[3.0]].concat())
            .collect(),
        rivers: RIVER.to_vec(),
        river_runs: RIVER[..4].to_vec(),
        swamps: vec![],
        forests: vec![WOOD[0], WOOD[1], WOOD[2], WOOD[3], 0.0],
        forest_rect_ids: vec![0],
        forest_metadata: vec![0.0, CANOPY, 0.0, 1.0, CANOPY, 1.0],
        forest_strokes: LINE.to_vec(),
        forest_trunk_ranges: vec![0, wood, wood, line],
        ..Source::default()
    }
}

fn tuning() -> Tuning {
    let game: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../fixtures/presentation.json")).unwrap();
    serde_json::from_value(game.clone()).unwrap()
}

fn ground_with(biome: Biome) -> Ground {
    Ground::new(world(), biome, tuning()).unwrap()
}

fn ground() -> Ground {
    ground_with(Biome::load(SUMMER, None).unwrap())
}

/// Trees 11 m tall and 4 m round, shrubs 2.8 by 3, dressing 0.5 by 0.6.
fn sizes(g: &Ground) -> Vec<Size> {
    let b = &g.biome;
    let dressing: Vec<&String> = b
        .forest_floor
        .dressing
        .kinds
        .iter()
        .map(|k| &k.appearance)
        .collect();
    let size = |name: &String| match name {
        n if dressing.contains(&n) => Size {
            height: 0.5,
            radius: 0.6,
        },
        n if *n == b.trees.hedgerows.appearance || *n == b.trees.understorey.appearance => Size {
            height: 2.8,
            radius: 3.0,
        },
        _ => Size {
            height: 11.0,
            radius: 4.0,
        },
    };
    g.scenery_kinds().iter().map(size).collect()
}

fn records(placed: &[f32]) -> impl Iterator<Item = &[f32; FLOATS]> {
    placed.as_chunks::<FLOATS>().0.iter()
}

fn scenery(g: &Ground) -> Scenery {
    g.scenery(&sizes(g))
}

#[test]
fn the_forest_draws_each_trunk_once_and_names_it() {
    let g = ground();
    let s = scenery(&g);
    let trunks: Vec<u32> = g
        .source
        .props
        .as_chunks::<PROP>()
        .0
        .iter()
        .filter(|p| p[2] == 0.0)
        .map(|p| p[0] as u32)
        .collect();
    let mut named = s.forest_ids.clone();
    named.sort();
    assert_eq!(named, trunks);
    for (r, &id) in records(&s.forest).zip(&s.forest_ids) {
        let p = g
            .source
            .props
            .as_chunks::<PROP>()
            .0
            .iter()
            .find(|p| p[0] as u32 == id)
            .unwrap();
        assert_eq!([r[0], r[1]], [p[3], p[4]], "tree {id} stands on its trunk");
    }
}

#[test]
fn a_lone_trunk_is_a_tree_of_its_own_height_of_a_species_let_stand_alone() {
    let g = ground();
    let s = scenery(&g);
    let lone = records(&s.forest).find(|r| [r[0], r[1]] == LONE).unwrap();
    let kinds = g.scenery_kinds();
    assert!(g
        .biome
        .trees
        .lone
        .species
        .contains(&kinds[lone[6] as usize]));
    let [low, high] = g.biome.trees.lone.top;
    let top = (lone[5] * 11.0) as f64 / 8.0;
    assert!(
        top >= low - 1e-6 && top <= high + 1e-6,
        "top {top} of its body's height"
    );
}

#[test]
fn every_wood_crown_stays_under_its_canopy_and_within_its_appearance() {
    let g = ground();
    let s = scenery(&g);
    let [_, girth] = g.biome.trees.forest.girth;
    for r in records(&s.forest).filter(|r| [r[0], r[1]] != LONE) {
        assert!(
            r[2] + r[5] * 11.0 <= CANOPY,
            "a crown at {}, {} tops {}",
            r[0],
            r[1],
            r[2] + r[5] * 11.0
        );
        assert!(
            r[4] as f64 <= girth && girth <= 1.0,
            "no crown wider than its appearance"
        );
    }
}

#[test]
fn a_tree_line_carries_shrubs_inside_its_strip_and_a_wood_none() {
    let g = ground();
    let s = scenery(&g);
    let line = &g.forests[1];
    assert!(records(&s.understorey).count() > 10);
    for r in records(&s.understorey) {
        let reach = 3.0 * r[4] as f64;
        assert!(
            line.inside(&g.source, r[0] as f64, r[1] as f64) >= reach - 1e-6,
            "a shrub's reach leaves the strip at {}, {}",
            r[0],
            r[1]
        );
    }
}

#[test]
fn the_backdrop_stands_clear_of_the_map_and_within_reach() {
    let g = ground();
    let s = scenery(&g);
    let b = &g.biome.trees.backdrop;
    assert!(records(&s.backdrop).count() > 0);
    for r in records(&s.backdrop) {
        let (x, y) = (r[0] as f64, r[1] as f64);
        let out = libm::hypot((-x).max(x - 200.0).max(0.0), (-y).max(y - 200.0).max(0.0));
        assert!(
            out >= b.clear_m && out <= b.reach_m,
            "{x}, {y} is {out} m off the map"
        );
    }
}

#[test]
fn dressing_lies_on_forest_floor_off_paving_and_clear_of_trunks_and_bodies() {
    let g = ground();
    let s = scenery(&g);
    let rules = &g.biome.forest_floor.dressing;
    let (mut pieces, mut cells) = (0, 0);
    for c in s.dressing.cells.as_chunks::<2>().0 {
        let laid = g.dress(&s, c[0] as i64, c[1] as i64);
        assert_eq!(
            laid,
            g.dress(&s, c[0] as i64, c[1] as i64),
            "a cell is the same whenever laid"
        );
        cells += 1;
        for r in records(&laid) {
            pieces += 1;
            let (x, y) = (r[0] as f64, r[1] as f64);
            let inside = g.forests[0]
                .inside(&g.source, x, y)
                .max(g.forests[1].inside(&g.source, x, y));
            assert!(inside >= rules.edge_m - 1e-6, "{x}, {y} off the floor");
            assert!(
                stroke_inside(&ROAD, [x, y]) <= -rules.clear_m + 1e-6,
                "{x}, {y} on the road"
            );
            for p in g.source.props.as_chunks::<PROP>().0 {
                let d = libm::hypot(p[3] as f64 - x, p[4] as f64 - y);
                let clear = if p[2] == 0.0 {
                    rules.trunk_clear_m
                } else {
                    libm::hypot(p[6] as f64, p[7] as f64) + rules.clear_m
                };
                assert!(d >= clear - 1e-6, "{x}, {y} is {d} m from a body");
            }
        }
    }
    let wood_cells = (WOOD[2] / DRESSING_CELL_M as f32).ceil() as usize;
    assert!(
        cells >= wood_cells && pieces > 100,
        "{cells} cells, {pieces} pieces"
    );
}

#[test]
fn the_field_reads_the_all_primitives_distance() {
    let g = ground();
    let field = g.terrain_field().unwrap();
    let paved = |x: f64, y: f64| {
        let p = [x, y];
        let tri = |t: &[f32]| {
            triangle_contains(
                [t[0], t[1]].map(f64::from),
                [t[2], t[3]].map(f64::from),
                [t[4], t[5]].map(f64::from),
                p,
            )
        };
        let inside = g
            .source
            .surface_triangles
            .as_chunks::<7>()
            .0
            .iter()
            .any(|t| tri(t));
        let edge = |e: &[f32]| {
            distance(
                p,
                closest(p, [e[0], e[1]].map(f64::from), [e[2], e[3]].map(f64::from)),
            )
        };
        let near = g
            .source
            .surface_boundaries
            .as_chunks::<5>()
            .0
            .iter()
            .map(|e| edge(e))
            .fold(1e9, f64::min);
        stroke_inside(&ROAD, p).max(if inside { near } else { -near })
    };
    let forest = |x: f64, y: f64| {
        g.forests
            .iter()
            .map(|f| f.inside(&g.source, x, y))
            .fold(-1e9, f64::max)
    };
    let water = |x: f64, y: f64| super::water::stretch_inside(&RIVER, x, y);
    let mut rng = crate::seed::Mulberry32::new(3);
    for _ in 0..4000 {
        let (x, y) = (rng.float(-20.0, 220.0), rng.float(-20.0, 220.0));
        let want = [paved(x, y), forest(x, y), water(x, y)];
        // A pixel as wide as the map reads every distance exactly; a play
        // pixel exactly as far as its rule's reach, and its side beyond.
        let exact = field.distances([x, y, 1e9]);
        let near = field.distances([x, y, 2.0]);
        let reach = g.reach(2.0, 0.0);
        for k in 0..3 {
            assert!(
                (exact[k] - want[k]).abs() < 1e-3,
                "rule {k} at {x}, {y}: {} against {}",
                exact[k],
                want[k]
            );
            if want[k].abs() <= reach[k] {
                assert!((near[k] - want[k]).abs() < 1e-3, "rule {k} near {x}, {y}");
            } else {
                assert_eq!(
                    near[k] > 0.0,
                    want[k] > 0.0,
                    "rule {k} keeps its side at {x}, {y}"
                );
            }
        }
    }
}

#[test]
fn each_paved_kind_draws_by_its_own_row_or_the_default_and_paints_by_its_layer() {
    let mut b: serde_json::Value = serde_json::from_str(SUMMER).unwrap();
    b["roads"].as_object_mut().unwrap().remove("dirt_track");
    b["roads"]["paving"]["layer"] = 9.into();
    let g = ground_with(Biome::load(&b.to_string(), None).unwrap());
    let params = g.terrain_tables(g.terrain_field().unwrap()).params;
    // `TerrainParams`: 6 vec4s, then 14 a paved kind, then the paint order.
    let row = |tag: usize| &params[24 + tag * 56..24 + (tag + 1) * 56];
    let core = |road: &super::biome::Road| {
        linear(g.biome.palette(&road.palette)[0]).map(|c| (c as f32).to_bits())
    };
    assert_eq!(
        row(2)[..3],
        core(g.biome.road("default")),
        "a dirt track with no row of its own draws as the default's"
    );
    assert_eq!(row(3)[..3], core(g.biome.road("paving")));
    // The dirt track's default row lays it by its tag (2); the road's own row
    // at 2.5, over it; the country road by its tag (3); paving's at 9, on top.
    assert_eq!(params[24 + 4 * 56..24 + 4 * 56 + 4], [2, 0, 1, 3]);
}

#[test]
fn plain_plots_keep_their_rows_and_lose_only_their_texture() {
    let g = ground();
    let (textured, plain) = (g.plot_records(true), g.plot_records(false));
    assert_eq!(textured.len(), plain.len());
    for (k, (a, b)) in textured.iter().zip(&plain).enumerate() {
        // Per plot: the rows' break, the grain's contrast and the wheelings.
        if ![10, 13, 20].contains(&(k % 24)) {
            assert_eq!(a, b, "word {k}");
        } else {
            assert_eq!(*b, 0, "word {k}");
        }
    }
}

#[test]
fn a_points_props_cell_lists_every_prop_that_could_cover_it() {
    let g = ground();
    let table = g.grass_props();
    let f = |w: u32| f32::from_bits(w) as f64;
    let (origin, cell, across) = ([f(table[0]), f(table[1])], f(table[2]), table[4] as usize);
    let margin = g.biome.grass.clear_m.prop;
    let mut rng = crate::seed::Mulberry32::new(9);
    let mut covered = 0;
    for _ in 0..20000 {
        let (x, y) = (rng.float(0.0, 200.0), rng.float(0.0, 200.0));
        let (i, j) = (
            ((x - origin[0]) / cell).floor(),
            ((y - origin[1]) / cell).floor(),
        );
        if i < 0.0 || j < 0.0 || i as usize >= across || j as usize >= table[5] as usize {
            continue;
        }
        let c = j as usize * across + i as usize;
        let [first, count] = [table[8 + c * 2], table[9 + c * 2]].map(|w| w as usize);
        let listed: Vec<[f64; 2]> = (0..count)
            .map(|r| {
                [
                    f(table[(first + r * 2) * 4]),
                    f(table[(first + r * 2) * 4 + 1]),
                ]
            })
            .collect();
        for p in g.source.props.as_chunks::<PROP>().0 {
            let (dx, dy) = (x - p[3] as f64, y - p[4] as f64);
            let (c, s) = (libm::cos(p[5] as f64), libm::sin(p[5] as f64));
            let inside = (dx * c + dy * s).abs() <= p[6] as f64 + margin
                && (-dx * s + dy * c).abs() <= p[7] as f64 + margin;
            if inside {
                covered += 1;
                assert!(
                    listed.contains(&[p[3] as f64, p[4] as f64].map(|v| v as f32 as f64)),
                    "{x}, {y} misses a prop"
                );
            }
        }
    }
    assert!(covered > 50, "{covered} points under a prop");
}

#[test]
fn the_grass_density_setting_scales_the_densest_stand_and_nothing_else() {
    use super::grass::{KindFacts, Limits, Quality};
    let g = ground();
    let facts: Vec<KindFacts> = g
        .grass_kinds()
        .iter()
        .map(|_| KindFacts {
            mesh_height: 0.3,
            top: 0.3,
            blades: 6.0,
            root_width: 0.02,
            bases: [0, 0],
        })
        .collect();
    let limits = Limits {
        variation: 1.2,
        tallest: 0.9,
    };
    let at = |density| {
        g.grass_params(&facts, &limits, &Quality { density })
            .unwrap()
    };
    let (full, half) = (at(1.0), at(0.5));
    // `GrassParams.density.y`, word 29: the most clumps a square metre holds.
    let most = |p: &[u32]| f32::from_bits(p[29]);
    assert_eq!(most(&half), most(&full) * 0.5);
    let differ: Vec<usize> = (0..full.len()).filter(|&k| full[k] != half[k]).collect();
    assert_eq!(differ, [29]);
}
