//! Ground delivery: each side learns ground cells by sight (a
//! cell is seen when its fog cell is), and receives them as patches in its
//! publication: a full snapshot when a stream starts, then only changed cells.
use std::collections::BTreeMap;

use contract::ids::Side;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::publication::{self, Publisher};

use crate::common;

/// A flat 1200 × 400 field: blue looks east from the west end, red west from
/// the east end, and the middle is out of both sides' sight.
fn field() -> String {
    json!({ "size": [1200, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] })
        .to_string()
}

fn units() -> Value {
    json!([
        { "side": "blue", "kind": "tank", "position": [100, 200] },
        { "side": "red", "kind": "tank", "position": [1100, 200], "yaw": std::f64::consts::PI },
    ])
}

/// One HE burst (the lab emitter) at each point, on `tick`.
fn bursts(tick: u64, points: &[[f64; 2]]) -> Vec<Value> {
    points
        .iter()
        .map(|p| json!({ "tick": tick, "burst": { "point": p, "weapon": "tank_he" } }))
        .collect()
}

fn battle(units: Value, events: Vec<Value>, scripts: Value) -> Battle {
    Battle::new(
        &common::scenario_with(&field(), units, Value::Array(events), scripts),
        1,
    )
}

fn drive(unit: u32, goal: [f64; 2]) -> Value {
    json!({ "tick": 1, "side": if unit == 0 { "blue" } else { "red" },
        "order": { "kind": "move", "units": [unit], "gesture": unit + 1, "goal": goal, "route": "shortest" } })
}

/// The ground patch of one packed publication, read by the layout alone.
#[derive(Debug, PartialEq)]
struct Patch {
    epoch: u32,
    side: String,
    base: u32,
    revision: u32,
    full: bool,
    /// Cell index → [crater, scorch, tracks, trampled, cleared].
    cells: Vec<(u32, [u8; 5])>,
}

fn decode_patch(layout: &Value, data: &[f32]) -> Patch {
    let header: Vec<&str> = layout["header"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let head = |name: &str| data[header.iter().position(|h| *h == name).unwrap()];
    let g = &layout["ground"];
    let fields: Vec<&str> = g["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let count = head(g["count"].as_str().unwrap()) as usize;
    let start = data.len() - count * fields.len();
    let tile_size = g["tileSize"].as_u64().unwrap() as u32;
    let cols = g["cols"].as_u64().unwrap() as u32;
    let tiles_x = cols.div_ceil(tile_size);
    let mut cells = Vec::new();
    for n in 0..count {
        let row = &data[start + n * fields.len()..];
        let f = |name: &str| row[fields.iter().position(|h| *h == name).unwrap()] as u32;
        let (tile, span) = (f("tile"), f("span"));
        let (a, b) = (f("craterScorch"), f("tracksTrampledCleared"));
        let marks = [
            (a & 0xff) as u8,
            (a >> 8) as u8,
            (b & 0xff) as u8,
            ((b >> 8) & 0xff) as u8,
            ((b >> 16) & 0xff) as u8,
        ];
        let local = span % (tile_size * tile_size);
        let len = span / (tile_size * tile_size);
        for c in local..local + len {
            let i = (tile % tiles_x) * tile_size + c % tile_size;
            let j = (tile / tiles_x) * tile_size + c / tile_size;
            cells.push((j * cols + i, marks));
        }
    }
    Patch {
        epoch: head("groundEpoch") as u32,
        side: g["sides"][head("groundSide") as usize]
            .as_str()
            .unwrap()
            .to_owned(),
        base: head("groundBase") as u32,
        revision: head("groundRevision") as u32,
        full: head("groundFull") == 1.0,
        cells,
    }
}

fn layout(b: &Battle) -> Value {
    serde_json::from_str(&publication::layout_json(b)).unwrap()
}

fn publish(p: &mut Publisher, b: &Battle, side: Side) -> Patch {
    let layout = layout(b);
    decode_patch(&layout, p.publish(b, side).unwrap())
}

/// Every cell `side` has learned, by index.
fn known(b: &Battle, side: Side) -> BTreeMap<u32, [u8; 5]> {
    let cols = b.ground().cols() as u32;
    let cell = b.ground().cell_m();
    b.known_ground(side)
        .cells()
        .map(|(x, y, c)| {
            let (i, j) = ((x / cell).round() as u32, (y / cell).round() as u32);
            (
                j * cols + i,
                [c.crater, c.scorch, c.tracks, c.trampled, c.cleared],
            )
        })
        .collect()
}

/// A consumer's view: patches applied in order, as the browser's does.
#[derive(Default)]
struct View {
    epoch: u32,
    revision: u32,
    cells: BTreeMap<u32, [u8; 5]>,
}

impl View {
    fn apply(&mut self, p: &Patch) {
        if p.full {
            assert!(p.epoch > self.epoch, "a full snapshot opens a new epoch");
            assert_eq!(p.base, 0);
            self.cells.clear();
        } else {
            assert_eq!(p.epoch, self.epoch, "a delta continues its epoch");
            assert_eq!(p.base, self.revision, "a delta starts where the view is");
        }
        for (cell, marks) in &p.cells {
            if !p.full {
                assert_ne!(
                    self.cells.get(cell).copied().unwrap_or_default(),
                    *marks,
                    "an unchanged cell is never re-sent"
                );
            }
            self.cells.insert(*cell, *marks);
        }
        self.epoch = p.epoch;
        self.revision = p.revision;
    }
}

#[test]
fn a_side_learns_only_ground_its_fog_shows() {
    let mut b = battle(
        units(),
        bursts(2, &[[300.0, 200.0], [600.0, 200.0], [900.0, 200.0]]),
        json!([]),
    );
    for _ in 0..12 {
        b.step();
    }
    let (layer, blue, red) = (
        b.ground(),
        b.known_ground(Side::Blue),
        b.known_ground(Side::Red),
    );
    for x in [300.0, 600.0, 900.0] {
        assert!(
            layer.cell(x, 200.0).crater > 0,
            "the burst at {x} dug a crater"
        );
    }
    assert_eq!(blue.cell(300.0, 200.0), layer.cell(300.0, 200.0));
    assert_eq!(red.cell(900.0, 200.0), layer.cell(900.0, 200.0));
    assert_eq!(
        blue.cell(900.0, 200.0).crater,
        0,
        "red's ground stays hidden from blue"
    );
    assert_eq!(
        red.cell(300.0, 200.0).crater,
        0,
        "blue's ground stays hidden from red"
    );
    for side in [blue, red] {
        assert_eq!(side.cell(600.0, 200.0).crater, 0, "nobody saw the middle");
    }
    // Everything a side knows, it saw: every learned cell matches the layer
    // (nothing out there changes after it is marked here).
    for side in Side::ALL {
        for (x, y, c) in b.known_ground(side).cells() {
            assert_eq!(c, b.ground().cell(x + 0.5, y + 0.5));
        }
    }
}

#[test]
fn marks_on_ground_a_side_never_saw_change_nothing_it_receives() {
    // Metamorphic: the same battle with and without red-side and no-man's-land
    // bursts. Blue's knowledge and every record it is sent stay identical,
    // byte for byte; red's differ, because red watched its craters dug.
    let mine = [[300.0, 200.0]];
    let theirs = [[600.0, 180.0], [900.0, 220.0], [1000.0, 150.0]];
    let scripts = json!([drive(0, [400.0, 260.0]), drive(1, [800.0, 140.0])]);
    let mut quiet = battle(units(), bursts(2, &mine), scripts.clone());
    let mut marked = battle(
        units(),
        [bursts(2, &mine), bursts(3, &theirs), bursts(40, &theirs)].concat(),
        scripts,
    );
    let (mut pq, mut pm) = (Publisher::new(), Publisher::new());
    let (mut rq, mut rm) = (Publisher::new(), Publisher::new());
    let mut red_differs = false;
    for _ in 0..90 {
        quiet.step();
        marked.step();
        assert_eq!(
            bits(pq.publish(&quiet, Side::Blue).unwrap()),
            bits(pm.publish(&marked, Side::Blue).unwrap()),
            "tick {}",
            quiet.tick()
        );
        red_differs |=
            rq.publish(&quiet, Side::Red).unwrap() != rm.publish(&marked, Side::Red).unwrap();
    }
    assert_eq!(known(&quiet, Side::Blue), known(&marked, Side::Blue));
    assert!(red_differs && known(&quiet, Side::Red) != known(&marked, Side::Red));
    assert!(marked.known_ground(Side::Red).cell(900.0, 220.0).crater > 0);
}

fn busy_setup() -> contract::scenario::ScenarioDefinition {
    let mut setup = common::scenario_with(
        &field(),
        units(),
        Value::Array(
            [
                bursts(2, &[[300.0, 200.0], [380.0, 150.0]]),
                bursts(30, &[[250.0, 260.0]]),
            ]
            .concat(),
        ),
        json!([drive(0, [420.0, 320.0]), drive(1, [760.0, 80.0])]),
    );
    setup.map.forests = serde_json::from_value(json!([{
        "rect": [108, 194, 65, 40], "density": "light", "canopy_height_m": 12,
        "trunk_radius_m": 0.35, "trunk_height_m": 10, "trunk_clearance_m": 2
    }]))
    .unwrap();
    setup
}

/// Blue and red tanks drive about, laying tracks where each side sees.
fn busy() -> Battle {
    Battle::new(&busy_setup(), 1)
}

#[test]
fn a_side_keeps_up_with_the_ground_it_sees() {
    // After each of blue's sweeps, every cell under a fog cell it sees is
    // known exactly as the layer holds it, though its own tracks keep
    // changing that ground.
    let mut b = busy();
    let mut checked = 0;
    for _ in 0..240 {
        b.step();
        let fog = &b.observe(Side::Blue).ground_visibility;
        if !b.tick().is_multiple_of(6) {
            continue;
        }
        let (layer, known) = (b.ground(), b.known_ground(Side::Blue));
        for (x, y, cell) in layer.cells() {
            let (cx, cy) = (x + 0.5, y + 0.5);
            if fog.visible(cx, cy) {
                assert_eq!(known.cell(cx, cy), cell, "({x}, {y}) at tick {}", b.tick());
                checked += 1;
            }
        }
    }
    assert!(checked > 1000, "{checked} seen cells checked");
}

#[test]
fn deltas_rebuild_exactly_the_side_knowledge_and_resend_nothing_unchanged() {
    let mut b = busy();
    let mut p = Publisher::new();
    let mut view = View::default();
    let (mut deltas, mut sent) = (0, 0);
    for _ in 0..240 {
        b.step();
        let patch = publish(&mut p, &b, Side::Blue);
        assert_eq!(patch.side, "blue");
        assert_eq!(patch.full, view.epoch == 0);
        if !patch.full {
            deltas += 1;
            sent += patch.cells.len();
        }
        view.apply(&patch);
        assert_eq!(view.cells, known(&b, Side::Blue), "tick {}", b.tick());
        assert_eq!(view.revision, b.known_ground(Side::Blue).revision());
    }
    assert!(
        sent > 100,
        "tracks kept arriving as deltas ({sent} cells in {deltas})"
    );
    assert!(
        view.cells.values().any(|c| c[..4] == [0; 4] && c[4] > 0),
        "clearing-only cells crossed the delivery stream"
    );
    // A new stream's snapshot is the same ground the deltas built.
    p.resync();
    let full = publish(&mut p, &b, Side::Blue);
    assert!(full.full && full.epoch == view.epoch + 1);
    assert_eq!(
        full.cells.into_iter().collect::<BTreeMap<_, _>>(),
        view.cells
    );
}

#[test]
fn a_repeated_publish_is_idempotent() {
    let mut b = busy();
    let mut p = Publisher::new();
    for _ in 0..20 {
        b.step();
    }
    let first = publish(&mut p, &b, Side::Blue);
    let again = publish(&mut p, &b, Side::Blue);
    assert!(!first.cells.is_empty());
    assert_eq!(
        again,
        Patch {
            epoch: first.epoch,
            side: "blue".into(),
            base: first.revision,
            revision: first.revision,
            full: false,
            cells: vec![],
        }
    );
    let mut view = View::default();
    view.apply(&first);
    view.apply(&again);
    assert_eq!(view.cells, known(&b, Side::Blue));
}

#[test]
fn skipped_publications_arrive_as_one_delta() {
    // Out of credits, the authority publishes nothing; the next record
    // carries every change since the consumer's cursor.
    let mut b = busy();
    let mut p = Publisher::new();
    let mut view = View::default();
    b.step();
    view.apply(&publish(&mut p, &b, Side::Blue));
    let before = view.revision;
    for _ in 0..90 {
        b.step();
    }
    let late = publish(&mut p, &b, Side::Blue);
    assert!(!late.full && late.base == before && late.revision > before);
    view.apply(&late);
    assert_eq!(view.cells, known(&b, Side::Blue));
    assert!(
        view.cells.values().any(|c| c[4] > 0),
        "clearing survives skipped publications"
    );
}

#[test]
fn a_side_switch_or_resync_opens_a_new_epoch_with_a_full_snapshot() {
    let mut b = busy();
    let mut p = Publisher::new();
    let mut view = View::default();
    for _ in 0..60 {
        b.step();
        view.apply(&publish(&mut p, &b, Side::Blue));
    }
    // Switch to red: its own ground, whole, in a new epoch; then deltas.
    let red = publish(&mut p, &b, Side::Red);
    assert!(red.full && red.side == "red" && red.epoch == view.epoch + 1);
    let mut view = View::default();
    view.apply(&red);
    assert_eq!(view.cells, known(&b, Side::Red));
    assert_ne!(view.cells, known(&b, Side::Blue));
    for _ in 0..60 {
        b.step();
        let next = publish(&mut p, &b, Side::Red);
        assert!(!next.full && next.epoch == red.epoch);
        view.apply(&next);
    }
    assert_eq!(view.cells, known(&b, Side::Red));
    // Back to blue, then a reconnect: each a full snapshot of blue.
    for _ in 0..2 {
        let blue = publish(&mut p, &b, Side::Blue);
        assert!(blue.full && blue.epoch > view.epoch);
        view.apply(&blue);
        assert_eq!(view.cells, known(&b, Side::Blue));
        assert!(
            view.cells.values().any(|c| c[4] > 0),
            "clearing survives a fresh snapshot"
        );
        p.resync();
    }
}

#[test]
fn learned_ground_replays_to_the_same_digests_and_patches() {
    let mut live = busy();
    let mut records = Vec::new();
    let mut p = Publisher::new();
    for _ in 0..120 {
        live.step();
        records.push((
            live.digest(),
            p.publish(&live, Side::Blue).unwrap().to_vec(),
        ));
    }
    let mut replay = Battle::from_replay(&busy_setup(), &live.replay()).unwrap();
    let mut p = Publisher::new();
    for (digest, record) in records {
        replay.step();
        assert_eq!(replay.digest(), digest);
        assert_eq!(bits(p.publish(&replay, Side::Blue).unwrap()), bits(&record));
    }
}

/// A record by its bits: NaN marks an absent value (a goal, a lean, an
/// area), and a NaN never equals itself as a float.
fn bits(record: &[f32]) -> Vec<u32> {
    record.iter().map(|v| v.to_bits()).collect()
}
