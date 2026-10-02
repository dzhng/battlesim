//! Thin WASM boundary for physical preparation and side-filtered battle runtime.
use contract::ballistics::WeaponBallistics;
use contract::command::CommandEnvelope;
use contract::ids::{Side, UnitId};
use contract::map::MapDefinition;
use contract::scenario::{Armor, RicochetRules, Rules, ScenarioDefinition};
use contract::templates::{BuildingTemplateDescriptor, PlacementFrame, TemplateGeometryCatalog};
use sim::battle::{Battle, Replay};
use sim::damage::{decide, RoundPower, StruckHull};
use sim::flight::{
    advance_projectiles, predicted_path, prepare_launch, Aim, ArcKind, Body, BodyId, FlightConfig,
    FlightEvent, ImpactContext, NoSolution, Pose, ProjectileId, Projectiles, Shape, Struck,
};
use sim::math::{v3, V3};
use sim::publication::{self, Publisher};
use sim::rng::Rng;
use sim::village::scripts::Plan;
use sim::village::ScriptedBlue;
use sim::world::{export, WorldGeometry};
use wasm_bindgen::prelude::*;

/// Compile a physical plan with the same complete result/diagnostics as the CLI.
#[wasm_bindgen]
pub fn compile_map(request_json: &str, descriptors_json: &str) -> Result<String, JsError> {
    mapgen::compile_json(request_json, descriptors_json).map_err(js_error)
}

/// Generate a seeded plan (layout, streets, parcels and buildings): the same
/// record the CLI's `generate` prints.
#[wasm_bindgen]
pub fn generate_map_plan(
    request_json: &str,
    presets_json: &str,
    descriptors_json: &str,
) -> Result<String, JsError> {
    mapgen::generate_plan_json(request_json, presets_json, descriptors_json).map_err(js_error)
}

/// Generate a plan and compile it into a map, as the CLI's `generate-map` does.
#[wasm_bindgen]
pub fn generate_map(
    request_json: &str,
    presets_json: &str,
    descriptors_json: &str,
) -> Result<String, JsError> {
    mapgen::generate_map_json(request_json, presets_json, descriptors_json).map_err(js_error)
}

/// Plan one encounter recipe on a compiled map: the map and the sites
/// `generate_map` hands out beside it, the rules a battle on it runs under,
/// one recipe of `fixtures/encounters.json` and the encounter seed as
/// canonical decimal text. Answers with the native planner's own outcome
/// record: the legal encounter, or the diagnostics that refuse it.
#[wasm_bindgen]
pub fn plan_encounter(
    map_json: &str,
    sites_json: &str,
    rules_json: &str,
    recipe_json: &str,
    encounter_seed: &str,
) -> String {
    sim::encounter::plan_encounter_json(
        map_json,
        sites_json,
        rules_json,
        recipe_json,
        encounter_seed,
    )
}

/// Check a battle preparation request (`contract::preparation::
/// PrepareBattleRequest`) before anything is resolved: answers `{ status:
/// "ok", request }` with the request in canonical form, or `{ status:
/// "error", diagnostics }` naming the field at fault.
#[wasm_bindgen]
pub fn check_prepare_request(request_json: &str) -> String {
    contract::preparation::check_request_json(request_json)
}

/// The generator version a generation request pins: a request naming another
/// is refused, so a caller that wants this build's maps asks here.
#[wasm_bindgen]
pub fn map_generator_version() -> String {
    mapgen::layout::GENERATOR_VERSION.into()
}

/// Resolve a saved map from its documents (`fixtures/maps/<id>/map.json` and
/// `SOURCES.json`, and the physical template library those sources name; the
/// map stores each building as its template and frame, materialized here):
/// the browser's and the tools' side of the one resolver, admitted as the
/// native catalogue reader (`sim::maps`) admits it. Answers `{ status: "ok", result: { definition,
/// identity } }`, or `{ status: "error", error: { code, location, message } }`.
#[wasm_bindgen]
pub fn resolve_saved_map(
    map_json: &str,
    sources_json: &str,
    library_json: &str,
) -> Result<String, JsError> {
    let outcome: contract::maps::ResolveOutcome = contract::maps::resolve(
        map_json,
        sources_json,
        library_json,
        contract::maps::MapAdmission::CATALOGUE,
    )
    .into();
    serde_json::to_string(&outcome).map_err(js_error)
}

/// Validate and canonically identify physical templates, independent of art.
#[wasm_bindgen]
pub fn template_catalogue_json(descriptors_json: &str) -> Result<String, JsError> {
    let descriptors: Vec<BuildingTemplateDescriptor> =
        serde_json::from_str(descriptors_json).map_err(js_error)?;
    TemplateGeometryCatalog::new(descriptors)
        .map_err(js_error)?
        .canonical_json()
        .map_err(js_error)
}

/// Materialize one physical descriptor in a translation/rotation frame.
#[wasm_bindgen]
pub fn materialize_template(descriptor_json: &str, frame_json: &str) -> Result<String, JsError> {
    let descriptor: BuildingTemplateDescriptor =
        serde_json::from_str(descriptor_json).map_err(js_error)?;
    let frame: PlacementFrame = serde_json::from_str(frame_json).map_err(js_error)?;
    serde_json::to_string(&descriptor.materialize(frame).map_err(js_error)?).map_err(js_error)
}

/// Strides, field order and enum tags of the geometry exports, with the
/// prop types' body columns and appearance bindings, for `rules_json` (a scenario's rules:
/// the fixture with its catalog).
#[wasm_bindgen]
pub fn world_layout(rules_json: &str) -> Result<String, JsError> {
    let rules: Rules = serde_json::from_str(rules_json).map_err(js_error)?;
    Ok(export::layout_json(rules.catalog.props()))
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
    /// The map's geometry as the simulation builds it under `rules_json` (a
    /// scenario's rules: the fixture with its catalog).
    #[wasm_bindgen(constructor)]
    pub fn new(map_json: &str, rules_json: &str) -> Result<WorldView, JsError> {
        let map: MapDefinition = serde_json::from_str(map_json).map_err(js_error)?;
        let rules: Rules = serde_json::from_str(rules_json).map_err(js_error)?;
        Ok(WorldView {
            world: WorldGeometry::new(&map, &rules),
        })
    }

    /// Sampled surface descriptor; absent vertex pages have height zero.
    pub fn terrain_grid(&self) -> String {
        self.world.export_terrain_grid()
    }

    pub fn terrain_page_ids(&self) -> Vec<u32> {
        self.world.export_terrain_page_ids()
    }

    pub fn terrain_heights(&self) -> Vec<f32> {
        self.world.export_terrain_heights()
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

    /// Public immutable building/template references; geometry stays in props.
    pub fn buildings(&self) -> String {
        self.world.export_buildings()
    }

    pub fn props(&self) -> Vec<f32> {
        self.world.export_props()
    }

    pub fn rivers(&self) -> Vec<f32> {
        self.world.export_rivers()
    }

    pub fn river_runs(&self) -> Vec<f32> {
        self.world.export_river_runs()
    }

    pub fn forests(&self) -> Vec<f32> {
        self.world.export_forests()
    }

    pub fn forest_trunk_ranges(&self) -> Vec<u32> {
        self.world.export_forest_trunk_ranges()
    }

    pub fn forest_rect_ids(&self) -> Vec<u32> {
        self.world.export_forest_rect_ids()
    }
    pub fn forest_metadata(&self) -> Vec<f32> {
        self.world.export_forest_metadata()
    }
    pub fn forest_strokes(&self) -> Vec<f32> {
        self.world.export_forest_strokes()
    }
    pub fn forest_triangles(&self) -> Vec<f32> {
        self.world.export_forest_triangles()
    }
    pub fn forest_boundaries(&self) -> Vec<f32> {
        self.world.export_forest_boundaries()
    }

    /// Sparse foliage: `[nx, ny, cell_m]`, then non-open
    /// `[column, row, canopy_m, depth_per_m]` records in row order.
    pub fn foliage(&self) -> Vec<f32> {
        self.world.export_foliage()
    }

    /// The public static foliage minus only the ground clearing this side learned.
    /// Sorted pairs hold16×16tile ID and local start+length*256. Query only
    /// forest cells through the borrowed spans; never rebuild a cell mask.
    pub fn foliage_cleared(&self, cleared_runs: &[u32], cols: u32, cell_m: f64) -> Vec<f32> {
        self.world.export_foliage_cleared(|x, y| {
            let (i, j) = ((x / cell_m).floor(), (y / cell_m).floor());
            if i < 0.0 || j < 0.0 || i >= cols as f64 {
                return false;
            }
            let (i, j) = (i as u32, j as u32);
            let tile = j / 16 * cols.div_ceil(16) + i / 16;
            let cell = j % 16 * 16 + i % 16;
            let (mut lo, mut hi) = (0, cleared_runs.len() / 2);
            while lo < hi {
                let mid = (lo + hi) / 2;
                let (key, start) = (cleared_runs[mid * 2], cleared_runs[mid * 2 + 1] % 256);
                if key < tile || (key == tile && start <= cell) {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            if lo == 0 {
                return false;
            }
            let (key, span) = (cleared_runs[(lo - 1) * 2], cleared_runs[(lo - 1) * 2 + 1]);
            key == tile && cell < span % 256 + span / 256
        })
    }

    pub fn surface_strokes(&self) -> Vec<f32> {
        self.world.export_surface_strokes()
    }

    pub fn surface_runs(&self) -> Vec<f32> {
        self.world.export_surface_runs()
    }

    pub fn surface_triangles(&self) -> Vec<f32> {
        self.world.export_surface_triangles()
    }

    pub fn surface_boundaries(&self) -> Vec<f32> {
        self.world.export_surface_boundaries()
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
    /// `rules_json` is a scenario's rules (the fixture with its catalog):
    /// flight, ricochet and the tick rate come from it; `armor_json` is the
    /// armour of every armoured body (the tank type's `body.hull.armor`).
    #[wasm_bindgen(constructor)]
    pub fn new(
        map_json: &str,
        rules_json: &str,
        armor_json: &str,
        seed: f64,
    ) -> Result<FlightLab, JsError> {
        let map: MapDefinition = serde_json::from_str(map_json).map_err(js_error)?;
        let rules: Rules = serde_json::from_str(rules_json).map_err(js_error)?;
        let config = FlightConfig::new(&rules.physics.flight, rules.tick_hz)
            .map_err(|e| JsError::new(&format!("{e:?}")))?;
        Ok(FlightLab {
            world: WorldGeometry::new(&map, &rules),
            store: Projectiles::new(config.clone()),
            config,
            rng: Rng::new(seed as u64),
            bodies: Vec::new(),
            armored: Vec::new(),
            armor: serde_json::from_str(armor_json).map_err(js_error)?,
            ricochet: rules.ricochet.clone(),
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
            Err(NoSolution::Blocked { arc, point, .. }) => {
                let path = predicted_path(
                    &self.config,
                    &profile,
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
            decide(power, hull, hit, rules, rng)
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
            .filter_map(|e| {
                Some(match e {
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
                    // The lab wears no structures, so a round passing through a
                    // fence or crate is not one of its events.
                    FlightEvent::Pass(_) => return None,
                })
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

/// Oracle vectors for the one sight shape, `sim::sight::multiplier`: rows of
/// `[front, side, rear, off, multiplier]` over shapes and angles that cover
/// every branch (ahead, abeam, astern, past a full turn). The renderer's
/// mirrors of the rule, TypeScript and WGSL, are pinned against them.
#[wasm_bindgen]
pub fn sight_multiplier_vectors() -> Vec<f64> {
    use contract::scenario::SightShape;
    let shapes = [
        SightShape {
            front: 1.0,
            side: 1.0,
            rear: 1.0,
        },
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

/// A unit catalog's documents (`fixtures/units/**`, or a test's own)
/// resolved by the simulation's one resolver: its view
/// (`contract::catalog::Catalog::view`), or the named error it fails with.
#[wasm_bindgen]
pub fn resolve_catalog(documents_json: &str) -> Result<String, JsError> {
    // Parsed by the catalog's own reader, which refuses a repeated key.
    let documents = contract::catalog::parse_document(documents_json).map_err(js_error)?;
    let documents = documents
        .as_array()
        .ok_or_else(|| js_error("the catalog is a list of documents"))?;
    let catalog = contract::catalog::resolve(documents).map_err(js_error)?;
    Ok(catalog.view().to_string())
}

/// The village encounter for `variant`, built from the one fixture (the
/// rules, with the village's resolved map under `map`): a scenario JSON for
/// `Battle` (with its defender and completion referee).
#[wasm_bindgen]
pub fn village_scenario(fixture_json: &str, variant: &str) -> Result<String, JsError> {
    let fixture: serde_json::Value = serde_json::from_str(fixture_json).map_err(js_error)?;
    let setup = sim::village::scenario(&fixture, variant).map_err(js_error)?;
    serde_json::to_string(&setup).map_err(js_error)
}

/// The synthetic endurance battle for `seed` on its resolved field
/// (`map_json`, the catalogue's `endurance`), with the late state's remains
/// when `late`; rules from the one fixture.
#[wasm_bindgen]
pub fn endurance_scenario(
    map_json: &str,
    fixture_json: &str,
    seed: u64,
    late: bool,
) -> Result<String, JsError> {
    let field: MapDefinition = serde_json::from_str(map_json).map_err(js_error)?;
    let fixture: serde_json::Value = serde_json::from_str(fixture_json).map_err(js_error)?;
    let setup = sim::endurance::scenario(&field, &fixture, seed, late).map_err(js_error)?;
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

    /// Resolve a proposed formation without accepting an order or advancing time.
    pub fn preview_move(&mut self, side: &str, move_json: &str) -> Result<String, JsError> {
        let side = parse_side(side)?;
        let request: contract::command::MovePreviewRequest =
            serde_json::from_str(move_json).map_err(js_error)?;
        let marks = self
            .battle
            .preview_move(
                side,
                &request.units,
                request.goal,
                request.facing,
                request.direction,
            )
            .unwrap_or_default();
        serde_json::to_string(&marks).map_err(js_error)
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

    /// Pack `side`'s observation with visibility and learned-ground changes
    /// (all of them after a side change or `resync_observation`); returns its
    /// length in f32s. Read it at `publication_ptr()` before any other call
    /// that may grow memory.
    pub fn publish(&mut self, side: &str) -> Result<usize, JsError> {
        let side = parse_side(side)?;
        self.publisher
            .publish(&self.battle, side)
            .map(|record| record.len())
            .map_err(js_error)
    }

    pub fn publication_ptr(&self) -> *const f32 {
        self.publisher.record().as_ptr()
    }

    /// The next publication opens a new epoch with full visibility and learned ground.
    pub fn resync_observation(&mut self) {
        self.publisher.resync();
    }
}
