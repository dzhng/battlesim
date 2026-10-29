//! Thin WASM boundary for physical preparation and side-filtered battle runtime.
use contract::ballistics::WeaponBallistics;
use contract::command::CommandEnvelope;
use contract::ids::{Side, UnitId};
use contract::map::MapDefinition;
use contract::random::Rng;
use contract::scenario::{Armor, RicochetRules, Rules, ScenarioDefinition};
use contract::templates::{BuildingTemplateDescriptor, TemplateGeometryCatalog};
use sim::battle::{Battle, Replay};
use sim::damage::{decide, RoundPower, StruckHull};
use sim::flight::{
    advance_projectiles, fire_round, predicted_path, Aim, ArcKind, Body, BodyId, FlightConfig,
    FlightEvent, ImpactContext, NoSolution, Pose, ProjectileId, Projectiles, Shape, Struck,
};
use sim::math::{v3, V3};
use sim::publication::{self, Publisher};
use sim::world::WorldGeometry;
use wasm_bindgen::prelude::*;

mod skirmish;
mod world_view;
pub use skirmish::synthetic_skirmish_setup;

/// Generate a plan and compile it into a map, as the CLI's `generate-map` does.
#[wasm_bindgen]
pub fn generate_map(
    request_json: &str,
    presets_json: &str,
    descriptors_json: &str,
    rules_json: &str,
) -> Result<String, JsError> {
    mapgen::generate_map_json(request_json, presets_json, descriptors_json, rules_json)
        .map_err(js_error)
}

/// Check a battle preparation request (`contract::preparation::
/// PrepareBattleRequest`) before anything is resolved: answers `{ status:
/// "ok", request }` with the request in canonical form, or `{ status:
/// "error", diagnostics }` naming the field at fault.
#[wasm_bindgen]
pub fn check_prepare_request(request_json: &str) -> String {
    contract::preparation::check_request_json(request_json)
}

/// Check the assembled scenario contract before a prepared world is consumed
/// by the battle authority. This keeps admission refusals on the normal
/// preparation diagnostic path instead of allowing a constructor panic.
#[wasm_bindgen]
pub fn check_scenario_player_contract(scenario_json: &str) -> String {
    let result = serde_json::from_str::<ScenarioDefinition>(scenario_json)
        .map_err(|error| error.to_string())
        .and_then(|scenario| scenario.validate_player_contract());
    match result {
        Ok(()) => serde_json::json!({ "status": "ok" }).to_string(),
        Err(message) => serde_json::json!({
            "status": "error",
            "diagnostics": [{
                "code": "invalid_scenario",
                "feature": "players",
                "location": "$.players",
                "message": message,
            }],
        })
        .to_string(),
    }
}

/// Validate a downloaded ScenarioBundle without consulting repository
/// fixtures. The bundle module owns canonical bytes, identities, topology,
/// and embedded asset admission.
#[wasm_bindgen]
pub fn check_scenario_bundle(bundle_json: &str) -> String {
    match contract::bundle::ScenarioBundle::from_json(bundle_json) {
        Ok(bundle) => serde_json::json!({
            "status": "ok",
            "identity": bundle.identity().expect("validated bundle identity"),
        })
        .to_string(),
        Err(contract::bundle::BundleError::Invalid(diagnostics)) => {
            serde_json::json!({ "status": "error", "diagnostics": diagnostics }).to_string()
        }
    }
}

/// Canonicalize a builder draft after deriving its map and rules identities.
/// This is intentionally separate from upload admission: saved files must
/// already carry the identities they claim.
#[wasm_bindgen]
pub fn author_scenario_bundle(bundle_json: &str) -> Result<String, JsError> {
    contract::bundle::ScenarioBundle::from_authoring_json(bundle_json)
        .map_err(js_error)?
        .to_json()
        .map_err(js_error)
}

/// Return the canonical bytes of a validated ScenarioBundle.
#[wasm_bindgen]
pub fn canonical_scenario_bundle(bundle_json: &str) -> Result<String, JsError> {
    let bundle = contract::bundle::ScenarioBundle::from_json(bundle_json)
        .map_err(|error| js_error(error.to_string()))?;
    bundle
        .to_json()
        .map_err(|error| js_error(error.to_string()))
}

/// Run one authored script through the same deterministic contract host used
/// by scenario battles. The observation JSON is the caller's side-scoped frame.
#[wasm_bindgen]
pub fn run_scenario_script(
    source: &str,
    side: &str,
    observation_json: &str,
    seed: f64,
    player: Option<String>,
) -> Result<String, JsError> {
    let side = parse_side(side)?;
    let observation: contract::observation::ObservationFrame =
        serde_json::from_str(observation_json).map_err(js_error)?;
    let input = contract::script::ScriptInput::new(side, &observation, seed as u64);
    let input = match player {
        Some(player) => input.with_player(player),
        None => input,
    };
    let script = contract::script::CompiledScript::compile(source).map_err(js_error)?;
    serde_json::to_string(&script.run(&input).map_err(js_error)?).map_err(js_error)
}

/// Resolve the effective controller state for a play session. The caller
/// supplies the one human-capable player selected on the play screen.
#[wasm_bindgen]
pub fn scenario_bundle_controllers(
    bundle_json: &str,
    active_human: Option<String>,
) -> Result<String, JsError> {
    let bundle = contract::bundle::ScenarioBundle::from_json(bundle_json).map_err(js_error)?;
    serde_json::to_string(
        &bundle
            .effective_controllers(active_human.as_deref())
            .map_err(js_error)?,
    )
    .map_err(js_error)
}

/// Return the resolved scenario and preparation metadata from an admitted
/// bundle. This is the browser's read-only bridge for camera/loading state;
/// map geometry and counts are still computed here with the same prepared-world
/// owner used by ordinary play.
#[wasm_bindgen]
pub fn scenario_bundle_details(bundle_json: &str) -> Result<String, JsError> {
    let bundle = contract::bundle::ScenarioBundle::from_json(bundle_json).map_err(js_error)?;
    let identity = bundle.identity().map_err(js_error)?;
    let scenario_json = bundle.scenario_json().map_err(js_error)?;
    let map = bundle.scenario.map.clone();
    let world = PreparedWorld::build(map.clone(), bundle.scenario.rules.clone())?;
    let start = bundle
        .scenario
        .units
        .iter()
        .find(|unit| unit.side == Side::Blue)
        .map(|unit| serde_json::json!({ "at": unit.position, "yaw": unit.yaw }))
        .unwrap_or_else(
            || serde_json::json!({ "at": [map.size[0] / 2.0, map.size[1] / 2.0], "yaw": 0.0 }),
        );
    let counts = serde_json::json!({
        "buildings": map.buildings.len(),
        "parts": map.buildings.iter().map(|building| building.parts.len()).sum::<usize>(),
        "props": map.props.len(),
        "surfaces": map.surfaces.len(),
        "forests": map.forests.len(),
    });
    let assets: Vec<_> = bundle
        .assets
        .iter()
        .filter(|asset| asset.media_type.starts_with("audio/"))
        .map(|asset| {
            serde_json::json!({
                "id": asset.id,
                "media_type": asset.media_type,
                "bytes_base64": asset.bytes_base64,
            })
        })
        .collect();
    let human_capable_players: Vec<_> = bundle
        .scenario
        .players
        .iter()
        .filter(|player| player.human_capable)
        .map(|player| player.id.clone())
        .collect();
    Ok(serde_json::json!({
        "scenario": scenario_json,
        "identity": identity,
        "size": map.size,
        "extents": serde_json::from_str::<serde_json::Value>(&world.extents())
            .map_err(js_error)?,
        "counts": counts,
        "assets": assets,
        "human_capable_players": human_capable_players,
        "start": start,
        "town": serde_json::Value::Null,
    })
    .to_string())
}

/// The lobby's picture of an admitted map (`mapgen::inspect::skirmish_picture`):
/// `map_json` is its `MapDefinition`, `sites_json` the admitted
/// `SkirmishSites`. Answers the SVG text.
#[wasm_bindgen]
pub fn skirmish_picture(map_json: &str, sites_json: &str) -> Result<String, JsError> {
    let map: MapDefinition = serde_json::from_str(map_json).map_err(js_error)?;
    let sites: contract::encounter::SkirmishSites =
        serde_json::from_str(sites_json).map_err(js_error)?;
    Ok(mapgen::inspect::skirmish_picture(&map, &sites))
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

/// Admit physical templates as buildings a map may place, together one
/// canonical catalogue (`{ hash, templates }`), independent of art. The asset
/// check holds the catalogue's descriptors to this, so no other code decides
/// what a legal template is. Fails with the contract's own refusal.
#[wasm_bindgen]
pub fn template_catalogue_json(descriptors_json: &str) -> Result<String, JsError> {
    let descriptors: Vec<BuildingTemplateDescriptor> =
        serde_json::from_str(descriptors_json).map_err(js_error)?;
    TemplateGeometryCatalog::new(descriptors)
        .map_err(js_error)?
        .canonical_json()
        .map_err(js_error)
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
            // The ballistics lab fires from a standing mount.
            carrier: V3::default(),
        };
        let min_range_m = number("min_range_m");
        if min_range_m > 0.0 && (aim.target - aim.origin).length() < min_range_m {
            return Ok(serde_json::json!({ "fired": false, "reason": "holding_fire" }).to_string());
        }
        let arc_name = |a: ArcKind| format!("{a:?}").to_lowercase();
        // The lab fires into nothing and holds no round.
        let launched = fire_round(
            &self.world,
            &self.config,
            &profile,
            &aim,
            scatter_mrad,
            &mut self.rng,
            None,
            |_| false,
            |_| false,
        );
        let out = match launched {
            Ok(None) => unreachable!("the lab holds no round"),
            Ok(Some((launch, s))) => {
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

/// The scenario a saved encounter (`encounter_json`, its file's text) makes on
/// its resolved map under `rules_json`, its overrides merged in, as
/// `EncounterDefinition::on` makes it natively; or the refusal naming the
/// encounter's field at fault.
#[wasm_bindgen]
pub fn encounter_scenario(
    map_json: &str,
    encounter_json: &str,
    rules_json: &str,
) -> Result<String, JsError> {
    let map: MapDefinition = serde_json::from_str(map_json).map_err(js_error)?;
    let encounter: contract::scenario::EncounterDefinition =
        serde_json::from_str(encounter_json).map_err(js_error)?;
    let rules: Rules = serde_json::from_str(rules_json).map_err(js_error)?;
    let setup = encounter.on(map, rules).map_err(js_error)?;
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

/// The full generated world with the simulation-owned central contact stress recipe.
#[wasm_bindgen]
pub fn city_stress_preparation(
    map_json: &str,
    fixture_json: &str,
    seed: u64,
    late: bool,
) -> Result<String, JsError> {
    let field: MapDefinition = serde_json::from_str(map_json).map_err(js_error)?;
    let fixture: serde_json::Value = serde_json::from_str(fixture_json).map_err(js_error)?;
    let setup = sim::endurance::city_scenario(&field, &fixture, seed, late).map_err(js_error)?;
    let first = setup
        .units
        .iter()
        .find(|u| u.side == Side::Blue)
        .ok_or_else(|| js_error("the city stress fixture has no blue unit"))?;
    let report = serde_json::json!({
        "start": { "at": first.position, "yaw": first.yaw },
        "livingUnits": {
            "blue": setup.units.iter().filter(|u| u.side == Side::Blue && u.condition.is_none()).count(),
            "red": setup.units.iter().filter(|u| u.side == Side::Red && u.condition.is_none()).count(),
        },
    });
    // Carry canonical scenario bytes without materializing the full map again
    // in JavaScript merely to obtain its small preparation metadata.
    Ok(format!(
        "{{\"scenario\":{},\"report\":{}}}",
        serde_json::to_string(&setup).map_err(js_error)?,
        serde_json::to_string(&report).map_err(js_error)?
    ))
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
}

#[wasm_bindgen(js_class = Battle)]
impl BattleHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(scenario_json: &str, seed: f64) -> Result<BattleHandle, JsError> {
        let setup: ScenarioDefinition = serde_json::from_str(scenario_json).map_err(js_error)?;
        Ok(BattleHandle {
            battle: Battle::new(&setup, seed as u64),
            publisher: Publisher::new(),
        })
    }

    /// Set play-time controller ownership before the first tick.
    pub fn set_controllers(&mut self, controllers_json: &str) -> Result<(), JsError> {
        let controllers: Vec<(String, contract::bundle::ControllerKind)> =
            serde_json::from_str(controllers_json).map_err(js_error)?;
        self.battle.set_controllers(controllers);
        Ok(())
    }

    /// A battle that re-applies a recorded replay and refuses live commands.
    pub fn from_replay(scenario_json: &str, replay_json: &str) -> Result<BattleHandle, JsError> {
        let setup: ScenarioDefinition = serde_json::from_str(scenario_json).map_err(js_error)?;
        let replay: Replay = serde_json::from_str(replay_json).map_err(js_error)?;
        let battle =
            Battle::from_replay(&setup, &replay).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(BattleHandle {
            battle,
            publisher: Publisher::new(),
        })
    }

    /// Returns the acknowledgement as JSON.
    pub fn accept(&mut self, command_json: &str) -> Result<String, JsError> {
        let command: CommandEnvelope = serde_json::from_str(command_json).map_err(js_error)?;
        serde_json::to_string(&self.battle.accept(command)).map_err(js_error)
    }

    /// Check a ghost destination using only the purchasing side's geometry.
    pub fn preview_purchase(
        &mut self,
        side: &str,
        variant: &str,
        x: f64,
        y: f64,
    ) -> Result<String, JsError> {
        let side = parse_side(side)?;
        serde_json::to_string(&self.battle.preview_purchase(side, variant, [x, y]))
            .map_err(js_error)
    }

    /// Resolve a proposed formation without accepting an order or advancing time.
    pub fn preview_move(&mut self, side: &str, move_json: &str) -> Result<String, JsError> {
        let side = parse_side(side)?;
        let request: contract::command::MovePreviewRequest =
            serde_json::from_str(move_json).map_err(js_error)?;
        let marks = self.battle.preview_move(side, &request).unwrap_or_default();
        serde_json::to_string(&marks).map_err(js_error)
    }

    /// Resolve a combined building action without accepting an order or advancing time.
    pub fn preview_building(&self, side: &str, request_json: &str) -> Result<String, JsError> {
        let side = parse_side(side)?;
        let request: contract::command::BuildingPreviewRequest =
            serde_json::from_str(request_json).map_err(js_error)?;
        let placement = self
            .battle
            .preview_building(side, &request)
            .map_err(|e| js_error(format!("{e:?}")))?;
        serde_json::to_string(&placement).map_err(js_error)
    }

    pub fn step(&mut self) -> f64 {
        self.battle.step() as f64
    }

    pub fn tick(&self) -> f64 {
        self.battle.tick() as f64
    }

    pub fn finished(&self) -> bool {
        self.battle.finished()
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

/// A worker-owned world: skirmish admission lends it, battle startup consumes it.
#[wasm_bindgen]
pub struct PreparedWorld {
    map: MapDefinition,
    rules: Rules,
    prepared: sim::encounter::PreparedMap,
    map_digest: u64,
    rules_digest: u64,
}

#[wasm_bindgen]
impl PreparedWorld {
    /// Final skirmish placement/route admission on this prepared physical world.
    pub fn admit_skirmish(&self, sites_json: &str, players_json: &str) -> Result<String, JsError> {
        let sites = serde_json::from_str(sites_json).map_err(js_error)?;
        let players: Vec<contract::players::Player> =
            serde_json::from_str(players_json).map_err(js_error)?;
        let admitted = sim::map_analysis::admit_skirmish(
            &self.prepared.queries(&self.map, &sites),
            &self.rules,
            &players,
        )
        .map_err(js_error)?;
        serde_json::to_string(&admitted).map_err(js_error)
    }
    /// Build the complete generated skirmish scenario from this prepared
    /// world's own map and rules serialization.
    pub fn skirmish_scenario(
        &self,
        sites_json: &str,
        players_json: &str,
    ) -> Result<String, JsError> {
        let sites: contract::encounter::EncounterSites =
            serde_json::from_str(sites_json).map_err(js_error)?;
        let players: Vec<contract::players::Player> =
            serde_json::from_str(players_json).map_err(js_error)?;
        let setup = sim::skirmish::setup_for(
            &players,
            sites
                .skirmish
                .ok_or_else(|| JsError::new("skirmish sites are missing"))?,
        );
        setup.sites.validate().map_err(js_error)?;
        serde_json::to_string(&ScenarioDefinition {
            players,
            skirmish: Some(setup),
            map: self.map.clone(),
            rules: self.rules.clone(),
            units: vec![],
            events: vec![],
            scripts: vec![],
            authored_scripts: vec![],
            victory: None,
            opponent: None,
            encounter: None,
        })
        .map_err(js_error)
    }

    #[wasm_bindgen(constructor)]
    pub fn new(map_json: &str, rules_json: &str) -> Result<PreparedWorld, JsError> {
        let map: MapDefinition = serde_json::from_str(map_json).map_err(js_error)?;
        let rules: Rules = serde_json::from_str(rules_json).map_err(js_error)?;
        Self::build(map, rules)
    }

    pub fn from_scenario(scenario_json: &str) -> Result<PreparedWorld, JsError> {
        let setup: ScenarioDefinition = serde_json::from_str(scenario_json).map_err(js_error)?;
        Self::build(setup.map, setup.rules)
    }

    /// Admit a self-contained download without consulting fixtures or a map generator.
    pub fn from_bundle(bundle_json: &str) -> Result<PreparedWorld, JsError> {
        let bundle = contract::bundle::ScenarioBundle::from_json(bundle_json).map_err(js_error)?;
        let scenario: ScenarioDefinition =
            serde_json::from_str(&bundle.scenario_json().map_err(js_error)?).map_err(js_error)?;
        Self::build(scenario.map, scenario.rules)
    }

    pub fn extents(&self) -> String {
        serde_json::to_string(&self.map.extents()).expect("finite map extents")
    }

    pub fn into_battle(self, scenario_json: &str, seed: f64) -> Result<BattleHandle, JsError> {
        let setup = self.scenario(scenario_json)?;
        Ok(BattleHandle {
            battle: Battle::from_prepared(&setup, seed as u64, self.prepared),
            publisher: Publisher::new(),
        })
    }

    pub fn into_replay(
        self,
        scenario_json: &str,
        replay_json: &str,
    ) -> Result<BattleHandle, JsError> {
        let setup = self.scenario(scenario_json)?;
        let replay: Replay = serde_json::from_str(replay_json).map_err(js_error)?;
        let battle = Battle::from_prepared_replay(&setup, &replay, self.prepared)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(BattleHandle {
            battle,
            publisher: Publisher::new(),
        })
    }
}

fn identity_digest<T: serde::Serialize>(value: &T) -> Result<u64, JsError> {
    Ok(sim::digest::of_str(
        &serde_json::to_string(value).map_err(js_error)?,
    ))
}

impl PreparedWorld {
    fn build(map: MapDefinition, rules: Rules) -> Result<Self, JsError> {
        let map_digest = identity_digest(&map)?;
        let rules_digest = identity_digest(&rules)?;
        let prepared = sim::encounter::PreparedMap::new(&map, &rules);
        Ok(Self {
            map,
            rules,
            prepared,
            map_digest,
            rules_digest,
        })
    }

    fn scenario(&self, scenario_json: &str) -> Result<ScenarioDefinition, JsError> {
        let setup: ScenarioDefinition = serde_json::from_str(scenario_json).map_err(js_error)?;
        if identity_digest(&setup.map)? != self.map_digest
            || identity_digest(&setup.rules)? != self.rules_digest
        {
            return Err(JsError::new("prepared map and rules do not match scenario"));
        }
        setup.validate_player_contract().map_err(js_error)?;
        Ok(setup)
    }
}

#[wasm_bindgen(inline_js = "export function report_panic(message) {
  setTimeout(() => {
    throw new Error(message);
  });
}")]
extern "C" {
    fn report_panic(message: &str);
}

/// A panic names itself before the trap: otherwise the page sees only the
/// trap, then "recursive use of an object" on every later call. Its message
/// is thrown, uncaught, on its own turn: the trap itself is caught by the
/// call that made it, but this reaches whichever page hosts the worker as
/// an error (the console, and the diagnostics report).
#[wasm_bindgen(start)]
pub fn report_panics() {
    std::panic::set_hook(Box::new(|info| {
        report_panic(&format!("simulation panic: {info}"))
    }));
}
