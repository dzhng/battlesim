//! Shared test setup. Rules always come from the one fixture owner
//! (village.json): scenario rules for authority tests; flight rules, weapon
//! rows, analytic maps and constant-velocity bodies for flight tests.
#![allow(dead_code)]
use contract::ballistics::{FlightRules, WeaponBallistics};
use contract::ids::UnitId;
use contract::map::MapDefinition;
use contract::scenario::ScenarioDefinition;
use serde_json::Value;
use sim::flight::{
    advance_projectiles, Body, BodyId, FlightConfig, FlightEvent, LaunchProfile, Pose, Projectiles,
    Shape,
};
use sim::math::{v3, V3};
use sim::world::WorldGeometry;

pub fn village() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/village.json")).unwrap()
}

pub fn flight_rules() -> FlightRules {
    serde_json::from_value(village()["physics"].clone()).unwrap()
}

pub fn tick_hz() -> u32 {
    village()["tick_hz"].as_u64().unwrap() as u32
}

pub fn config() -> FlightConfig {
    FlightConfig::new(&flight_rules(), tick_hz()).unwrap()
}

pub fn weapon(name: &str) -> WeaponBallistics {
    serde_json::from_value(village()["weapons"][name].clone()).unwrap()
}

pub fn profile(name: &str) -> LaunchProfile {
    config().profile(&weapon(name)).unwrap()
}

pub fn physics(key: &str) -> f64 {
    village()["physics"][key].as_f64().unwrap()
}

/// A flat map `size` metres, plus extra map JSON fields (leading comma).
pub fn flat(size: [f64; 2], extra: &str) -> WorldGeometry {
    let map: MapDefinition = serde_json::from_str(&format!(
        r#"{{"size":[{},{}],"height_grid_m":4,"slope_cutoff_deg":35{extra}}}"#,
        size[0], size[1]
    ))
    .unwrap();
    WorldGeometry::new(&map)
}

pub fn soldier_shape() -> Shape {
    Shape::Capsule {
        radius: physics("soldier_radius_m"),
        height: physics("soldier_height_m"),
    }
}

pub fn tank_shape() -> Shape {
    let h = &village()["physics"]["tank_half_extents_m"];
    Shape::Box {
        half: v3(
            h[0].as_f64().unwrap(),
            h[1].as_f64().unwrap(),
            h[2].as_f64().unwrap(),
        ),
    }
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

/// Advance until the store empties or `max_ticks`, posing bodies by `bodies(tick)`.
/// Returns each event with its tick.
pub fn fly(
    store: &mut Projectiles,
    world: &WorldGeometry,
    max_ticks: u64,
    mut bodies: impl FnMut(u64) -> Vec<Body>,
) -> Vec<(u64, FlightEvent)> {
    let mut out = Vec::new();
    let mut events = Vec::new();
    for tick in 1..=max_ticks {
        if store.active().is_empty() {
            break;
        }
        events.clear();
        advance_projectiles(store, world, &bodies(tick), &mut events);
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

pub const GEOMETRY_LAB: &str = include_str!("../../../../fixtures/geometry-lab.json");
pub const MOVEMENT_LAB: &str = include_str!("../../../../fixtures/movement-lab.json");
/// The runtime rules section of a scenario, picked from the village fixture.
pub fn scenario_rules() -> serde_json::Value {
    let v = village();
    serde_json::json!({
        "tick_hz": v["tick_hz"],
        "movement": v["movement"],
        "physics": v["physics"],
        "health": v["health"],
        "sensors": v["sensors"],
        "cost_priority": v["cost_priority"],
    })
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
    serde_json::from_value(serde_json::json!({
        "map": map, "rules": scenario_rules(), "units": units, "events": events, "scripts": scripts,
    }))
    .unwrap()
}
