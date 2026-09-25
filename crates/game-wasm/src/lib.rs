//! Thin WASM boundary over `sim`: commands in, side-filtered observations out.
use contract::ballistics::{FlightRules, WeaponBallistics};
use contract::command::CommandEnvelope;
use contract::ids::{Side, UnitId};
use contract::map::MapDefinition;
use contract::scenario::ScenarioDefinition;
use sim::battle::{Battle, Replay};
use sim::flight::{
    advance_projectiles, predicted_path, prepare_launch, Aim, ArcKind, Body, BodyId, FlightConfig,
    FlightEvent, NoSolution, Pose, Projectiles, Shape, Struck,
};
use sim::math::{v3, V3};
use sim::publication;
use sim::rng::Rng;
use sim::world::{export, WorldGeometry};
use wasm_bindgen::prelude::*;

/// Build identity, so the browser can report which simulation it loaded.
#[wasm_bindgen]
pub fn build_id() -> String {
    format!("game-wasm {}", env!("CARGO_PKG_VERSION"))
}

/// Strides, field order and enum tags of the geometry exports.
#[wasm_bindgen]
pub fn world_layout() -> String {
    export::layout_json()
}

/// Lab-only view of authoritative world geometry: exported meshes and direct
/// queries for the geometry probe. Player routes read geometry through the
/// battle authority instead.
#[wasm_bindgen]
pub struct WorldView {
    world: WorldGeometry,
}

#[wasm_bindgen]
impl WorldView {
    #[wasm_bindgen(constructor)]
    pub fn new(map_json: &str) -> Result<WorldView, JsError> {
        let map: MapDefinition =
            serde_json::from_str(map_json).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(WorldView {
            world: WorldGeometry::new(&map),
        })
    }

    pub fn terrain_positions(&self) -> Vec<f32> {
        self.world.export_terrain_positions()
    }

    pub fn terrain_indices(&self) -> Vec<u32> {
        self.world.export_terrain_indices()
    }

    pub fn terrain_triangle_surfaces(&self) -> Vec<u8> {
        self.world.export_terrain_triangle_surfaces()
    }

    pub fn props(&self) -> Vec<f32> {
        self.world.export_props()
    }

    pub fn water(&self) -> Vec<f32> {
        self.world.export_water()
    }

    pub fn forests(&self) -> Vec<f32> {
        self.world.export_forests()
    }

    pub fn slope_cutoff_deg(&self) -> f64 {
        self.world.slope_cutoff_deg()
    }

    pub fn height_at(&self, x: f64, y: f64) -> Option<f64> {
        self.world.height_at(x, y)
    }

    pub fn surface_at(&self, x: f64, y: f64) -> Vec<f64> {
        export::surface_record(self.world.surface_at(x, y))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn raycast(
        &self,
        ox: f64,
        oy: f64,
        oz: f64,
        dx: f64,
        dy: f64,
        dz: f64,
        max_t: f64,
    ) -> Vec<f64> {
        let dir = v3(dx, dy, dz).normalized();
        export::hit_record(self.world.raycast(v3(ox, oy, oz), dir, max_t))
    }

    pub fn obstacle_revision(&self) -> f64 {
        self.world.obstacle_revision() as f64
    }
}

/// Lab-only flight bench over authoritative geometry: fires the launch a
/// weapon would (solve, spread, launch) and steps the one projectile store
/// against scripted bodies. Player routes never construct it.
#[wasm_bindgen]
pub struct FlightLab {
    world: WorldGeometry,
    config: FlightConfig,
    store: Projectiles,
    rng: Rng,
    bodies: Vec<Body>,
    events: Vec<FlightEvent>,
}

fn v3_of(v: &[f64]) -> Result<V3, JsError> {
    match v {
        [x, y, z] => Ok(v3(*x, *y, *z)),
        _ => Err(JsError::new("expected [x, y, z]")),
    }
}

fn xyz(v: V3) -> serde_json::Value {
    serde_json::json!([v.x, v.y, v.z])
}

fn struck_label(s: Struck) -> String {
    match s {
        Struck::Terrain => "terrain".into(),
        Struck::Prop(id) => format!("prop:{id}"),
        Struck::Body(id) => format!("body:{}", id.0),
    }
}

/// Floats per body in [`FlightLab::set_bodies`].
const BODY_STRIDE: usize = 14;

#[wasm_bindgen]
impl FlightLab {
    /// `physics_json` is the fixture's `physics` section.
    #[wasm_bindgen(constructor)]
    pub fn new(
        map_json: &str,
        physics_json: &str,
        tick_hz: u32,
        seed: f64,
    ) -> Result<FlightLab, JsError> {
        let map: MapDefinition = serde_json::from_str(map_json).map_err(js_error)?;
        let rules: FlightRules = serde_json::from_str(physics_json).map_err(js_error)?;
        let config =
            FlightConfig::new(&rules, tick_hz).map_err(|e| JsError::new(&format!("{e:?}")))?;
        Ok(FlightLab {
            world: WorldGeometry::new(&map),
            store: Projectiles::new(config.clone()),
            config,
            rng: Rng::new(seed as u64),
            bodies: Vec::new(),
            events: Vec::new(),
        })
    }

    pub fn subsegments_per_tick(&self) -> u32 {
        self.config.subsegments_per_tick()
    }

    /// Fire `weapon_json` (a fixture weapon row) from `origin` at `target`
    /// moving at `target_velocity`, with effective spread `scatter_mrad`.
    /// Returns JSON: `{fired, projectile?, arc, velocity, time_of_flight,
    /// intercept}` or `{fired: false, reason, arc?, path?, blocked_at?}`.
    pub fn fire(
        &mut self,
        weapon_json: &str,
        origin: &[f64],
        target: &[f64],
        target_velocity: &[f64],
        scatter_mrad: f64,
    ) -> Result<String, JsError> {
        let weapon: WeaponBallistics = serde_json::from_str(weapon_json).map_err(js_error)?;
        let profile = self
            .config
            .profile(&weapon)
            .map_err(|e| JsError::new(&format!("{e:?}")))?;
        let aim = Aim {
            origin: v3_of(origin)?,
            target: v3_of(target)?,
            target_velocity: v3_of(target_velocity)?,
        };
        let arc_name = |a: ArcKind| format!("{a:?}").to_lowercase();
        let out = match prepare_launch(
            &self.world,
            &self.config,
            &profile,
            &aim,
            scatter_mrad,
            &mut self.rng,
            None,
        ) {
            Ok((launch, s)) => serde_json::json!({
                "fired": true,
                "projectile": self.store.launch(launch).0,
                "arc": arc_name(s.arc),
                "velocity": xyz(s.velocity),
                "time_of_flight": s.time_of_flight_s,
                "intercept": xyz(s.intercept),
            }),
            Err(NoSolution::OutOfReach) => {
                serde_json::json!({ "fired": false, "reason": "out_of_reach" })
            }
            Err(NoSolution::Blocked { arc, point }) => {
                let path = predicted_path(
                    &self.config,
                    profile.gravity(&self.config),
                    aim.origin,
                    arc.velocity,
                    arc.time_of_flight_s,
                );
                serde_json::json!({
                    "fired": false,
                    "reason": "blocked",
                    "arc": arc_name(arc.arc),
                    "blocked_at": xyz(point),
                    "path": path.into_iter().map(xyz).collect::<Vec<_>>(),
                })
            }
        };
        Ok(out.to_string())
    }

    /// Bodies over the next step, [`BODY_STRIDE`] floats each: id, unit,
    /// shape (0 capsule, 1 box), capsule radius/height or box half x/y/z
    /// (three slots), then from x/y/z/yaw and to x/y/z/yaw.
    pub fn set_bodies(&mut self, flat: &[f64]) -> Result<(), JsError> {
        if !flat.len().is_multiple_of(BODY_STRIDE) {
            return Err(JsError::new("body records are 14 floats"));
        }
        self.bodies = flat
            .chunks(BODY_STRIDE)
            .map(|b| Body {
                id: BodyId(b[0] as u32),
                unit: UnitId(b[1] as u32),
                shape: if b[2] == 0.0 {
                    Shape::Capsule {
                        radius: b[3],
                        height: b[4],
                    }
                } else {
                    Shape::Box {
                        half: v3(b[3], b[4], b[5]),
                    }
                },
                from: Pose {
                    base: v3(b[6], b[7], b[8]),
                    yaw: b[9],
                },
                to: Pose {
                    base: v3(b[10], b[11], b[12]),
                    yaw: b[13],
                },
            })
            .collect();
        Ok(())
    }

    /// Advance one tick. Returns JSON `{events: [...], rounds: [[id, x, y, z], ...]}`
    /// with events in the store's order.
    pub fn step(&mut self) -> String {
        self.events.clear();
        advance_projectiles(&mut self.store, &self.world, &self.bodies, &mut self.events);
        let events: Vec<serde_json::Value> = self
            .events
            .iter()
            .map(|e| match e {
                FlightEvent::Impact(i) => serde_json::json!({
                    "kind": "impact",
                    "projectile": i.projectile.0,
                    "struck": struck_label(i.struck),
                    "normal": xyz(i.normal),
                    "point": xyz(i.point),
                    "time": i.time,
                }),
                FlightEvent::NearMiss(m) => serde_json::json!({
                    "kind": "near_miss",
                    "projectile": m.projectile.0,
                    "unit": m.unit.0,
                    "body": m.body.0,
                    "distance": m.distance,
                    "point": xyz(m.point),
                    "time": m.time,
                }),
                FlightEvent::Expired(x) => serde_json::json!({
                    "kind": "expired",
                    "projectile": x.projectile.0,
                    "cause": format!("{:?}", x.cause).to_lowercase(),
                    "point": xyz(x.point),
                    "time": x.time,
                }),
            })
            .collect();
        let rounds: Vec<serde_json::Value> = self
            .store
            .active()
            .iter()
            .map(|p| serde_json::json!([p.id.0, p.position.x, p.position.y, p.position.z]))
            .collect();
        serde_json::json!({ "events": events, "rounds": rounds }).to_string()
    }
}

/// Field order and tags of a side publication.
#[wasm_bindgen]
pub fn observation_layout() -> String {
    publication::layout_json()
}

/// The village encounter for `variant`, built from the one fixture: a
/// scenario JSON for `Battle` (with its defender and completion referee).
#[wasm_bindgen]
pub fn village_scenario(fixture_json: &str, variant: &str) -> Result<String, JsError> {
    let fixture: serde_json::Value = serde_json::from_str(fixture_json).map_err(js_error)?;
    let setup = sim::village::scenario(&fixture, variant).map_err(js_error)?;
    serde_json::to_string(&setup).map_err(js_error)
}

fn js_error(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

fn parse_side(side: &str) -> Result<Side, JsError> {
    serde_json::from_value(serde_json::Value::String(side.to_owned())).map_err(js_error)
}

/// The battle authority behind the worker. Small rare messages cross as JSON;
/// each tick's side observation is packed into a reusable buffer the host
/// copies out of WASM memory.
#[wasm_bindgen(js_name = Battle)]
pub struct BattleHandle {
    battle: Battle,
    publication: Vec<f32>,
}

#[wasm_bindgen(js_class = Battle)]
impl BattleHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(scenario_json: &str, seed: f64) -> Result<BattleHandle, JsError> {
        let setup: ScenarioDefinition = serde_json::from_str(scenario_json).map_err(js_error)?;
        Ok(BattleHandle {
            battle: Battle::new(&setup, seed as u64),
            publication: Vec::new(),
        })
    }

    /// A battle that re-applies a recorded replay and refuses live commands.
    pub fn from_replay(scenario_json: &str, replay_json: &str) -> Result<BattleHandle, JsError> {
        let setup: ScenarioDefinition = serde_json::from_str(scenario_json).map_err(js_error)?;
        let replay: Replay = serde_json::from_str(replay_json).map_err(js_error)?;
        let battle = Battle::from_replay(&setup, &replay)
            .map_err(|e| JsError::new(&format!("replay does not match this scenario: {e:?}")))?;
        Ok(BattleHandle {
            battle,
            publication: Vec::new(),
        })
    }

    /// Returns the acknowledgement as JSON.
    pub fn accept(&mut self, command_json: &str) -> Result<String, JsError> {
        let command: CommandEnvelope = serde_json::from_str(command_json).map_err(js_error)?;
        serde_json::to_string(&self.battle.accept(command)).map_err(js_error)
    }

    pub fn step(&mut self) -> f64 {
        self.battle.step() as f64
    }

    pub fn tick(&self) -> f64 {
        self.battle.tick() as f64
    }

    /// State digest as 16 hex digits (a u64 does not fit a JS number).
    pub fn digest(&self) -> String {
        format!("{:016x}", self.battle.digest())
    }

    pub fn replay_json(&self) -> Result<String, JsError> {
        serde_json::to_string(&self.battle.replay()).map_err(js_error)
    }

    /// Pack `side`'s observation; returns its length in f32s. Read it at
    /// `publication_ptr()` before any other call that may grow memory.
    pub fn publish(&mut self, side: &str) -> Result<usize, JsError> {
        let side = parse_side(side)?;
        publication::pack(self.battle.observe(side), &mut self.publication);
        Ok(self.publication.len())
    }

    pub fn publication_ptr(&self) -> *const f32 {
        self.publication.as_ptr()
    }
}
