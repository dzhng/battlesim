//! Thin WASM boundary over `sim`: commands in, side-filtered observations out.
use contract::ballistics::{FlightRules, WeaponBallistics};
use contract::command::CommandEnvelope;
use contract::ids::{Side, UnitId};
use contract::map::MapDefinition;
use contract::scenario::{Armor, ForestRules, PropTable, RicochetRules, ScenarioDefinition};
use sim::battle::{Battle, Replay};
use sim::damage::{meet_hull, RoundPower, StruckHull};
use sim::flight::{
    advance_projectiles, predicted_path, prepare_launch, Aim, ArcKind, Body, BodyId, FlightConfig,
    FlightEvent, ImpactContext, ImpactDecision, NoSolution, Pose, ProjectileId, Projectiles, Shape,
    Struck,
};
use sim::math::{v3, V3};
use sim::publication::{self, Publisher};
use sim::rng::Rng;
use sim::village::scripts::Plan;
use sim::village::ScriptedBlue;
use sim::world::{export, WorldGeometry};
use wasm_bindgen::prelude::*;

/// Build identity, so the browser can report which simulation it loaded.
#[wasm_bindgen]
pub fn build_id() -> String {
    format!("game-wasm {}", env!("CARGO_PKG_VERSION"))
}

/// Strides, field order and enum tags of the geometry exports, with the
/// body table's columns per prop kind (`props_json`: the fixture's `props`).
#[wasm_bindgen]
pub fn world_layout(props_json: &str) -> Result<String, JsError> {
    let table: PropTable = serde_json::from_str(props_json).map_err(js_error)?;
    Ok(export::layout_json(&table))
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
    /// The map's geometry, each prop with its row of `props_json` (the
    /// fixture's body table), each forest's trunks as `forests_json` (the
    /// fixture's `forests`) places them.
    #[wasm_bindgen(constructor)]
    pub fn new(map_json: &str, props_json: &str, forests_json: &str) -> Result<WorldView, JsError> {
        let map: MapDefinition = serde_json::from_str(map_json).map_err(js_error)?;
        let table: PropTable = serde_json::from_str(props_json).map_err(js_error)?;
        let forests: ForestRules = serde_json::from_str(forests_json).map_err(js_error)?;
        Ok(WorldView {
            world: WorldGeometry::new(&map, &table, &forests),
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

    /// The foliage grid (`[nx, ny, cell_m]`, then `canopy_m, depth_per_m`
    /// per cell): what sight meets under standing trees.
    pub fn foliage(&self) -> Vec<f32> {
        self.world.export_foliage()
    }

    pub fn roads(&self) -> Vec<f32> {
        self.world.export_roads()
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
/// against scripted bodies, judging armoured boxes by the battle's hull
/// policy. Player routes never construct it.
#[wasm_bindgen]
pub struct FlightLab {
    world: WorldGeometry,
    config: FlightConfig,
    store: Projectiles,
    rng: Rng,
    bodies: Vec<Body>,
    /// Bodies judged as tank hulls.
    armored: Vec<BodyId>,
    armor: Armor,
    ricochet: RicochetRules,
    /// The ricochet stream, apart from launch spread.
    ricochet_rng: Rng,
    /// What armour sees of each round fired.
    rounds: std::collections::BTreeMap<ProjectileId, RoundPower>,
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
/// Seed salt for the lab's ricochet stream.
const RICOCHET_STREAM: u64 = 0x7269_636f_6368_6574;

#[wasm_bindgen]
impl FlightLab {
    /// `physics_json` is the fixture's `physics` section, `armor_json` the
    /// armour of every armoured body (the fixture's `health.tank_armor`) and
    /// `ricochet_json` its `ricochet` section.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        map_json: &str,
        props_json: &str,
        forests_json: &str,
        physics_json: &str,
        armor_json: &str,
        ricochet_json: &str,
        tick_hz: u32,
        seed: f64,
    ) -> Result<FlightLab, JsError> {
        let map: MapDefinition = serde_json::from_str(map_json).map_err(js_error)?;
        let table: PropTable = serde_json::from_str(props_json).map_err(js_error)?;
        let forests: ForestRules = serde_json::from_str(forests_json).map_err(js_error)?;
        let rules: FlightRules = serde_json::from_str(physics_json).map_err(js_error)?;
        let config =
            FlightConfig::new(&rules, tick_hz).map_err(|e| JsError::new(&format!("{e:?}")))?;
        Ok(FlightLab {
            world: WorldGeometry::new(&map, &table, &forests),
            store: Projectiles::new(config.clone()),
            config,
            rng: Rng::new(seed as u64),
            bodies: Vec::new(),
            armored: Vec::new(),
            armor: serde_json::from_str(armor_json).map_err(js_error)?,
            ricochet: serde_json::from_str(ricochet_json).map_err(js_error)?,
            ricochet_rng: Rng::new(seed as u64 ^ RICOCHET_STREAM),
            rounds: Default::default(),
            events: Vec::new(),
        })
    }

    pub fn subsegments_per_tick(&self) -> u32 {
        self.config.subsegments_per_tick()
    }

    /// Fire `weapon_json` (a fixture weapon row) from `origin` at `target`
    /// moving at `target_velocity`, with effective spread `scatter_mrad`.
    /// Armour reads the row's `penetration` and `blast_radius_m` (0 when
    /// absent).
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
        let row: serde_json::Value = serde_json::from_str(weapon_json).map_err(js_error)?;
        let number = |key: &str| row[key].as_f64().unwrap_or(0.0);
        let power = RoundPower {
            penetration: number("penetration"),
            bursts: number("blast_radius_m") > 0.0,
        };
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
            Ok((launch, s)) => {
                let id = self.store.launch(launch);
                self.rounds.insert(id, power);
                serde_json::json!({
                "fired": true,
                "projectile": id.0,
                "arc": arc_name(s.arc),
                "velocity": xyz(s.velocity),
                "time_of_flight": s.time_of_flight_s,
                "intercept": xyz(s.intercept),
                })
            }
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
    /// shape (0 capsule, 1 box, 2 armoured box), capsule radius/height or box
    /// half x/y/z (three slots), then from x/y/z/yaw and to x/y/z/yaw.
    pub fn set_bodies(&mut self, flat: &[f64]) -> Result<(), JsError> {
        if !flat.len().is_multiple_of(BODY_STRIDE) {
            return Err(JsError::new("body records are 14 floats"));
        }
        self.armored = flat
            .chunks(BODY_STRIDE)
            .filter(|b| b[2] == 2.0)
            .map(|b| BodyId(b[0] as u32))
            .collect();
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
        let (rounds, armored, armor) = (&self.rounds, &self.armored, &self.armor);
        let (rules, rng) = (&self.ricochet, &mut self.ricochet_rng);
        let bodies = &self.bodies;
        let mut resolver = |hit: &ImpactContext| {
            let power = rounds[&hit.projectile];
            let hull = match (hit.struck, hit.pose) {
                (Struck::Body(b), Some(pose)) if armored.contains(&b) => bodies
                    .iter()
                    .find(|x| x.id == b)
                    .and_then(|x| match x.shape {
                        Shape::Box { half } => Some(StruckHull { armor, half, pose }),
                        Shape::Capsule { .. } => None,
                    }),
                _ => None,
            };
            match hull {
                Some(hull) => meet_hull(power, hull, hit, rules, rng),
                None if power.bursts => ImpactDecision::Detonate,
                None => ImpactDecision::Stop,
            }
        };
        advance_projectiles(
            &mut self.store,
            &self.world,
            &self.bodies,
            &mut self.events,
            &mut resolver,
        );
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
                    "bounces": i.bounces,
                    "detonated": i.detonated,
                }),
                FlightEvent::Ricochet(r) => serde_json::json!({
                    "kind": "ricochet",
                    "projectile": r.projectile.0,
                    "struck": struck_label(Struck::Body(r.body)),
                    "normal": xyz(r.normal),
                    "point": xyz(r.point),
                    "deflected": xyz(r.deflected),
                    "time": r.time,
                    "bounces": r.bounces,
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

/// Packs an `ObservationFrame` and a `GroundPatch` given as JSON exactly as a
/// battle publishes them: the seam decoder tests round-trip any record
/// through, whatever values a live battle happens to reach.
#[wasm_bindgen]
pub fn pack_observation(frame_json: &str, patch_json: &str) -> Result<Vec<f32>, JsError> {
    let frame = serde_json::from_str(frame_json).map_err(js_error)?;
    let patch = serde_json::from_str(patch_json).map_err(js_error)?;
    let mut out = Vec::new();
    publication::pack(&frame, &patch, &mut out);
    Ok(out)
}

/// Oracle vectors for the one sight shape, `sim::sight::multiplier`: rows of
/// `[front, side, rear, off, multiplier]` over shapes and angles that cover
/// every branch (ahead, abeam, astern, past a full turn). The renderer's
/// mirrors of the rule, TypeScript and WGSL, are pinned against them.
#[wasm_bindgen]
pub fn sight_multiplier_vectors() -> Vec<f64> {
    use contract::scenario::SightShape;
    let shapes = [
        SightShape::ISOTROPIC,
        SightShape {
            front: 1.0,
            side: 0.5,
            rear: 0.3,
        },
        SightShape {
            front: 1.0,
            side: 0.2,
            rear: 0.05,
        },
        SightShape {
            front: 0.9,
            side: 0.9,
            rear: 0.1,
        },
    ];
    let mut out = Vec::new();
    for shape in shapes {
        for k in -26..=26 {
            // Steps of π/12 across ±2π+, plus an off-grid angle per step.
            for off in [
                k as f64 * std::f64::consts::PI / 12.0,
                k as f64 * 0.2437 + 0.1,
            ] {
                let m = sim::sight::multiplier(&shape, off);
                out.extend([shape.front, shape.side, shape.rear, off, m]);
            }
        }
    }
    out
}

/// The village encounter for `variant`, built from the one fixture: a
/// scenario JSON for `Battle` (with its defender and completion referee).
#[wasm_bindgen]
pub fn village_scenario(fixture_json: &str, variant: &str) -> Result<String, JsError> {
    let fixture: serde_json::Value = serde_json::from_str(fixture_json).map_err(js_error)?;
    let setup = sim::village::scenario(&fixture, variant).map_err(js_error)?;
    serde_json::to_string(&setup).map_err(js_error)
}

/// The synthetic endurance battle for `seed`, with the late
/// state's remains when `late`; rules from the one fixture.
#[wasm_bindgen]
pub fn endurance_scenario(fixture_json: &str, seed: u64, late: bool) -> Result<String, JsError> {
    let fixture: serde_json::Value = serde_json::from_str(fixture_json).map_err(js_error)?;
    let setup = sim::endurance::scenario(&fixture, seed, late).map_err(js_error)?;
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
    /// Packs each publication and keeps the consumer's ground cursor.
    publisher: Publisher,
    /// Blue's commander when a comparison script plays it (the benchmark).
    blue: Option<ScriptedBlue>,
}

#[wasm_bindgen(js_class = Battle)]
impl BattleHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(scenario_json: &str, seed: f64) -> Result<BattleHandle, JsError> {
        let setup: ScenarioDefinition = serde_json::from_str(scenario_json).map_err(js_error)?;
        Ok(BattleHandle {
            battle: Battle::new(&setup, seed as u64),
            publisher: Publisher::new(),
            blue: None,
        })
    }

    /// A battle whose blue side is played by the comparison script `plan`
    /// (`village_report`'s name, such as "scout-suppress-flank"). Its orders
    /// go through `accept` before every step and are recorded like input, so
    /// the battle replays. The script is blue's only commander: a live blue
    /// command would be refused as out of sequence.
    pub fn scripted(scenario_json: &str, seed: f64, plan: &str) -> Result<BattleHandle, JsError> {
        let setup: ScenarioDefinition = serde_json::from_str(scenario_json).map_err(js_error)?;
        let plan = Plan::named(plan).ok_or_else(|| JsError::new(&format!("no script {plan}")))?;
        Ok(BattleHandle {
            battle: Battle::new(&setup, seed as u64),
            publisher: Publisher::new(),
            blue: Some(ScriptedBlue::new(plan, &setup)),
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
            publisher: Publisher::new(),
            blue: None,
        })
    }

    /// Returns the acknowledgement as JSON.
    pub fn accept(&mut self, command_json: &str) -> Result<String, JsError> {
        let command: CommandEnvelope = serde_json::from_str(command_json).map_err(js_error)?;
        serde_json::to_string(&self.battle.accept(command)).map_err(js_error)
    }

    pub fn step(&mut self) -> f64 {
        if let Some(blue) = self.blue.as_mut() {
            blue.command(&mut self.battle);
        }
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

    /// Field order and tags of this battle's side publications (its round
    /// kinds are the rules' weapon rows; its ground grid the layer's).
    pub fn observation_layout(&self) -> String {
        publication::layout_json(&self.battle)
    }

    /// Pack `side`'s observation with the ground cells its consumer lacks
    /// (all of them after a side change or `resync_ground`); returns its
    /// length in f32s. Read it at `publication_ptr()` before any other call
    /// that may grow memory.
    pub fn publish(&mut self, side: &str) -> Result<usize, JsError> {
        let side = parse_side(side)?;
        Ok(self.publisher.publish(&self.battle, side).len())
    }

    pub fn publication_ptr(&self) -> *const f32 {
        self.publisher.record().as_ptr()
    }

    /// The next publication opens a new ground epoch with a full snapshot.
    pub fn resync_ground(&mut self) {
        self.publisher.resync();
    }
}
