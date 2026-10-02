//! Shared test setup. Rules always come from the one fixture owner
//! (game.json): scenario rules for authority tests; flight rules, weapon
//! rows, analytic maps and constant-velocity bodies for flight tests.
#![allow(dead_code)]
use contract::ballistics::{FlightRules, WeaponBallistics};
use contract::catalog::{PropCatalog, PropKind};
use contract::command::{CommandEnvelope, Order, OrderError};
use contract::ids::{Side, UnitId};
use contract::map::MapDefinition;
use contract::random::Rng;
use contract::scenario::{Armor, RicochetRules};
use contract::scenario::{ForestRules, ScenarioDefinition};
use serde_json::Value;
use sim::battle::Battle;
use sim::damage::{decide, RoundPower, StruckHull};
use sim::flight::{
    advance_projectiles, Body, BodyId, FlightConfig, FlightEvent, ImpactContext, ImpactDecision,
    ImpactResolver, LaunchProfile, Pose, ProjectileId, Projectiles, Shape, Struck,
};
use sim::math::{v3, V3};
use sim::world::WorldGeometry;
use std::collections::{BTreeMap, BTreeSet};

pub fn game() -> Value {
    sim::fixtures::game()
}

/// The shipped prop types.
pub fn props() -> &'static PropCatalog {
    static PROPS: std::sync::OnceLock<PropCatalog> = std::sync::OnceLock::new();
    PROPS.get_or_init(|| rules().catalog.props().clone())
}

/// The shipped prop type named `id`: what a prop of a world built on the
/// shipped rules carries as its `kind`.
pub fn kind(id: &str) -> PropKind {
    props().kind(id)
}

/// The shipped unit type named `id`: what an observed unit of a battle on
/// the shipped catalog carries as its `kind`.
pub fn unit_kind(id: &str) -> contract::catalog::TypeIndex {
    static RULES: std::sync::OnceLock<contract::scenario::Rules> = std::sync::OnceLock::new();
    let rules = RULES.get_or_init(rules);
    rules
        .catalog
        .index(id)
        .unwrap_or_else(|| panic!("no unit type {id:?}"))
}

/// The fixture's one forest rule (`forests`).
pub fn forest_rules() -> ForestRules {
    serde_json::from_value(game()["forests"].clone()).unwrap()
}

pub fn flight_rules() -> FlightRules {
    serde_json::from_value(game()["physics"].clone()).unwrap()
}

pub fn tick_hz() -> u32 {
    game()["tick_hz"].as_u64().unwrap() as u32
}

pub fn config() -> FlightConfig {
    FlightConfig::new(&flight_rules(), tick_hz()).unwrap()
}

pub fn weapon(name: &str) -> WeaponBallistics {
    serde_json::from_value(game()["weapons"][name].clone()).unwrap()
}

pub fn profile(name: &str) -> LaunchProfile {
    config().profile(&weapon(name)).unwrap()
}

pub fn physics(key: &str) -> f64 {
    game()["physics"][key].as_f64().unwrap()
}

/// A flat map `size` metres, plus extra map JSON fields (leading comma).
pub fn flat(size: [f64; 2], extra: &str) -> WorldGeometry {
    let map: MapDefinition = serde_json::from_str(&format!(
        r#"{{"size":[{},{}],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35{extra}}}"#,
        size[0], size[1]
    ))
    .unwrap();
    WorldGeometry::new(&physical_map(map, &rules()), &rules())
}

/// Analytic box inputs are authored into the same physical contract before
/// entering the world. This is test preparation, never a runtime fallback.
pub fn physical_map(mut map: MapDefinition, rules: &contract::scenario::Rules) -> MapDefinition {
    use contract::map::{BuildingDefinition, BuildingPartReference};
    use contract::templates::{
        BuildingCategory, BuildingTemplateDescriptor, PlacementFrame, TemplateGeometryCatalog,
    };
    if !map
        .props
        .iter()
        .any(|p| rules.catalog.props().by_id(&p.kind).body.garrison)
    {
        return map;
    }
    let mut ground = map.clone();
    ground.props.clear();
    ground.buildings.clear();
    ground.template_catalog_hash = None;
    let ground = WorldGeometry::new(&ground, rules);
    let mut templates = BTreeMap::new();
    let mut ordinary = Vec::new();
    for (i, mut p) in std::mem::take(&mut map.props).into_iter().enumerate() {
        let id = p.id.unwrap_or(i as u32);
        if rules.catalog.props().by_id(&p.kind).body.garrison {
            let half = p.half_extents;
            let template = BuildingTemplateDescriptor::solid_box(
                format!("api-box-{}-{}-{}", half[0], half[1], half[2]),
                BuildingCategory::Farmstead,
                "api_fixture".into(),
                half,
            );
            let base = p
                .base_z
                .unwrap_or_else(|| ground.height_at(p.center[0], p.center[1]).unwrap_or(0.0));
            map.buildings.push(
                BuildingDefinition::materialize(
                    &template,
                    PlacementFrame {
                        translation: [p.center[0], p.center[1], base],
                        yaw: p.yaw,
                    },
                    p.kind.clone(),
                    id,
                    vec![BuildingPartReference {
                        part: "body".into(),
                        prop: id,
                    }],
                )
                .unwrap(),
            );
            templates.insert(template.id.clone(), template);
        } else {
            p.id = Some(id);
            ordinary.push(p);
        }
    }
    map.props = ordinary;
    map.template_catalog_hash = Some(
        TemplateGeometryCatalog::new(templates.into_values().collect())
            .unwrap()
            .hash()
            .into(),
    );
    map
}

pub fn soldier_shape() -> Shape {
    Shape::Capsule {
        radius: physics("soldier_radius_m"),
        height: physics("soldier_height_m"),
    }
}

pub fn tank_shape() -> Shape {
    let [x, y, z] = hull("tank").half_extents_m;
    Shape::Box { half: v3(x, y, z) }
}

/// The shipped rules, catalog resolved.
pub fn rules() -> contract::scenario::Rules {
    serde_json::from_value(game()).unwrap()
}

/// A shipped unit type's hull.
pub fn hull(id: &str) -> contract::catalog::Hull {
    rules().catalog.by_id(id).hull().expect("a hull").clone()
}

/// A body moving at constant `velocity` and turning at `turn_rate` from
/// `base`/`yaw` at time zero; `at(tick)` poses it over that tick.
#[derive(Clone, Copy)]
pub struct Mover {
    pub id: u32,
    pub unit: u32,
    pub shape: Shape,
    pub base: V3,
    pub yaw: f64,
    pub velocity: V3,
    pub turn_rate: f64,
}

impl Mover {
    pub fn standing(id: u32, unit: u32, shape: Shape, base: V3) -> Self {
        Mover {
            id,
            unit,
            shape,
            base,
            yaw: 0.0,
            velocity: V3::default(),
            turn_rate: 0.0,
        }
    }

    pub fn moving(self, velocity: V3) -> Self {
        Mover { velocity, ..self }
    }

    pub fn pose(&self, t: f64) -> Pose {
        Pose {
            base: self.base + self.velocity * t,
            yaw: self.yaw + self.turn_rate * t,
        }
    }

    /// The body over tick `k` (1-based: the first advance is tick 1).
    pub fn body(&self, k: u64, dt: f64) -> Body {
        Body {
            id: BodyId(self.id),
            unit: UnitId(self.unit),
            shape: self.shape,
            from: self.pose((k - 1) as f64 * dt),
            to: self.pose(k as f64 * dt),
        }
    }
}

/// The resolver of flight tests that judge no armour: every hit stops.
pub fn stop(_: &ImpactContext) -> ImpactDecision {
    ImpactDecision::Stop
}

/// Advance until the store empties or `max_ticks`, posing bodies by `bodies(tick)`.
/// Returns each event with its tick.
pub fn fly(
    store: &mut Projectiles,
    world: &WorldGeometry,
    max_ticks: u64,
    bodies: impl FnMut(u64) -> Vec<Body>,
) -> Vec<(u64, FlightEvent)> {
    fly_with(store, world, max_ticks, bodies, &mut stop)
}

/// [`fly`], each hit judged by `resolver`.
pub fn fly_with(
    store: &mut Projectiles,
    world: &WorldGeometry,
    max_ticks: u64,
    mut bodies: impl FnMut(u64) -> Vec<Body>,
    resolver: &mut impl ImpactResolver,
) -> Vec<(u64, FlightEvent)> {
    let mut out = Vec::new();
    let mut events = Vec::new();
    for tick in 1..=max_ticks {
        if store.active().is_empty() {
            break;
        }
        events.clear();
        advance_projectiles(store, world, &bodies(tick), &mut events, resolver);
        out.extend(events.iter().map(|e| (tick, *e)));
    }
    out
}

/// Seconds since launch at which an event happened.
pub fn event_time(tick: u64, event: &FlightEvent, dt: f64) -> f64 {
    ((tick - 1) as f64 + event.time()) * dt
}

pub fn impacts(events: &[(u64, FlightEvent)]) -> Vec<(u64, sim::flight::Impact)> {
    events
        .iter()
        .filter_map(|(t, e)| match e {
            FlightEvent::Impact(i) => Some((*t, *i)),
            _ => None,
        })
        .collect()
}

/// The saved map `id` of the catalogue (`fixtures/maps/<id>`), resolved, as
/// the map JSON a scenario takes.
pub fn saved_map(id: &str) -> &'static str {
    static MAPS: std::sync::Mutex<BTreeMap<String, &'static str>> =
        std::sync::Mutex::new(BTreeMap::new());
    MAPS.lock().unwrap().entry(id.into()).or_insert_with(|| {
        let map = sim::maps::load(id).unwrap_or_else(|e| panic!("{e}"));
        &*serde_json::to_string(&map.definition).unwrap().leak()
    })
}

/// The runtime rules of a scenario: the game fixture itself (rules read the
/// sections they own and ignore the rest).
pub fn scenario_rules() -> serde_json::Value {
    game()
}

pub fn scenario(
    map: &str,
    units: serde_json::Value,
    events: serde_json::Value,
) -> ScenarioDefinition {
    scenario_with(map, units, events, serde_json::json!([]))
}

pub fn scenario_with(
    map: &str,
    units: serde_json::Value,
    events: serde_json::Value,
    scripts: serde_json::Value,
) -> ScenarioDefinition {
    let map: serde_json::Value = serde_json::from_str(map).unwrap();
    let mut setup: ScenarioDefinition = serde_json::from_value(serde_json::json!({
        "map": map, "rules": scenario_rules(), "units": units, "events": events, "scripts": scripts,
    }))
    .unwrap();
    setup.map = physical_map(setup.map, &setup.rules);
    setup
}

pub fn ricochet_rules() -> contract::scenario::RicochetRules {
    serde_json::from_value(game()["ricochet"].clone()).unwrap()
}

/// A flight-test resolver with the battle's hull policy: bodies in `hulls`
/// are tank hulls with the fixture's tank armour, and each round is judged by
/// the weapon row it was registered with.
pub struct TankHulls {
    pub hulls: BTreeSet<BodyId>,
    pub rounds: BTreeMap<ProjectileId, RoundPower>,
    pub armor: Armor,
    pub rules: RicochetRules,
    pub rng: Rng,
}

impl TankHulls {
    pub fn new(hulls: impl IntoIterator<Item = u32>, seed: u64) -> Self {
        TankHulls {
            hulls: hulls.into_iter().map(BodyId).collect(),
            rounds: BTreeMap::new(),
            armor: hull("tank").armor,
            rules: ricochet_rules(),
            rng: Rng::new(seed),
        }
    }

    /// Judge `id` as a round of the weapon row `name`.
    pub fn register(&mut self, id: ProjectileId, name: &str) {
        let row = &game()["weapons"][name];
        let power = RoundPower {
            penetration: row["penetration"].as_f64().unwrap(),
            bursts: row["blast_radius_m"].as_f64().unwrap() > 0.0,
        };
        self.rounds.insert(id, power);
    }
}

impl ImpactResolver for TankHulls {
    fn resolve(&mut self, hit: &ImpactContext) -> ImpactDecision {
        let power = self.rounds[&hit.projectile];
        let hull = match (hit.struck, hit.pose) {
            (Struck::Body(b), Some(pose)) if self.hulls.contains(&b) => {
                let Shape::Box { half } = tank_shape() else {
                    unreachable!()
                };
                Some(StruckHull {
                    armor: &self.armor,
                    half,
                    pose,
                })
            }
            _ => None,
        };
        decide(power, hull, hit, &self.rules, &mut self.rng)
    }
}

/// Hand `side`'s order to the battle as command `seq`; the order must be
/// accepted.
pub fn order(
    b: &mut sim::battle::Battle,
    side: contract::ids::Side,
    seq: u64,
    order: contract::command::Order,
) {
    let ack = b.accept(contract::command::CommandEnvelope {
        side,
        seq,
        order,
        queued: false,
    });
    assert_eq!(ack.error, None, "{ack:?}");
}

/// Orders with independent sequence numbers for each side.
pub struct Commander {
    seq: [u64; 2],
}

impl Commander {
    pub fn new() -> Self {
        Commander { seq: [0, 0] }
    }

    pub fn send(
        &mut self,
        b: &mut Battle,
        side: Side,
        order: Order,
        queued: bool,
    ) -> Option<OrderError> {
        self.seq[side.index()] += 1;
        b.accept(CommandEnvelope {
            side,
            seq: self.seq[side.index()],
            order,
            queued,
        })
        .error
    }

    pub fn ok(&mut self, b: &mut Battle, side: Side, order: Order) {
        let e = self.send(b, side, order, false);
        assert_eq!(e, None, "{side:?} command {}", self.seq[side.index()]);
    }
}

/// `BLESS_PARITY=1` re-records the native half of a `fixtures/parity/` paired
/// record from this build instead of asserting it; the web test then holds
/// Wasm to it. Only for a named behaviour change.
pub fn bless_parity(path: &str, value: &serde_json::Value) -> bool {
    if std::env::var_os("BLESS_PARITY").is_none() {
        return false;
    }
    let file = format!(
        "{}/../../fixtures/parity/{path}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::write(file, serde_json::to_string(value).unwrap()).unwrap();
    true
}

#[cfg(target_os = "macos")]
#[path = "../../examples/common/instructions.rs"]
pub mod counters;

/// Process counters need an otherwise idle test binary, even in parallel suites.
#[cfg(target_os = "macos")]
pub fn isolated_cost_test(name: &str) -> bool {
    if std::env::var("SIM_COST_TEST_CHILD").as_deref() == Ok(name) {
        return true;
    }
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--test-threads=1"])
        .env("SIM_COST_TEST_CHILD", name)
        .status()
        .unwrap();
    assert!(status.success(), "isolated {name} failed");
    false
}
