//! The helicopters' closing scene (D14), the air map's `closing` encounter as
//! its lab (`/lab/air-closing`) plays it: the Apache flies in from the map's
//! edge, pops up over the village roofs and kills the tank behind them; flying
//! on, it is shot down by the IFV the tower hid, and its wreck comes down in
//! the wood and flattens the trees there.

use contract::ids::{Side, UnitId};
use contract::scenario::Rules;
use sim::battle::Battle;
use sim::fixtures::{self, CatalogSet};
use sim::flight::{FlightEvent, Struck};
use sim::math::V2;
use sim::weapons::VEHICLE_BODY_BASE;

const APACHE: UnitId = UnitId(0);
const TANK: UnitId = UnitId(1);
const IFV: UnitId = UnitId(2);
/// The lab's seed (`apps/battle-lab/src/routes/airClosing.tsx`).
const SEED: u64 = 6;

/// The encounter as its lab runs it: the game's rules with the test set's
/// catalog (the game's units plus the test units).
fn battle() -> Battle {
    let map = sim::maps::load("air").unwrap().definition;
    let encounter = sim::maps::encounter("air", "closing").unwrap();
    let mut game = fixtures::game();
    game["catalog"] = serde_json::Value::Array(fixtures::catalog_documents(CatalogSet::Test));
    let rules: Rules = serde_json::from_value(game).unwrap();
    Battle::new(&encounter.on(map, rules), SEED)
}

/// What happened, by tick, in the beat's first 30 seconds.
#[derive(Default)]
struct Beat {
    /// The tick blue first identified the tank, and the Apache's height then.
    tank_seen: Option<(u64, f64)>,
    tank_died: Option<u64>,
    apache_died: Option<u64>,
    /// The unit whose round last struck the Apache's hull before it died.
    downed_by: Option<u32>,
}

fn play() -> (Battle, Beat) {
    let mut b = battle();
    let mut beat = Beat::default();
    let mut shooters = std::collections::BTreeMap::new();
    for _ in 0..30 * b.rules().tick_hz {
        b.step();
        let tick = b.tick();
        for e in b.flight_events() {
            let FlightEvent::Impact(i) = e else { continue };
            if beat.apache_died.is_none()
                && i.struck == Struck::Body(sim::flight::BodyId(VEHICLE_BODY_BASE + APACHE.0))
            {
                beat.downed_by = shooters.get(&i.projectile).copied();
            }
        }
        for (p, r) in b.rounds() {
            shooters.entry(p.id).or_insert(r.unit.0);
        }
        let apache = b.unit(APACHE).unwrap();
        if beat.tank_seen.is_none() && !b.observe(Side::Blue).identified.is_empty() {
            beat.tank_seen = Some((tick, apache.position.z - ground(&b, apache.position.xy())));
        }
        if beat.tank_died.is_none() && !b.unit(TANK).unwrap().alive() {
            beat.tank_died = Some(tick);
        }
        if beat.apache_died.is_none() && !apache.alive() {
            beat.apache_died = Some(tick);
        }
    }
    (b, beat)
}

fn ground(b: &Battle, at: V2) -> f64 {
    b.world().height_at(at.x, at.y).unwrap()
}

#[test]
fn the_apache_flies_in_from_the_edge_and_pops_over_the_roofs_to_find_the_tank() {
    let b = battle();
    let start = b.unit(APACHE).unwrap().position;
    assert!(
        start.x < 20.0,
        "it starts at the map's west edge: {start:?}"
    );
    let (b, beat) = play();
    let (_, height) = beat.tank_seen.expect("the Apache never saw the tank");
    let cruise = b.rules().air.cruise_agl_m;
    assert!(
        height > cruise + 5.0,
        "it saw the tank from {height:.1} m, not popped up over the roofs (cruise {cruise} m)"
    );
}

#[test]
fn the_apache_kills_the_tank_then_the_ifv_shoots_it_down() {
    let (_, beat) = play();
    let tank = beat.tank_died.expect("the tank survived");
    let apache = beat.apache_died.expect("the Apache was never shot down");
    assert!(
        tank < apache,
        "the Apache died at {apache}, before the tank at {tank}"
    );
    assert_eq!(beat.downed_by, Some(IFV.0), "the IFV did not bring it down");
}

#[test]
fn its_wreck_lands_in_the_wood_and_flattens_the_trees() {
    let (b, _) = play();
    let apache = b.rules().catalog.index("ah_64e_guardian").unwrap();
    let wreck = b
        .world()
        .props()
        .find(|p| p.wreck_of == Some(apache))
        .expect("no Apache wreck");
    let at = wreck.center;
    assert!(
        (wreck.base_z - ground(&b, at)).abs() < 1e-6,
        "the wreck hangs at {:.1} m over the ground at {:.1} m",
        wreck.base_z,
        ground(&b, at)
    );
    assert!(
        b.world()
            .forests()
            .iter()
            .any(|f| f.shape.contains([at.x, at.y], 0.0)),
        "the wreck at {at:?} lies outside every wood"
    );
    let tree = b.world().types().index(&b.rules().forests.tree).unwrap();
    let standing: Vec<V2> = b
        .world()
        .props_near(at, 5.0)
        .into_iter()
        .filter(|p| p.kind == tree && p.footprint().distance(at) < 5.0)
        .map(|p| p.center)
        .collect();
    assert!(
        standing.is_empty(),
        "trees still stand by the wreck: {standing:?}"
    );
    // The wood stood there before: the crash felled trees, rather than
    // landing in a clearing.
    let before = battle();
    let stood = before
        .world()
        .props_near(at, 5.0)
        .into_iter()
        .filter(|p| p.kind == tree && p.footprint().distance(at) < 5.0)
        .count();
    assert!(stood > 0, "no tree stood where the wreck lies at {at:?}");
}

#[test]
fn the_closing_scene_replays_to_the_same_digest() {
    assert_eq!(play().0.digest(), play().0.digest());
}
