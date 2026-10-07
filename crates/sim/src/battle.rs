//! The one battle authority: commands in, fixed ticks, side observations out.
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use contract::catalog::Destroyed;
use contract::command::{
    BuildingPlacement, BuildingPreviewRequest, CommandAck, CommandEnvelope, Engagement,
    MovePlacement, MovePreviewRequest, Order, OrderError, RoutePolicy, TargetRef,
};
use contract::ids::{Side, Tick, UnitId};
use contract::map::{MoverClass, PropDefinition};
use contract::observation::{
    Blast, Corpse, EncounterStatus, FallenBody, GuidedMissile, KnownProp, MemberOrder, MoveState,
    ObservationFrame, OwnUnit, Posture, SegmentHit, SegmentRicochet, ServiceStatus, SoundCue,
    SquadArea, UnitSight, VisibilityField, VisibleSegment,
};
use contract::scenario::{
    EncounterRules, EventAction, Opponent, Rules, ScenarioDefinition, ScenarioEvent, ScriptedOrder,
    UnitCondition,
};
use serde::{Deserialize, Serialize};

use crate::arrangement;
use crate::cover;
use crate::damage::{self, DamageContext, HullResolver};
use crate::deployment;
use crate::digest::{self, Digest};
use crate::encounter::{Defender, Referee};
use crate::flight::{
    self, Body, BodyId, FlightEvent, Pose, ProjectileId, Projectiles, Shape, Struck,
};
use crate::garrison;
use crate::ground::{self, GroundLayer, KnownGround, Wear};
use crate::hearing;
use crate::knowledge::SideKnowledge;
use crate::math::{v2, v3, Obb2, Rotation, V2, V3};
use crate::movement::{self, MovementContext, SideGeometry};
use crate::navigation::RoadNet;
use crate::route_planner::RoutePlanner;
use crate::sensing::{self, Sighting};
use crate::sight;
use crate::structures::Structures;
use crate::supply;
use crate::units::{self, MoveOrder, Pursuit, Soldier, Unit, UnitOrder};
use crate::visibility::OcclusionGrid;
use crate::weapons::{self, Arsenal, FireContext, Support, Target, VEHICLE_BODY_BASE};
use crate::world::{PropId, WorldGeometry};
use contract::random::Rng;

/// Ticks between ground-visibility sweeps for each side (sides alternate).
/// Identification runs every [`sensing::SENSE_EVERY`] ticks per observer;
/// the fog display lags by at most this many ticks.
const FOG_INTERVAL_TICKS: u64 = 6;
/// Seed salt for each side's observation-uncertainty stream (contact placement),
/// kept apart from combat and policy randomness.
const OBSERVATION_STREAM: u64 = 0x6f62_7365_7276_6531;
/// Seed salt for combat randomness (spread, area aim points).
const COMBAT_STREAM: u64 = 0x636f_6d62_6174_2121;
/// Seed salt for blast fragment sampling, kept apart from aim randomness.
const DAMAGE_STREAM: u64 = 0x6461_6d61_6765_2121;
/// Seed salt for ricochet rolls and scatter, drawn during flight.
const RICOCHET_STREAM: u64 = 0x7269_636f_6368_6574;
/// An attack reaching its target's last reported place within this distance,
/// without regaining sight, is complete.
const PURSUIT_ARRIVAL_M: f64 = 5.0;
/// Enemy round flight is shown only over seen ground, sampled this finely.
const SEGMENT_SAMPLES: usize = 8;

/// Where a round in flight came from, for consequences and visibility.
#[derive(Clone, Copy, Debug)]
pub struct Round {
    pub weapon: usize,
    pub unit: UnitId,
    pub side: Side,
}

/// One round's flight this tick, for the animation feed: presentation data
/// rebuilt every tick from the flight events, never carried state.
#[derive(Clone, Debug)]
struct Flown {
    side: Side,
    from: V3,
    /// Where it glanced off hulls on the way, in order, with the outward
    /// normal there.
    ricochets: Vec<(V3, V3)>,
    to: V3,
    weapon: usize,
    /// The soldier who fired it; `None` for a vehicle's gun.
    shooter: Option<u32>,
    /// What it struck at `to`, and the outward normal there.
    hit: Option<(SegmentHit, V3)>,
}

/// Where everything stood at one moment of a tick: each unit's pose, and
/// each soldier's position in unit then member order.
struct Poses {
    units: Vec<Pose>,
    soldiers: Vec<V3>,
}

fn xyz(p: V3) -> [f64; 3] {
    [p.x, p.y, p.z]
}

impl Flown {
    /// The path's corners: the start, each ricochet, the end.
    fn corners(&self) -> impl Iterator<Item = V3> + '_ {
        std::iter::once(self.from)
            .chain(self.ricochets.iter().map(|&(p, _)| p))
            .chain(std::iter::once(self.to))
    }

    fn segment(&self, piece: Piece, own: bool, hit: Option<(SegmentHit, V3)>) -> VisibleSegment {
        VisibleSegment {
            path: piece.path.into_iter().map(xyz).collect(),
            ricochets: piece
                .ricochets
                .into_iter()
                .map(|(point, n)| SegmentRicochet {
                    point,
                    normal: xyz(n),
                })
                .collect(),
            own,
            kind: self.weapon,
            shooter_member: self.shooter,
            hit: hit.map_or(SegmentHit::None, |(h, _)| h),
            impact_normal: hit.map(|(_, n)| xyz(n)),
        }
    }

    /// The whole flight, for the side that fired it.
    fn whole(&self) -> VisibleSegment {
        let piece = Piece {
            path: self.corners().collect(),
            ricochets: (1..).zip(self.ricochets.iter().map(|&(_, n)| n)).collect(),
        };
        self.segment(piece, true, self.hit)
    }
}

/// A drawn stretch of a round's path: its points, and the ricochets among
/// them by index.
#[derive(Default)]
struct Piece {
    path: Vec<V3>,
    ricochets: Vec<(usize, V3)>,
}

/// Completed work brackets for native cost reports. The ordinary tick uses
/// the same path with a no-op callback; measurement never enters battle state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickPhase {
    Orders,
    Navigation,
    Movement,
    Flight,
    Sight,
    Fog,
    Learning,
    Weapons,
    Observation,
    Other,
}

/// A snapshot of the battle's size (see [`Battle::load`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Load {
    pub living_units: usize,
    pub living_soldiers: usize,
    pub corpses: usize,
    pub wrecks: usize,
    pub active_projectiles: usize,
    pub rounds_launched: u64,
    pub path_searches: u64,
    /// Units holding for a route, and the planning work the latest tick spent.
    pub routes_pending: usize,
    pub planning_work: u64,
    /// Cells both sides' planning grids have worked out again since the
    /// battle began, as the sides learned of bodies and cleared ground.
    pub grid_cells_relaid: u64,
    /// Bytes the ground layer holds.
    pub ground_bytes: usize,
    /// Bytes both sides' learned copies of it hold.
    pub known_ground_bytes: usize,
}

/// Everything needed to reproduce a battle in the same build: the setup
/// identity, the seed and every sequenced command from both sides.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Replay {
    pub engine_build: String,
    pub scenario_digest: String,
    pub config_digest: String,
    pub seed: u64,
    /// `(applied_tick, command)` in acceptance order.
    pub accepted: Vec<(Tick, CommandEnvelope)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReplayError {
    BuildMismatch,
    ScenarioMismatch,
    ConfigMismatch,
}

/// The shared native/Wasm engine fingerprint generated from simulation build inputs.
pub const ENGINE_BUILD_ID: &str = env!("SIM_ENGINE_BUILD_ID");

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::BuildMismatch => "replay was recorded by a different simulation build",
            Self::ScenarioMismatch => "replay does not match this scenario",
            Self::ConfigMismatch => "replay does not match these simulation rules",
        })
    }
}

pub struct Battle {
    skirmish: Option<crate::skirmish::Skirmish>,
    world: WorldGeometry,
    rules: Rules,
    seed: u64,
    tick: Tick,
    units: Vec<Unit>,
    /// Props authored with the map (ids below this) are known to every side.
    authored_props: PropId,
    sides: [SideGeometry; 2],
    /// The map's road graph, built once: public terrain, the same for both
    /// sides.
    roads: RoadNet,
    /// Every unit's route request in progress.
    planner: RoutePlanner,
    events: VecDeque<ScenarioEvent>,
    scripts: VecDeque<ScriptedOrder>,
    knowledge: [SideKnowledge; 2],
    occlusion: OcclusionGrid,
    fog: [VisibilityField; 2],
    /// Each side's sightings as of each observer's latest identification run
    /// (27 perf), in observer then target order.
    sightings: [Vec<Sighting>; 2],
    /// Units that fired during the current sound bucket.
    fired: BTreeSet<UnitId>,
    audible: [Vec<SoundCue>; 2],
    arsenal: Arsenal,
    /// Building health and the ruins collapses left.
    structures: Structures,
    /// Craters and wear on the ground.
    ground: GroundLayer,
    projectiles: Projectiles,
    rounds: BTreeMap<ProjectileId, Round>,
    /// The strongest suppression each round in flight has dealt each squad
    /// (by unit index): a round suppresses a squad once however many ticks
    /// it takes to pass it.
    suppressed: BTreeMap<(ProjectileId, usize), f64>,
    combat_rng: Rng,
    damage_rng: Rng,
    ricochet_rng: Rng,
    /// The last soldier id issued; replacements continue from it (L03).
    last_soldier: u32,

    /// This tick's flight events, in order.
    flight_events: Vec<FlightEvent>,
    /// This tick's flown stretch per round.
    segments: Vec<Flown>,
    /// This tick's bursts, by firing side.
    blasts: Vec<(Side, Blast)>,
    next_seq: [u64; 2],
    order_bucket: [(u64, Tick); 2],
    /// The observation-bound opponent and its memory (off while replaying).
    opponent: Option<(Opponent, Defender)>,
    skirmish_ai: Option<crate::skirmish_ai::SkirmishAi>,
    /// The fixture's completion referee and its latest verdict.
    referee: Option<(EncounterRules, Referee)>,
    encounter: Option<EncounterStatus>,
    pending: Vec<PreparedCommand>,
    accepted: Vec<(Tick, CommandEnvelope)>,
    /// Recorded commands still to apply when this battle is a replay.
    replaying: Option<VecDeque<(Tick, CommandEnvelope)>>,
    observations: [ObservationFrame; 2],
    scenario_digest: u64,
    config_digest: u64,
    /// Transient bodies (a row with a `lifetime_s`) and the tick each goes.
    expiries: BTreeMap<PropId, Tick>,
    /// Every toppling body that went down: where, which way and when. Each
    /// side publishes those it knocked down or has seen where they stood.
    fallen: BTreeMap<PropId, FallenBody>,
}

/// The parts of a round's flight over ground `fog` shows as seen, leg by leg
/// between its ricochets.
fn clip_to_seen(fog: &VisibilityField, round: &Flown) -> Vec<VisibleSegment> {
    let corners: Vec<V3> = round.corners().collect();
    let legs = corners.len() - 1;
    // Runs of seen samples, each ending at its last seen sample: a drawn
    // stretch never reaches over unseen ground. A run keeps each ricochet it
    // passes as a corner; along a leg only its latest sample (`tail`)
    // matters. An impact shows only if the struck point itself is seen.
    let mut out = Vec::new();
    let mut run: Option<(Piece, Option<V3>)> = None;
    for leg in 0..legs {
        let (a, b) = (corners[leg], corners[leg + 1]);
        for k in (leg.min(1))..=SEGMENT_SAMPLES {
            let p = a + (b - a) * (k as f64 / SEGMENT_SAMPLES as f64);
            let end = leg + 1 == legs && k == SEGMENT_SAMPLES;
            let seen = fog.visible(p.x, p.y);
            if seen {
                let (piece, tail) = run.get_or_insert_with(Default::default);
                let ricochet = (k == SEGMENT_SAMPLES && !end).then(|| round.ricochets[leg].1);
                if piece.path.is_empty() || ricochet.is_some() {
                    piece.path.push(p);
                    *tail = None;
                    if let Some(n) = ricochet {
                        piece.ricochets.push((piece.path.len() - 1, n));
                    }
                } else {
                    *tail = Some(p);
                }
                if !end {
                    continue;
                }
            }
            if let Some((mut piece, tail)) = run.take() {
                piece.path.extend(tail);
                if piece.path.len() >= 2 {
                    let hit = round.hit.filter(|_| seen && end);
                    out.push(round.segment(piece, false, hit));
                }
            }
        }
    }
    out
}

/// Whether any fog cell under a prop's footprint is seen: large remains
/// (a ruin) are learned when any of them is in view, not only their middle.
fn footprint_seen(field: &VisibilityField, prop: &crate::world::Prop) -> bool {
    let step = field.cell_m / 2.0;
    let rotation = Rotation::new(prop.yaw);
    let (nx, ny) = (
        (2.0 * prop.half.x / step).ceil().max(1.0) as usize,
        (2.0 * prop.half.y / step).ceil().max(1.0) as usize,
    );
    (0..=nx).any(|i| {
        (0..=ny).any(|j| {
            let local = v2(
                -prop.half.x + 2.0 * prop.half.x * i as f64 / nx as f64,
                -prop.half.y + 2.0 * prop.half.y * j as f64 / ny as f64,
            );
            let p = prop.center + rotation.apply(local);
            field.visible(p.x, p.y)
        })
    })
}

/// Authored starting wear: vehicle health, fallen soldiers (their records lie
/// where they stand in the squad's starting arrangement) and spent rounds.
fn wear(unit: &mut Unit, c: &UnitCondition, arsenal: &Arsenal, rules: &Rules) {
    if let (Some(hp), Some(_)) = (c.hp, unit.hull) {
        unit.hp = hp.clamp(1.0, unit.max_hp(rules));
    }
    if let Some(hp) = c.soldier_hp {
        for soldier in &mut unit.members {
            soldier.hp = hp.max(1.0);
        }
    }
    let n = unit.members.len();
    for k in n.saturating_sub(c.casualties as usize)..n {
        let at = unit.members[k].position;
        unit.members[k].fall(at, unit.yaw, None);
    }
    if !unit.members.is_empty() {
        unit.suppression = c.suppression.clamp(0.0, 1.0);
    }
    let specs = arsenal.specs(unit.kind);
    for mount in &mut unit.mounts {
        for (k, &row) in specs[mount.spec].kinds.iter().enumerate() {
            if let (Some(spent), Some(n)) = (
                c.spent.get(&arsenal.weapons[row].id),
                mount.ammo[k].as_mut(),
            ) {
                *n = n.saturating_sub(*spent);
            }
        }
        for cycle in &mut mount.cycles {
            if cycle.loaded.is_some_and(|k| mount.ammo[k] == Some(0)) {
                cycle.loaded = None;
            }
        }
    }
}

/// Physical construction is shared by authored placements and reinforcements.
fn spawn_unit(
    i: usize,
    u: &contract::scenario::UnitSetup,
    world: &WorldGeometry,
    rules: &Rules,
    arsenal: &Arsenal,
    soldier_ids: &mut u32,
    seed: u64,
) -> Unit {
    let kind = rules
        .catalog
        .index(&u.kind)
        .unwrap_or_else(|| panic!("unit {i} names unknown unit type {:?}", u.kind));
    let t = rules.catalog.get(kind);
    let xy = v2(u.position[0], u.position[1]);
    // A squad starts spread out like any squad that has just
    // arrived: a seeded arrangement around its position.
    let slots = t.slots().unwrap_or_default();
    let count = slots.len();
    let solid = |p: &crate::world::Prop| p.blocks(MoverClass::Infantry);
    let mut draws = arrangement::rng(seed, i as u32, 0);
    let members: Vec<Soldier> = arrangement::squad_spots(
        world,
        xy,
        count,
        &rules.infantry_movement,
        rules.physics.soldier_radius_m,
        &solid,
        &mut draws,
    )
    .into_iter()
    .enumerate()
    .map(|(slot, p)| {
        *soldier_ids += 1;
        let z = world.surface_at(p.x, p.y).map_or(0.0, |s| s.z);
        let hp = rules.catalog.soldier(&slots[slot]).hp;
        Soldier::new(*soldier_ids, slot, p.with_z(z), hp)
    })
    .collect();
    let mounts = arsenal.mounts_for(kind, u.yaw, &members);
    Unit {
        id: UnitId(i as u32),
        side: u.side,
        kind,
        position: xy.with_z(world.surface_at(xy.x, xy.y).map_or(0.0, |s| s.z)),
        yaw: u.yaw,
        mobility: units::mobility(t, rules),
        hull: t.hull().map(|h| {
            let [x, y, z] = h.half_extents_m;
            crate::math::v3(x, y, z)
        }),
        members,
        orders: VecDeque::new(),
        route: None,
        route_from: xy,
        state: MoveState::Idle,
        blocker: None,
        planned_revision: 0,
        progress: (f64::INFINITY, 0),
        stalls: (f64::INFINITY, 0),
        pursuit: None,
        planned_goal: None,
        engagement: u.engagement.unwrap_or(Engagement::FireAtWill),
        mounts,
        attackers: BTreeSet::new(),
        reach: Default::default(),
        hp: t.hull().map_or(0.0, |h| h.hp),
        retired: false,
        withdrawing: false,
        suppression: 0.0,
        suppressed_at: 0,
        deployment: deployment::initial(t, rules),
        garrison: None,
        stock: t.capabilities.supply.map(|s| u.stock.unwrap_or(s.stock)),
        protection: t
            .capabilities
            .active_protection
            .map(crate::protection::State::new),
        progress_service: Default::default(),
        service: ServiceStatus::OutOfRange,
        sight_forward: u.yaw,
        cover: Default::default(),
        // A squad placed by the scenario holds round where it was put.
        anchor: t.hull().is_none().then_some(crate::cover::Anchor {
            at: xy,
            halt: false,
        }),
        manoeuvre: None,
        reversing: false,
        drive_speed_mps: 0.0,
        turn_to: None,
    }
}

fn scenario_digest(setup: &ScenarioDefinition) -> u64 {
    let map = serde_json::to_string(&setup.map).expect("map serializes");
    let units = serde_json::to_string(&setup.units).expect("units serialize");
    let events = serde_json::to_string(&setup.events).expect("events serialize");
    let scripts = serde_json::to_string(&setup.scripts).expect("scripts serialize");
    // The opponent's policy and the encounter's rules change what a replay's
    // commands mean, so a replay is pinned to them too.
    let opponent = serde_json::to_string(&setup.opponent).expect("opponent serializes");
    let encounter = serde_json::to_string(&setup.encounter).expect("encounter serializes");
    let skirmish = setup
        .skirmish
        .as_ref()
        .map(|s| serde_json::to_string(s).expect("skirmish serializes"))
        .unwrap_or_default();
    digest::of_str(&(map + &units + &events + &scripts + &opponent + &encounter + &skirmish))
}

fn config_digest(setup: &ScenarioDefinition) -> u64 {
    digest::of_str(&serde_json::to_string(&setup.rules).expect("rules serialize"))
}

impl Battle {
    fn dispatch_reinforcements(&mut self) {
        use crate::encounter::legality::{apart, footprint, stands_on};
        let Some(skirmish) = &mut self.skirmish else {
            return;
        };
        if skirmish.phase != contract::skirmish::Phase::Active {
            return;
        }
        for side in [Side::Blue, Side::Red] {
            let index = side.index();
            if self.tick < skirmish.next_dispatch[index] {
                continue;
            }
            let Some(reservation) = skirmish.reservations[index].front_mut() else {
                continue;
            };
            let entry = &skirmish.setup.sites.entries[index];
            let at = v2(entry.center[0], entry.center[1]);
            let t = self.rules.catalog.get(reservation.data.kind);
            let mobility = units::mobility(t, &self.rules);
            let candidate = footprint(t, &self.rules, at, entry.yaw);
            let clear = stands_on(
                &self.world,
                self.sides[index].grid(&self.world, self.authored_props),
                &self.rules,
                t,
                &mobility,
                at,
                entry.yaw,
            )
            .is_ok()
                && self.units.iter().filter(|u| u.alive()).all(|u| {
                    apart(
                        &candidate,
                        &footprint(
                            self.rules.catalog.get(u.kind),
                            &self.rules,
                            u.position.xy(),
                            u.yaw,
                        ),
                        0.0,
                    )
                });
            reservation.data.blocked = !clear;
            if !clear {
                continue;
            }
            let reservation = skirmish.reservations[index].pop_front().unwrap();
            let setup = contract::scenario::UnitSetup {
                side,
                kind: self.rules.catalog.id(reservation.data.kind).to_string(),
                position: entry.center,
                yaw: entry.yaw,
                engagement: None,
                condition: None,
                stock: None,
            };
            let mut unit = spawn_unit(
                self.units.len(),
                &setup,
                &self.world,
                &self.rules,
                &self.arsenal,
                &mut self.last_soldier,
                self.seed,
            );
            unit.settle();
            unit.orders.push_back(UnitOrder::Move(MoveOrder {
                destination: v2(
                    reservation.data.destination[0],
                    reservation.data.destination[1],
                ),
                policy: RoutePolicy::Fastest,
                gesture: 0,
                direction: contract::command::MoveDirection::Forward,
                facing: None,
                short: None,
            }));
            skirmish.entered.insert(
                unit.id,
                crate::skirmish::EnteredUnit {
                    price: reservation.price,
                    entered_tick: self.tick,
                },
            );
            skirmish.next_dispatch[index] = self.tick
                + u64::from(skirmish.setup.rules.dispatch_interval_s)
                    * u64::from(self.rules.tick_hz);
            self.units.push(unit);
        }
    }

    fn retire_withdrawals(&mut self) {
        let Some(skirmish) = &mut self.skirmish else {
            return;
        };
        for unit in &mut self.units {
            if !unit.alive() || !unit.withdrawing || unit.movement_goal().is_some() {
                continue;
            }
            let entry = skirmish
                .setup
                .sites
                .entries
                .iter()
                .find(|entry| entry.side == unit.side)
                .unwrap();
            // Retirement follows completed ordinary movement at the admitted base.
            let half = unit.hull.map_or(2.0, |half| half.x.max(half.y));
            if (unit.position.xy() - v2(entry.center[0], entry.center[1])).length() > half * 2.0 {
                continue;
            }
            let entered = &skirmish.entered[&unit.id];
            let (health, ammo) = crate::withdrawal::condition(unit, &self.rules, &self.arsenal);
            let age = self.tick.saturating_sub(entered.entered_tick) as f64
                / f64::from(self.rules.tick_hz);
            let credits = (f64::from(entered.price)
                * crate::withdrawal::fraction(health, ammo, age)
                * crate::skirmish::CREDIT_SCALE as f64)
                .round() as u64;
            skirmish.wallets[unit.side.index()] += credits;
            unit.retired = true;
            unit.withdrawing = false;
            unit.orders.clear();
            unit.route = None;
        }
    }

    pub fn new(setup: &ScenarioDefinition, seed: u64) -> Self {
        Self::from_prepared(
            setup,
            seed,
            crate::encounter::PreparedMap::new(&setup.map, &setup.rules),
        )
    }

    /// Consume the physical world and navigation already used to place this encounter.
    /// `prepared` must have been built from this scenario's map and rules.
    pub fn from_prepared(
        setup: &ScenarioDefinition,
        seed: u64,
        prepared: crate::encounter::PreparedMap,
    ) -> Self {
        let rules = setup.rules.clone();
        assert!(
            setup.skirmish.is_none()
                || (setup.units.is_empty()
                    && setup.events.is_empty()
                    && setup.scripts.is_empty()
                    && setup.opponent.is_none()
                    && setup.encounter.is_none()),
            "skirmish begins empty and owns its controller/referee"
        );
        assert!(
            rules.physics.vehicle_aim_height_fraction > 0.0
                && rules.physics.vehicle_aim_height_fraction <= 1.0,
            "physics.vehicle_aim_height_fraction must be in (0, 1]"
        );
        units::validate_drive(&rules);
        sensing::validate(&rules.sensors);
        damage::validate(&rules);
        ground::validate(&rules);
        crate::cover::validate(&rules);
        flight::validate_guided(&rules.guided);
        let crate::encounter::PreparedMap {
            world,
            roads,
            base: grid,
            ..
        } = prepared;
        let arsenal = Arsenal::new(&rules);
        supply::validate(&rules).expect("fixture supply rules are valid");
        for e in &setup.events {
            if let EventAction::AddProp(def) = &e.action {
                assert!(
                    rules.catalog.props().index(&def.kind).is_some(),
                    "an added prop names an unknown prop type {:?}",
                    def.kind
                );
            }
            if let EventAction::Burst { weapon, .. } = &e.action {
                assert!(
                    arsenal.weapons.iter().any(|w| &w.id == weapon),
                    "a burst names an unknown weapon row {weapon}"
                );
            }
        }
        let mut soldier_ids = 0u32;
        let mut units = setup
            .units
            .iter()
            .enumerate()
            .map(|(i, u)| spawn_unit(i, u, &world, &rules, &arsenal, &mut soldier_ids, seed))
            .collect::<Vec<Unit>>();
        for (unit, setup) in units.iter_mut().zip(&setup.units) {
            if let Some(c) = &setup.condition {
                wear(unit, c, &arsenal, &rules);
            }
            unit.settle();
        }
        let mut events: Vec<ScenarioEvent> = setup.events.clone();
        events.sort_by_key(|e| e.tick);
        let mut scripts: Vec<ScriptedOrder> = setup.scripts.clone();
        scripts.sort_by_key(|o| o.tick);
        let occlusion = OcclusionGrid::new(&world, setup.map.fog_cell_m);
        let structures = Structures::default();
        let ground = GroundLayer::new(world.width(), world.depth(), &rules.ground);
        let knowledge = [
            SideKnowledge::new(seed ^ OBSERVATION_STREAM, &ground),
            SideKnowledge::new(seed ^ OBSERVATION_STREAM ^ 1, &ground),
        ];
        let mut battle = Battle {
            skirmish: setup.skirmish.clone().map(crate::skirmish::Skirmish::new),
            authored_props: world.props().count() as PropId,
            world,
            rules,
            seed,
            tick: 0,
            units,
            sides: [
                SideGeometry::new(std::sync::Arc::clone(&grid)),
                SideGeometry::new(grid),
            ],
            roads,
            planner: RoutePlanner::default(),
            events: events.into(),
            scripts: scripts.into(),
            knowledge,
            sightings: Default::default(),
            fired: BTreeSet::new(),
            audible: Default::default(),
            projectiles: Projectiles::new(arsenal.config.clone()),
            arsenal,
            structures,
            ground,
            rounds: BTreeMap::new(),
            suppressed: BTreeMap::new(),
            combat_rng: Rng::new(seed ^ COMBAT_STREAM),
            damage_rng: Rng::new(seed ^ DAMAGE_STREAM),
            ricochet_rng: Rng::new(seed ^ RICOCHET_STREAM),
            last_soldier: soldier_ids,

            flight_events: Vec::new(),
            segments: Vec::new(),
            blasts: Vec::new(),
            fog: [occlusion.field(), occlusion.field()],
            occlusion,
            next_seq: [1, 1],
            order_bucket: [(
                u64::from(setup.rules.commands.orders_per_s) * u64::from(setup.rules.tick_hz),
                0,
            ); 2],
            opponent: setup.opponent.clone().map(|o| (o, Defender::default())),
            skirmish_ai: setup
                .skirmish
                .as_ref()
                .and_then(|s| s.ai_side)
                .map(|_| crate::skirmish_ai::SkirmishAi::default()),
            referee: setup.encounter.clone().map(|e| (e, Referee::default())),
            encounter: None,
            pending: Vec::new(),
            accepted: Vec::new(),
            replaying: None,
            observations: Default::default(),
            scenario_digest: scenario_digest(setup),
            config_digest: config_digest(setup),
            expiries: BTreeMap::new(),
            fallen: BTreeMap::new(),
        };
        let authored: Vec<PropId> = battle.world.props().map(|p| p.id).collect();
        for id in authored {
            battle.schedule_expiry(id);
        }
        sight::snapshot(&mut battle.units, &battle.arsenal);
        for side in Side::ALL {
            battle.sense(side, true);
            battle.sweep_fog(side);
        }
        battle.observe_all();
        battle
    }

    /// A battle that re-applies `replay`'s commands at their recorded ticks.
    /// Player input is refused for its whole life.
    pub fn from_replay(setup: &ScenarioDefinition, replay: &Replay) -> Result<Self, ReplayError> {
        Self::load_replay(setup, replay, || Self::new(setup, replay.seed))
    }

    /// Restore a replay while retaining the prepared map's physical allocations.
    pub fn from_prepared_replay(
        setup: &ScenarioDefinition,
        replay: &Replay,
        prepared: crate::encounter::PreparedMap,
    ) -> Result<Self, ReplayError> {
        Self::load_replay(setup, replay, || {
            Self::from_prepared(setup, replay.seed, prepared)
        })
    }

    fn load_replay(
        setup: &ScenarioDefinition,
        replay: &Replay,
        build: impl FnOnce() -> Self,
    ) -> Result<Self, ReplayError> {
        if replay.engine_build != ENGINE_BUILD_ID {
            return Err(ReplayError::BuildMismatch);
        }
        if format!("{:016x}", scenario_digest(setup)) != replay.scenario_digest {
            return Err(ReplayError::ScenarioMismatch);
        }
        if format!("{:016x}", config_digest(setup)) != replay.config_digest {
            return Err(ReplayError::ConfigMismatch);
        }
        let mut battle = build();
        battle.replaying = Some(replay.accepted.iter().cloned().collect());
        Ok(battle)
    }

    pub fn tick(&self) -> Tick {
        self.tick
    }

    pub fn rules(&self) -> &Rules {
        &self.rules
    }

    pub fn world(&self) -> &WorldGeometry {
        &self.world
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// This tick's flight events, in order; damage has already applied them.
    pub fn flight_events(&self) -> &[FlightEvent] {
        &self.flight_events
    }

    /// Rounds currently in flight, with where each came from.
    pub fn rounds(&self) -> impl Iterator<Item = (&flight::Projectile, &Round)> {
        self.projectiles
            .active()
            .iter()
            .filter_map(|p| self.rounds.get(&p.id).map(|r| (p, r)))
    }

    /// What the battle is carrying now, read from the owning stores (the
    /// endurance report's counters; no second bookkeeping).
    pub fn load(&self) -> Load {
        let living = self.units.iter().filter(|u| u.alive());
        Load {
            living_units: living.clone().count(),
            living_soldiers: living
                .map(|u| u.members.iter().filter(|s| s.alive()).count())
                .sum(),
            corpses: self
                .units
                .iter()
                .flat_map(|u| &u.members)
                .filter(|s| !s.alive())
                .count(),
            wrecks: self
                .world
                .props()
                .filter(|p| {
                    let catalog = &self.rules.catalog;
                    let kind = self.world.types().id(p.kind);
                    catalog
                        .indices()
                        .any(|t| catalog.get(t).hull().is_some_and(|h| h.wreck == kind))
                })
                .count(),
            active_projectiles: self.projectiles.active().len(),
            rounds_launched: self.projectiles.launched(),
            path_searches: Side::ALL.into_iter().map(|s| self.route_searches(s)).sum(),
            routes_pending: self.planner.waiting(),
            planning_work: self.planner.spent(),
            grid_cells_relaid: self.sides.iter().map(|s| s.cells_relaid()).sum(),
            ground_bytes: self.ground.bytes(),
            known_ground_bytes: self.knowledge.iter().map(|k| k.ground().bytes()).sum(),
        }
    }

    /// A unit as the authority holds it, for native tests and reports.
    pub fn unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.get(id.0 as usize)
    }

    /// The referee's latest verdict (the one both sides' observations carry);
    /// `None` without an encounter or before the first step.
    pub fn encounter(&self) -> Option<EncounterStatus> {
        self.encounter
    }

    pub fn arsenal(&self) -> &Arsenal {
        &self.arsenal
    }

    /// Every prop's integrity and remains, for native tests and reports.
    pub fn structures(&self) -> &Structures {
        &self.structures
    }

    /// Craters and wear on the ground, for native tests and reports. Players
    /// only ever receive a side's [`Battle::known_ground`].
    pub fn ground(&self) -> &GroundLayer {
        &self.ground
    }

    /// The ground as `side` last saw it: what its publications deliver.
    pub fn known_ground(&self, side: Side) -> &KnownGround {
        self.knowledge[side.index()].ground()
    }

    /// `side`'s navigation revision: it rises only when the side learns an
    /// obstacle that changes its plans.
    pub fn navigation_revision(&self, side: Side) -> u64 {
        self.sides[side.index()].revision
    }

    /// Route searches run so far for `side`.
    pub fn route_searches(&self, side: Side) -> u64 {
        self.sides[side.index()].searches
    }

    /// Validate and schedule a command for the next step. Every command whose
    /// sequence number is next for its side is recorded for replay, including
    /// ones rejected for their content, so a replay reproduces the same acks.
    pub fn accept(&mut self, command: CommandEnvelope) -> CommandAck {
        let applied_tick = self.tick + 1;
        let ack = |error, placement, building| CommandAck {
            seq: command.seq,
            applied_tick,
            error,
            placement,
            building,
        };
        if self.replaying.is_some() {
            return ack(Some(OrderError::ReplayInProgress), None, None);
        }
        self.admit(command.clone(), applied_tick).map_or_else(
            |e| ack(Some(e), None, None),
            |(placement, building)| {
                let error = placement
                    .as_ref()
                    .filter(|p| p.destinations.iter().all(|d| !d.placed))
                    .filter(|_| building.as_ref().is_none_or(|b| b.entrant.is_none()))
                    .map(|_| OrderError::NoValidDestination);
                ack(error, placement, building)
            },
        )
    }

    fn spend_order(&mut self, side: usize, tick: Tick) -> bool {
        let per_s = u64::from(self.rules.commands.orders_per_s);
        let hz = u64::from(self.rules.tick_hz);
        let (left, at) = &mut self.order_bucket[side];
        *left = (*left + tick.saturating_sub(*at) * per_s).min(per_s * hz);
        *at = (*at).max(tick);
        if *left < hz {
            return false;
        }
        *left -= hz;
        true
    }

    fn admit(
        &mut self,
        command: CommandEnvelope,
        applied_tick: Tick,
    ) -> Result<(Option<MovePlacement>, Option<BuildingPlacement>), OrderError> {
        let side = command.side.index();
        if command.seq != self.next_seq[side] {
            return Err(OrderError::OutOfSequence {
                expected: self.next_seq[side],
            });
        }
        self.next_seq[side] += 1;
        self.accepted.push((applied_tick, command.clone()));
        if !self.spend_order(side, applied_tick) {
            return Err(OrderError::RateLimited);
        }
        let prepared = self.prepare(command, applied_tick.saturating_sub(1))?;
        let placement = prepared.placement.clone();
        let building = prepared.building.as_ref().map(|p| p.placement.clone());
        self.pending.push(prepared);
        Ok((placement, building))
    }

    /// A side-scoped target reference as the sim's own target, if the side
    /// holds it right now.
    fn resolve_target(&self, side: Side, target: &TargetRef) -> Option<Target> {
        let knowledge = &self.knowledge[side.index()];
        match *target {
            TargetRef::Identified { id } => knowledge.unit_for(id).map(Target::Unit),
            TargetRef::Contact { id } => knowledge.contact(id).map(|c| Target::Contact(c.id)),
            // The aim point sits on the public terrain, whatever height the client sent.
            TargetRef::Ground { point } => self
                .world
                .height_at(point[0], point[1])
                .map(|z| Target::Ground(v3(point[0], point[1], z))),
        }
    }

    pub fn finished(&self) -> bool {
        self.skirmish
            .as_ref()
            .is_some_and(|s| s.phase == contract::skirmish::Phase::Finished)
    }

    fn validate(&self, command: &CommandEnvelope) -> Result<(), OrderError> {
        if self.finished() {
            return Err(OrderError::MatchFinished);
        }
        let units = match &command.order {
            Order::Refund { units } => {
                let Some(skirmish) = &self.skirmish else {
                    return Err(OrderError::NotSkirmish);
                };
                self.validate_units(command.side, units)?;
                if units.iter().any(|id| !skirmish.entered.contains_key(id)) {
                    return Err(OrderError::NotSkirmish);
                }
                units
            }
            Order::Ready | Order::ConfirmPurchase { .. } | Order::CancelPending { .. } => {
                return self
                    .skirmish
                    .as_ref()
                    .map(|_| ())
                    .ok_or(OrderError::NotSkirmish)
            }
            Order::Move { units, goal, .. } | Order::AttackMove { units, goal, .. } => {
                if self.world.height_at(goal[0], goal[1]).is_none() {
                    return Err(OrderError::OutOfBounds);
                }
                units
            }
            Order::Attack { units, target } => {
                if self.resolve_target(command.side, target).is_none() {
                    return Err(OrderError::UnknownTarget);
                }
                units
            }
            Order::Garrison { units, building } => {
                self.validate_units(command.side, units)?;
                // A squad ordered in earlier this tick holds no order yet.
                let claimed = self.pending.iter().any(|c| {
                    c.command.side == command.side
                        && (matches!(c.command.order, Order::Garrison { building: b, .. } if self.world.remembered_structure_owner(b) == self.world.remembered_structure_owner(*building))
                            || c.building.as_ref().is_some_and(|b| b.placement.entrant.is_some() && b.placement.building == self.world.remembered_structure_owner(*building)))
                });
                return garrison::validate(
                    &self.world,
                    &self.units,
                    command.side,
                    units,
                    self.sides[command.side.index()]
                        .prop(&self.world, self.authored_props, *building)
                        .filter(|p| self.world.building_of(p.id).is_some())
                        .map(|mut p| {
                            p.id = self.world.remembered_structure_owner(p.id);
                            p
                        }),
                    claimed,
                    &self.rules,
                );
            }
            Order::Stop { units }
            | Order::OccupyBuilding { units, .. }
            | Order::SetEngagement { units, .. }
            | Order::SetDeployment { units, .. }
            | Order::ExitBuilding { units } => units,
            // Gesture tokens are scoped by side at application; nothing else to check.
            Order::UpgradeMove { .. } => return Ok(()),
        };
        self.validate_units(command.side, units)
    }

    /// Every unit named exists, is the side's own and is alive.
    fn validate_units(&self, side: Side, units: &[UnitId]) -> Result<(), OrderError> {
        if units.is_empty() {
            return Err(OrderError::NoUnits);
        }
        for &unit in units {
            match self.units.get(unit.0 as usize) {
                None => return Err(OrderError::UnknownUnit { unit }),
                Some(u) if u.side != side => return Err(OrderError::NotOwnUnit { unit }),
                Some(u) if !u.alive() => return Err(OrderError::Destroyed { unit }),
                Some(_) => {}
            }
        }
        Ok(())
    }

    /// Advance one fixed tick: authored events, scheduled commands, movement, observation.
    pub fn step(&mut self) -> Tick {
        self.step_profiled(|_| {})
    }

    /// Advance the production tick, notifying the report after each work
    /// bracket. Repeated phases (movement and each side's fog) add together.
    pub fn step_profiled(&mut self, mut completed: impl FnMut(TickPhase)) -> Tick {
        if self.finished() {
            return self.tick;
        }
        self.skirmish_ai_turn();
        self.tick += 1;
        for side in Side::ALL {
            for contact in self.knowledge[side.index()].expire_contacts(self.tick) {
                for unit in self.units.iter_mut().filter(|u| u.side == side) {
                    unit.orders.retain(|order| {
                        !matches!(order, UnitOrder::Attack { target, .. } if *target == Target::Contact(contact))
                    });
                    for mount in &mut unit.mounts {
                        if mount
                            .lock
                            .as_ref()
                            .is_some_and(|l| l.target == Target::Contact(contact))
                        {
                            mount.lock = None;
                        }
                    }
                }
            }
        }
        self.prune_attackers();
        if let Some(recorded) = self.replaying.as_mut() {
            let mut due = Vec::new();
            while recorded.front().is_some_and(|(t, _)| *t == self.tick) {
                due.push(recorded.pop_front().unwrap().1);
            }
            for command in due {
                let tick = self.tick;
                // Recorded content errors reproduce as the same no-op.
                let _ = self.admit(command, tick);
            }
        }
        self.expire_props();
        while self.events.front().is_some_and(|e| e.tick <= self.tick) {
            let event = self.events.pop_front().unwrap();
            match event.action {
                EventAction::AddProp(prop) => {
                    self.add_prop(&prop);
                }
                EventAction::Fire { unit } => self.record_fire(unit, 0),
                EventAction::Burst { point, weapon } => self.burst_event(point, &weapon),
            }
        }
        // Fixture scripts are authored setup, identical live and in replay.
        while self.scripts.front().is_some_and(|o| o.tick <= self.tick) {
            let mut script = self.scripts.pop_front().unwrap();
            // A fixed script addresses a formation's survivors. Keep invalid
            // ids for ordinary validation; only its own fallen actors leave.
            match &mut script.order {
                Order::Refund { units }
                | Order::Move { units, .. }
                | Order::Stop { units }
                | Order::Attack { units, .. }
                | Order::AttackMove { units, .. }
                | Order::SetEngagement { units, .. }
                | Order::SetDeployment { units, .. }
                | Order::Garrison { units, .. }
                | Order::OccupyBuilding { units, .. }
                | Order::ExitBuilding { units } => units.retain(|id| {
                    self.units
                        .get(id.0 as usize)
                        .is_none_or(|u| u.side != script.side || u.alive())
                }),
                Order::UpgradeMove { .. }
                | Order::Ready
                | Order::ConfirmPurchase { .. }
                | Order::CancelPending { .. } => {}
            }
            let command = CommandEnvelope {
                side: script.side,
                seq: 0,
                order: script.order,
                queued: script.queued,
            };
            if let Ok(prepared) = self.prepare(command, self.tick.saturating_sub(1)) {
                self.pending.push(prepared);
            }
        }
        for command in std::mem::take(&mut self.pending) {
            self.apply(command);
        }
        if let Some(skirmish) = &mut self.skirmish {
            skirmish.advance(self.tick, self.rules.tick_hz);
        }
        self.dispatch_reinforcements();
        completed(TickPhase::Orders);
        garrison::advance(
            &self.world,
            &self.sides,
            self.authored_props,
            &mut self.units,
            &self.rules,
            self.seed,
            self.tick,
        );
        self.update_pursuit();
        damage::recover(&mut self.units, &self.rules, self.tick);
        // Progress moves before movement, so the gate opens on the tick packing completes.
        deployment::advance_all(&mut self.units);
        let before = self.poses();
        let treads = self.treads();
        // Every prop the world changed since last tick, for each side's
        // planning grid to look at again.
        let touched = self.world.take_touched();
        for side in &mut self.sides {
            side.touch(&touched);
        }
        let ctx = MovementContext {
            world: &self.world,
            roads: &self.roads,
            ground: &self.ground,
            ground_rules: &self.rules.ground,
            authored: self.authored_props,
            tick: self.tick,
            tick_hz: self.rules.tick_hz,
            infantry: &self.rules.infantry_movement,
            soldier_radius_m: self.rules.physics.soldier_radius_m,
            seed: self.seed,
            rules: &self.rules,
            knowledge: [&self.knowledge[0], &self.knowledge[1]],
            arsenal: &self.arsenal,
        };
        completed(TickPhase::Other);
        let shoves = movement::advance(
            &ctx,
            &mut self.units,
            &mut self.sides,
            &mut self.planner,
            &mut completed,
        );
        self.shove_props(shoves);
        self.clear_lanes(&before);
        for ((from, channel), (to, _)) in treads.into_iter().zip(self.treads()) {
            self.ground.wear(from, to, channel, &self.rules.ground);
        }
        let after = self.poses();
        let moved: Vec<bool> = before
            .units
            .iter()
            .zip(&after.units)
            .map(|(a, b)| (a.base - b.base).length() > 1e-9)
            .collect();
        completed(TickPhase::Movement);
        self.guide(&moved);
        self.fly(&before, &after);
        // Where each unit looks, before this tick's fire turns any turret.
        completed(TickPhase::Flight);
        sight::snapshot(&mut self.units, &self.arsenal);
        for side in Side::ALL {
            self.sense(side, false);
            completed(TickPhase::Sight);
            if self.tick % FOG_INTERVAL_TICKS == side.index() as u64 * FOG_INTERVAL_TICKS / 2 {
                self.sweep_fog_profiled(side, &mut completed);
                completed(TickPhase::Learning);
            }
        }
        self.prune_attackers();
        let fired = self.fire(&moved);
        supply::service(
            &self.world,
            &mut self.units,
            &self.arsenal,
            &self.rules,
            &moved,
            &fired,
            &mut self.last_soldier,
        );
        self.retire_withdrawals();
        if let Some(skirmish) = &mut self.skirmish {
            if skirmish.phase == contract::skirmish::Phase::Active {
                skirmish.objectives.advance(
                    &skirmish.setup.sites.objectives,
                    &self.units,
                    &self.rules,
                );
                if skirmish.objectives.result.is_some() {
                    skirmish.phase = contract::skirmish::Phase::Finished;
                }
            }
        }
        completed(TickPhase::Weapons);
        let bucket = (self.rules.sensors.sound_bucket_s * self.rules.tick_hz as f64).round() as u64;
        for side in Side::ALL {
            self.audible[side.index()] = if self.tick.is_multiple_of(bucket) {
                hearing::hear(
                    side,
                    self.tick,
                    &self.units,
                    &self.knowledge[side.index()],
                    &self.fired,
                    &self.rules,
                )
            } else {
                Vec::new()
            };
        }
        if self.tick.is_multiple_of(bucket) {
            self.fired.clear();
        }
        if let Some((rules, referee)) = self.referee.as_mut() {
            self.encounter = Some(referee.judge(
                rules,
                &self.rules.catalog,
                &self.units,
                self.tick,
                self.rules.tick_hz,
            ));
        }
        // A squad stands where its living soldiers stand, the fallen and the
        // joined included.
        for unit in &mut self.units {
            unit.settle();
        }
        self.ground.seal();
        completed(TickPhase::Other);
        self.observe_all();
        completed(TickPhase::Observation);
        self.opponent_turn();
        completed(TickPhase::Other);
        self.tick
    }

    /// Add a prop after setup, scheduling its end if its row is transient.
    fn add_prop(&mut self, def: &PropDefinition) -> PropId {
        let id = self.world.add_prop(def);
        self.schedule_expiry(id);
        id
    }

    /// A transient body (its row has a `lifetime_s`) goes that long after
    /// it appears (Q28).
    fn schedule_expiry(&mut self, id: PropId) {
        let lifetime = self.world.prop(id).and_then(|p| p.body.lifetime_s);
        if let Some(s) = lifetime {
            let ticks = (s * self.rules.tick_hz as f64).round().max(1.0) as Tick;
            self.expiries.insert(id, self.tick + ticks);
        }
    }

    /// Transient bodies whose time is up go; every side sees them go, and
    /// both revisions bump.
    fn expire_props(&mut self) {
        let due: Vec<PropId> = self
            .expiries
            .iter()
            .filter(|(_, &t)| t <= self.tick)
            .map(|(&id, _)| id)
            .collect();
        for id in due {
            self.expiries.remove(&id);
            let Some(prop) = self.world.remove_prop(id) else {
                continue;
            };
            for side in &mut self.sides {
                side.forget(&prop);
            }
        }
    }

    /// Move the bodies vehicles shoved this tick (Q2). A side that has not
    /// seen an authored body move keeps it where it stood (L1); the soldiers
    /// a body now covers step out of it.
    fn shove_props(&mut self, shoves: Vec<movement::Shove>) {
        for s in shoves {
            let Some(prop) = self.world.prop(s.prop).cloned() else {
                continue;
            };
            if prop.body.topples {
                self.knock_down(prop, s.by, s.heading);
                continue;
            }
            for side in &mut self.sides {
                side.before_move(&prop, self.authored_props);
            }
            self.world.move_prop(s.prop, s.center, s.yaw, self.tick);
            if let Some(moved) = self.world.prop(s.prop).map(|p| p.footprint()) {
                let r = self.rules.physics.soldier_radius_m;
                movement::clear_of(&self.world, &mut self.units, &moved, r);
            }
        }
    }

    /// A vehicle of side `by`, driving along `heading`, knocked a tree down
    /// (Q16): its body goes and it falls the way the vehicle drove. Its side
    /// replans without it and knows the fall at once (contact). Every other
    /// side keeps it standing until it sees the ground where it stood (L1).
    fn knock_down(&mut self, prop: crate::world::Prop, by: Side, heading: V2) {
        self.world.knock_down(prop.id);
        self.keep_standing(&prop, Some(by));
        self.fell(&prop, heading);
        self.knowledge[by.index()].learn_fallen(prop.id);
    }

    /// Record that toppling `prop` went down toward `toward` this tick. A
    /// degenerate direction (a burst at the foot) falls back to one from the
    /// body's id, so the record is always a unit vector and deterministic.
    fn fell(&mut self, prop: &crate::world::Prop, toward: V2) {
        let len = toward.length();
        let toward = if len > 1e-9 {
            toward * (1.0 / len)
        } else {
            // No trigonometry: it must match bit for bit in Wasm.
            const D: f64 = std::f64::consts::FRAC_1_SQRT_2;
            const COMPASS: [(f64, f64); 8] = [
                (1.0, 0.0),
                (D, D),
                (0.0, 1.0),
                (-D, D),
                (-1.0, 0.0),
                (-D, -D),
                (0.0, -1.0),
                (D, -D),
            ];
            let (x, y) = COMPASS[prop.id as usize % 8];
            v2(x, y)
        };
        self.fallen.insert(
            prop.id,
            FallenBody {
                prop: prop.id,
                at: [prop.center.x, prop.center.y],
                toward: [toward.x, toward.y],
                tick: self.tick,
            },
        );
    }

    /// `prop` is gone: side `by` (the one that touched it) replans without
    /// it at once; every other side that planned with it keeps it standing
    /// until it sees where it stood (L1).
    fn keep_standing(&mut self, prop: &crate::world::Prop, by: Option<Side>) {
        for side in Side::ALL {
            let known = &mut self.sides[side.index()];
            if Some(side) == by {
                known.forget(prop);
            } else if prop.id < self.authored_props || known.seen.contains_key(&prop.id) {
                known.keep_standing(prop.clone());
            }
        }
    }

    /// Destroy a prop whose integrity ran out (Q17) into its row's destroyed
    /// state: a tree falls and its spot becomes open ground, like a lane a
    /// tank knocks through; a crate is removed; sandbags, a wall, a building
    /// or a wreck leave their remains on the same plan (a building's
    /// occupants escape its collapse, L10). No side learns of it by contact:
    /// each sees it gone once it sees where it stood. A tree falls `toward`
    /// the way the felling blow pushed. Returns units destroyed.
    fn destroy_prop(&mut self, id: PropId, toward: V2) -> Vec<UnitId> {
        let Some(owner) = self.world.structure_owner(id) else {
            return Vec::new();
        };
        let parts = self.world.structure_parts(owner);
        let replacements: Vec<_> = parts
            .into_iter()
            .filter_map(|part| self.destroy_part(part, toward).map(|new| (part, new)))
            .collect();
        self.world.replace_building_parts(owner, &replacements);
        // All shells/remains are final before any occupant attempts escape.
        garrison::collapse(
            &self.world,
            &mut self.units,
            owner,
            &self.rules,
            &mut self.damage_rng,
            self.tick,
        )
    }

    fn destroy_part(&mut self, id: PropId, toward: V2) -> Option<PropId> {
        let prop = self.world.prop(id).cloned()?;
        let state = self.world.prop_type(prop.kind).destroyed.clone()?;
        self.keep_standing(&prop, None);
        // Whatever it leaves, a toppling body falls the way it was pushed.
        if prop.body.topples {
            self.fell(&prop, toward);
        }
        match state {
            Destroyed::Cleared => {
                self.world.knock_down(id);
                // The tree's own share of the forest, out to its spacing.
                let reach = if prop.forest_tree {
                    self.world.forest_rule().trunk_spacing_m
                } else {
                    self.world.cleared_cell_m()
                };
                for cell in self.world.clear_spot(prop.center, reach) {
                    self.ground.clear(cell);
                }
                None
            }
            Destroyed::Removed => {
                self.world.remove_prop(id);
                self.structures.note_removed(prop);
                None
            }
            Destroyed::Into {
                prop: remains,
                mut height_m,
                building,
            } => {
                let mut remains = remains;
                if let Some(rule) = building {
                    if let Some(definition) = self.world.building(id) {
                        let geometry = &definition.geometry;
                        let floors = geometry.floor_z.as_ref().map_or(1, Vec::len);
                        if floors > rule.collapse_max_floors {
                            remains = rule.gutted_prop;
                            height_m = 2.0 * prop.half.z;
                        } else {
                            height_m = (geometry.height_m * rule.height_fraction)
                                .clamp(height_m, rule.max_height_m);
                        }
                    }
                }
                self.world.remove_prop(id);
                let remains = self.world.add_replacement(
                    &PropDefinition {
                        kind: remains,
                        center: [prop.center.x, prop.center.y],
                        yaw: prop.yaw,
                        half_extents: [prop.half.x, prop.half.y, height_m / 2.0],
                        base_z: Some(prop.base_z),
                        // A lighter wreck is still the same unit's.
                        wreck_of: prop
                            .wreck_of
                            .map(|t| self.rules.catalog.id(t).to_string()),
                    },
                    id,
                );
                self.schedule_expiry(remains);
                self.structures.note_replaced(remains, id);
                // Remains that still close an authored body's footprint to
                // every mover it stopped are planned with by every side, as
                // the body was: a fall a side never saw cannot open a route.
                let closes =
                    |c| !prop.blocks(c) || self.world.prop(remains).is_some_and(|r| r.blocks(c));
                if id < self.authored_props && MoverClass::ALL.into_iter().all(closes) {
                    self.world.set_known_to_all(remains);
                }
                Some(remains)
            }
        }
    }

    /// Every vehicle that knocks trees down and moved this tick clears the
    /// forest ground its hull (and `ground.lane_margin_m` to either side) has left
    /// behind: the lane stops being forest (Q16), with crushed-ground marks.
    /// The ground under the hull itself stays forest until it has passed, so
    /// carving a lane goes at forest speed and only the lane is open ground.
    fn clear_lanes(&mut self, before: &Poses) {
        // A forest is its trees: what knocks them down is its tree's weight.
        let tree = self.world.types().by_id(&self.rules.forests.tree).body;
        for (u, was) in self.units.iter().zip(&before.units) {
            let knocks = tree.topples && u.mobility.push.pushes(tree.weight_class);
            if !u.alive() || !knocks || (was.base - u.position).length() <= 1e-9 {
                continue;
            }
            let (Some(hull), Some(h)) = (u.hull_box(), u.hull) else {
                continue;
            };
            if !self.world.forest_near(
                hull.center,
                hull.half.length() + self.rules.ground.lane_margin_m,
            ) {
                continue;
            }
            let left = Obb2 {
                center: was.base.xy(),
                yaw: was.yaw,
                // Wider, not longer: a margin ahead would clear the ground
                // the hull is about to cover.
                half: h.xy() + v2(0.0, self.rules.ground.lane_margin_m),
            };
            for cell in self.world.clear(&left, &hull) {
                self.ground.clear(cell);
            }
        }
    }

    /// The lab emitter: a round of `weapon` bursts on the ground at `point`,
    /// marking it and wearing down the props its blast reaches.
    fn burst_event(&mut self, point: [f64; 2], weapon: &str) {
        let w = self
            .arsenal
            .weapons
            .iter()
            .find(|w| w.id == weapon)
            .expect("burst weapons are checked at setup");
        let Some(z) = self.world.height_at(point[0], point[1]) else {
            return;
        };
        let at = v3(point[0], point[1], z);
        // Its blast wears the props it reaches down, as a round's would (Q17).
        let mut structural = Vec::new();
        damage::blast_props(&self.world, &w.def, at, None, None, &mut structural);
        self.ground
            .burst(&self.world, at, w.def.blast_radius_m, &self.rules.ground);
        for hit in structural {
            if self.structures.damage(&self.world, hit.prop, hit.amount) {
                self.destroy_prop(hit.prop, hit.toward);
            }
        }
    }

    /// Where every walking soldier and every vehicle track touches the ground,
    /// in unit order: the same list before and after movement.
    fn treads(&self) -> Vec<(V2, Wear)> {
        let mut out = Vec::new();
        for unit in self.units.iter().filter(|u| u.alive() && !u.garrisoned()) {
            match unit.hull {
                Some(half) => {
                    let side = v2(0.0, half.y * self.rules.ground.track_gauge).rotated(unit.yaw);
                    let c = unit.position.xy();
                    out.push((c + side, Wear::Tracks));
                    out.push((c - side, Wear::Tracks));
                }
                None => out.extend(unit.member_positions().map(|p| (p.xy(), Wear::Trampled))),
            }
        }
        out
    }

    /// The opponent reads its side's fresh observation and commands through
    /// the same path as a player; its commands are recorded, so a replay
    /// replays them and never reruns it. Its memory is outside the digest:
    /// continuing live play from a replay or a snapshot would need it.
    fn opponent_turn(&mut self) {
        if self.replaying.is_some() {
            return;
        }
        let mut commands = Vec::new();
        if let Some((op, defender)) = self.opponent.as_mut() {
            let side = op.side;
            commands.extend(
                defender
                    .decide(op, &self.observations[side.index()], &self.rules)
                    .into_iter()
                    .map(|order| (side, order)),
            );
        }
        for (side, order) in commands {
            let seq = self.next_seq[side.index()];
            let _ = self.accept(CommandEnvelope {
                side,
                seq,
                order,
                queued: false,
            });
        }
    }

    fn skirmish_ai_turn(&mut self) {
        if self.replaying.is_some() || self.finished() {
            return;
        }
        let mut commands = Vec::new();
        if let Some(mut ai) = self.skirmish_ai.take() {
            let setup = &self.skirmish.as_ref().unwrap().setup;
            let side = setup.ai_side.unwrap();
            let entry = setup.sites.entries.iter().find(|e| e.side == side).unwrap();
            let frame = &self.observations[side.index()];
            let mut known = self.sides[side.index()].clone();
            let grid = known.grid(&self.world, self.authored_props);
            let orders = ai.decide(
                side,
                setup.factions[side.index()],
                entry,
                frame,
                &self.rules,
                |id, destination| {
                    let own = frame.own.iter().find(|u| u.id == id)?;
                    let mobility = units::mobility(self.rules.catalog.get(own.kind), &self.rules);
                    let from = v2(own.position[0], own.position[1]);
                    let goal = v2(destination[0], destination[1]);
                    let (plan, _) = crate::navigation::plan(
                        grid,
                        &self.roads,
                        crate::navigation::Leg {
                            from,
                            goal,
                            m: &mobility,
                            policy: RoutePolicy::Fastest,
                            avoid: &[],
                        },
                        &self.rules.navigation,
                    );
                    let crate::navigation::Plan::Route(route) = plan else {
                        return None;
                    };
                    let time = grid.route_time(from, &route, &mobility);
                    time.is_finite().then_some(time)
                },
            );
            self.skirmish_ai = Some(ai);
            commands.extend(orders.into_iter().map(|order| (side, order)));
        }
        for (side, order) in commands {
            let seq = self.next_seq[side.index()];
            let _ = self.accept(CommandEnvelope {
                side,
                seq,
                order,
                queued: false,
            });
        }
    }

    /// `unit` fired: every opposing side learns an uncertain firing area and
    /// may hear the shot, sounding like the weapon rows in `heard` (the lab
    /// emitter's shot is of no weapon: 0). Weapons and the lab emitter share
    /// this seam.
    fn record_fire(&mut self, unit: UnitId, heard: u32) {
        let Some(shooter) = self.units.get(unit.0 as usize) else {
            return;
        };
        let (side, at) = (shooter.side, shooter.position.xy());
        for other in Side::ALL.into_iter().filter(|s| *s != side) {
            self.knowledge[other.index()].note_fire(unit, at, heard);
        }
        self.fired.insert(unit);
    }

    /// Every unit's pose (ground contact centre and heading), and every
    /// soldier's position in unit then member order.
    fn poses(&self) -> Poses {
        Poses {
            units: self
                .units
                .iter()
                .map(|u| Pose {
                    base: u.position,
                    yaw: u.yaw,
                })
                .collect(),
            soldiers: self
                .units
                .iter()
                .flat_map(|u| u.members.iter().map(|s| s.exposed(self.tick)))
                .collect(),
        }
    }

    /// Colliders for this tick: a capsule per living soldier, swept from
    /// where he stood to where he stands, and a box per vehicle.
    fn bodies(&self, before: &Poses, after: &Poses) -> Vec<Body> {
        let b = &self.rules.physics;
        let mut bodies = Vec::new();
        let mut first = 0;
        for (i, unit) in self.units.iter().enumerate() {
            let soldiers = first..first + unit.members.len();
            first = soldiers.end;
            if !unit.alive() {
                continue;
            }
            match unit.hull {
                Some(half) => bodies.push(Body {
                    id: BodyId(VEHICLE_BODY_BASE + unit.id.0),
                    unit: unit.id,
                    shape: Shape::Box { half },
                    from: before.units[i],
                    to: after.units[i],
                }),
                None => {
                    for (s, k) in unit.members.iter().zip(soldiers).filter(|(s, _)| s.alive()) {
                        let at = |p: &Poses, yaw: f64| Pose {
                            base: p.soldiers[k],
                            yaw,
                        };
                        bodies.push(Body {
                            id: BodyId(s.id),
                            unit: unit.id,
                            shape: Shape::Capsule {
                                radius: b.soldier_radius_m,
                                height: b.soldier_height_m,
                            },
                            from: at(before, before.units[i].yaw),
                            to: at(after, after.units[i].yaw),
                        });
                    }
                }
            }
        }
        bodies
    }

    /// Fly every round one tick against the world and this tick's bodies,
    /// keeping each round's flown segment for visibility.
    fn fly(&mut self, before: &Poses, after: &Poses) {
        let bodies = self.bodies(before, after);
        // Where each round starts this tick, and the soldier who fired it.
        let starts: BTreeMap<ProjectileId, (V3, Option<u32>)> = self
            .projectiles
            .active()
            .iter()
            .map(|p| {
                let soldier = p
                    .shooter
                    .map(|s| s.body.0)
                    .filter(|&b| b < VEHICLE_BODY_BASE);
                (p.id, (p.position, soldier))
            })
            .collect();
        self.flight_events.clear();
        let mut resolver = HullResolver {
            rules: &self.rules,
            arsenal: &self.arsenal,
            rounds: &self.rounds,
            units: &mut self.units,
            tick: self.tick,
            rng: &mut self.ricochet_rng,
        };
        flight::advance_projectiles(
            &mut self.projectiles,
            &self.world,
            &bodies,
            &mut self.flight_events,
            &mut resolver,
        );
        let mut ends: BTreeMap<ProjectileId, V3> = self
            .projectiles
            .active()
            .iter()
            .map(|p| (p.id, p.position))
            .collect();
        let mut struck = BTreeMap::new();
        let mut ricochets: BTreeMap<ProjectileId, Vec<(V3, V3)>> = BTreeMap::new();
        self.blasts.clear();
        for e in &self.flight_events {
            match e {
                FlightEvent::Impact(i) => {
                    ends.insert(i.projectile, i.point);
                    let hit = match i.struck {
                        Struck::Terrain => SegmentHit::Ground,
                        Struck::Prop(_) => SegmentHit::Prop,
                        Struck::Body(b) if b.0 >= VEHICLE_BODY_BASE => SegmentHit::Hull,
                        Struck::Body(_) => SegmentHit::Soldier,
                    };
                    struck.insert(i.projectile, (hit, i.normal));
                    if let Some(round) = self.rounds.get(&i.projectile).filter(|_| i.detonated) {
                        let blast = Blast {
                            point: xyz(i.point),
                            radius: self.arsenal.weapons[round.weapon].def.blast_radius_m,
                            kind: round.weapon,
                        };
                        self.blasts.push((round.side, blast));
                    }
                }
                // Events come in time order, so each round's ricochets do too.
                FlightEvent::Ricochet(r) => {
                    ricochets
                        .entry(r.projectile)
                        .or_default()
                        .push((r.point, r.normal));
                }
                FlightEvent::Expired(x) => {
                    ends.insert(x.projectile, x.point);
                }
                FlightEvent::NearMiss(_) | FlightEvent::Pass(_) => {}
            }
        }
        self.segments.clear();
        for (id, (from, shooter)) in starts {
            if let (Some(&to), Some(round)) = (ends.get(&id), self.rounds.get(&id)) {
                self.segments.push(Flown {
                    side: round.side,
                    from,
                    ricochets: ricochets.remove(&id).unwrap_or_default(),
                    to,
                    weapon: round.weapon,
                    shooter,
                    hit: struck.get(&id).copied(),
                });
            }
        }
        let ctx = DamageContext {
            world: &self.world,
            ground: &self.ground,
            arsenal: &self.arsenal,
            rules: &self.rules,
            tick: self.tick,
        };
        let outcome = damage::resolve(
            &ctx,
            &self.flight_events,
            &self.rounds,
            &mut self.suppressed,
            &mut self.units,
            &mut self.damage_rng,
        );
        self.consequences(outcome);
        // Each burst marks the ground after it has done its damage.
        for (_, blast) in &self.blasts {
            let [x, y, z] = blast.point;
            self.ground
                .burst(&self.world, v3(x, y, z), blast.radius, &self.rules.ground);
        }
        let live: BTreeSet<ProjectileId> = self.projectiles.active().iter().map(|p| p.id).collect();
        self.rounds.retain(|id, _| live.contains(id));
        self.suppressed.retain(|(id, _), _| live.contains(id));
    }

    /// Hostile damage or suppression grants return fire; a destroyed vehicle
    /// leaves a permanent wreck (M06, M07); a death a side was watching ends
    /// its track, while an unseen death discloses nothing. The vehicle's own
    /// side knows where it died, and a side that watched it die saw the wreck
    /// appear: both learn the wreck at once, so a vehicle never vanishes from
    /// the picture of a side that knew where it stood.
    fn consequences(&mut self, outcome: damage::Outcome) {
        for (victim, shooter) in outcome.attacked {
            self.units[victim.0 as usize].attackers.insert(shooter);
        }
        let mut destroyed = outcome.destroyed;
        // Structural damage in event order; a prop is destroyed once (L10, Q17).
        for hit in outcome.structural {
            if self.structures.damage(&self.world, hit.prop, hit.amount) {
                destroyed.extend(self.destroy_prop(hit.prop, hit.toward).into_iter().map(
                    |victim| damage::UnitDeath {
                        victim,
                        source: hit.source,
                    },
                ));
            }
        }
        if let Some(skirmish) = &mut self.skirmish {
            let deaths: Vec<_> = destroyed
                .iter()
                .filter_map(|death| {
                    let entered = skirmish.entered.get(&death.victim)?;
                    let unit = &self.units[death.victim.0 as usize];
                    Some(crate::settlement::EconomicDeath {
                        victim: death.victim,
                        side: unit.side,
                        price: entered.price,
                        source: death.source,
                        supply: unit.unit_type(&self.rules).capabilities.supply.is_some(),
                    })
                })
                .collect();
            let starting_budget = u64::from(skirmish.setup.rules.credits_per_minute)
                * u64::from(skirmish.setup.rules.starting_minutes);
            skirmish
                .settlement
                .settle(&deaths, starting_budget, &mut skirmish.wallets);
        }
        for death in destroyed {
            let id = death.victim;
            let unit = &self.units[id.0 as usize];
            let own = unit.side;
            let wreck = unit.unit_type(&self.rules).hull().map(|h| h.wreck.clone());
            let hulls: Vec<Obb2> = self
                .units
                .iter()
                .filter(|o| o.id != id && o.alive())
                .filter_map(|o| o.hull_box())
                .collect();
            let soldiers: Vec<V2> = self
                .units
                .iter()
                .filter(|o| o.id != id)
                .flat_map(|o| o.member_positions().map(|p| p.xy()))
                .collect();
            let rest = crate::movement::drive::death_roll(
                &self.world,
                &self.rules,
                unit,
                &hulls,
                &soldiers,
            );
            let wreck = match (unit.hull, wreck) {
                (Some(half), Some(kind)) => Some(self.add_prop(&PropDefinition {
                    kind,
                    center: [rest.x, rest.y],
                    yaw: unit.yaw,
                    half_extents: [half.x, half.y, half.z],
                    base_z: Some(unit.position.z),
                    wreck_of: Some(self.rules.catalog.id(unit.kind).to_string()),
                })),
                _ => None,
            };
            let mut knowing = vec![own];
            for side in Side::ALL.into_iter().filter(|&s| s != own) {
                let knowledge = &mut self.knowledge[side.index()];
                if knowledge.identifies(id, self.tick - 1) {
                    knowledge.saw_destroyed(id);
                    knowing.push(side);
                }
            }
            if let Some(prop) = wreck.and_then(|w| self.world.prop(w)) {
                for side in knowing {
                    self.sides[side.index()].learn(
                        prop,
                        self.authored_props,
                        self.rules.pushing.relearn_m,
                        true,
                    );
                }
            }
        }
    }

    /// An attack pursues an identified target it cannot fire on from here, or
    /// its last reported place once identification lapses (W17); it ends on
    /// arriving there unseen, or when its area expires unidentified.
    fn update_pursuit(&mut self) {
        for unit in &mut self.units {
            unit.pursuit = None;
            // A garrisoned attack fires from the building; it never walks out.
            if unit.garrisoned() {
                continue;
            }
            let knowledge = &self.knowledge[unit.side.index()];
            let Some(UnitOrder::Attack { target, last_known }) = unit.orders.front_mut() else {
                continue;
            };
            let out_of_reach = unit.reach.needs_closer;
            let done = match *target {
                // A death the side watched completes the attack.
                Target::Unit(u) if knowledge.knows_destroyed(u) => true,
                Target::Unit(u) => match knowledge.track(u) {
                    Some(track) => {
                        *last_known = Some(track.position.xy());
                        if out_of_reach {
                            unit.pursuit = Some(Pursuit {
                                to: track.position.xy(),
                                searching: false,
                            });
                        }
                        false
                    }
                    None => match *last_known {
                        Some(p) if (unit.position.xy() - p).length() > PURSUIT_ARRIVAL_M => {
                            unit.pursuit = Some(Pursuit {
                                to: p,
                                searching: true,
                            });
                            false
                        }
                        _ => true,
                    },
                },
                Target::Contact(c) => knowledge.contact(c).is_none(),
                Target::Ground(_) => false,
            };
            if done {
                unit.orders.pop_front();
            }
        }
    }

    /// Return-fire permission lasts only while the side still knows the attacker.
    fn prune_attackers(&mut self) {
        for unit in &mut self.units {
            let knowledge = &self.knowledge[unit.side.index()];
            unit.attackers.retain(|&a| {
                knowledge.track(a).is_some() || knowledge.all_contacts().any(|c| c.emitter == a)
            });
        }
    }

    /// Guided missiles (P05, P06): a launcher supports its missile while it
    /// stands still, has a living operator (including a survivor who takes
    /// over the launcher), its side identifies the target, and its operator
    /// has physical line of sight independent of spotting range. It
    /// steers it at the target's observed position. Losing any of these (or a
    /// Stop, which drops the mount's support) releases it at once and for good:
    /// the missile coasts straight on for `guided.release_coast_s`, then goes
    /// to ground, so a far one misses a target it can no longer see.
    fn guide(&mut self, moved: &[bool]) {
        let mut supported = BTreeSet::new();
        let alive: Vec<bool> = self.units.iter().map(|u| u.alive()).collect();
        // Guidance and gun fire share the vehicle aim height.
        let aim_z: Vec<f64> = self
            .units
            .iter()
            .map(|u| {
                u.hull.map_or(self.rules.physics.infantry_aim_m, |h| {
                    2.0 * h.z * self.rules.physics.vehicle_aim_height_fraction
                })
            })
            .collect();
        for (i, unit) in self.units.iter_mut().enumerate() {
            weapons::assign_operators(&self.arsenal, unit);
            let knowledge = &self.knowledge[unit.side.index()];
            for m in 0..unit.mounts.len() {
                let mount = &unit.mounts[m];
                let Some(s) = mount.support else { continue };
                let target = match s.target {
                    Target::Unit(t) => Some(t),
                    _ => None,
                };
                let sighting = target
                    .filter(|&t| alive[t.0 as usize])
                    // Guidance precedes this tick's sensing and reads the
                    // previous completed observation, as before.
                    .and_then(|t| {
                        knowledge
                            .track(t)
                            .filter(|tr| tr.last_seen + 1 == self.tick)
                            .map(|tr| tr.position + v3(0.0, 0.0, aim_z[t.0 as usize]))
                    });
                // Entering or leaving a building is a transition that releases too.
                let settled = unit
                    .garrison
                    .as_ref()
                    .is_none_or(|g| matches!(g.phase, garrison::Phase::Inside));
                let keep = !moved[i]
                    && alive[i]
                    && settled
                    && (unit.hull.is_some() || mount.operator.is_some())
                    && self.projectiles.get(s.projectile).is_some();
                match sighting.filter(|&at| {
                    keep && weapons::guidance_clear(&self.world, &self.rules, unit, mount, at)
                }) {
                    Some(at) => {
                        self.projectiles.steer(s.projectile, at);
                        supported.insert(s.projectile);
                    }
                    None => unit.mounts[m].support = None,
                }
            }
        }
        let released: Vec<ProjectileId> = self
            .projectiles
            .active()
            .iter()
            .filter(|p| p.guidance.is_some_and(|g| g.supported) && !supported.contains(&p.id))
            .map(|p| p.id)
            .collect();
        for id in released {
            self.projectiles
                .release(id, self.rules.guided.release_coast_s, &self.world);
        }
    }

    /// Every mount acts; shots become rounds in flight and firing evidence.
    /// Returns the units that launched this tick.
    fn fire(&mut self, moved: &[bool]) -> BTreeSet<UnitId> {
        // Flight may have killed an operator after this tick's guidance.
        for unit in &mut self.units {
            weapons::assign_operators(&self.arsenal, unit);
        }
        let ctx = FireContext {
            world: &self.world,
            structures: &self.structures,
            ground: &self.ground,
            arsenal: &self.arsenal,
            rules: &self.rules,
            tick: self.tick,
            knowledge: &self.knowledge,
        };
        let aims = weapons::garrison_aims(&ctx, &self.units);
        garrison::allocate_slots(&mut self.units, &aims, &self.rules, self.tick);
        let shots = weapons::advance(&ctx, &mut self.units, moved, &mut self.combat_rng);
        let fired = shots.iter().map(|s| s.unit).collect();
        for shot in shots {
            let side = self.units[shot.unit.0 as usize].side;
            for launch in shot.launches {
                let guided = launch.guidance.is_some();
                let id = self.projectiles.launch(launch);
                self.rounds.insert(
                    id,
                    Round {
                        weapon: shot.weapon,
                        unit: shot.unit,
                        side,
                    },
                );
                // A guided round is supported by the mount that launched it.
                if guided {
                    let unit = &mut self.units[shot.unit.0 as usize];
                    unit.mounts[shot.mount].support = Some(Support {
                        projectile: id,
                        target: shot.target,
                    });
                }
            }
            // The attacked unit may answer this attacker under Return fire only.
            if let Target::Unit(t) = shot.target {
                self.units[t.0 as usize].attackers.insert(shot.unit);
            }
            let shooter = &self.units[shot.unit.0 as usize];
            let heard = self
                .arsenal
                .heard(shooter.kind, shooter.mounts[shot.mount].spec);
            self.record_fire(shot.unit, heard);
        }
        fired
    }

    /// This side's own sensors, folded into its knowledge: the observers
    /// due this tick (`all`: every one) identify afresh, the rest keep their
    /// last sightings ([`sensing::SENSE_EVERY`]).
    fn sense(&mut self, side: Side, all: bool) {
        let tick = self.tick;
        let due = |u: &Unit| all || sensing::due(u, tick);
        let units = &self.units;
        let mut sightings: Vec<Sighting> = self.sightings[side.index()]
            .iter()
            .filter(|s| !due(&units[s.observer.0 as usize]))
            .filter_map(|s| sensing::kept(s, units, &self.rules))
            .collect();
        sightings.extend(sensing::evaluate(
            &self.world,
            units,
            &self.rules,
            side,
            due,
        ));
        sightings.sort_by_key(|s| (s.observer, s.target));
        let identified = self.knowledge[side.index()].update(tick, &sightings, units, &self.rules);
        self.sightings[side.index()] = sightings;
        // An attack on an area whose cause is now identified carries over to
        // that enemy, queued ones too, as if the player had right-clicked it.
        for (area, enemy) in identified {
            for unit in self.units.iter_mut().filter(|u| u.side == side) {
                for order in &mut unit.orders {
                    match order {
                        UnitOrder::Attack { target, .. } if *target == Target::Contact(area) => {
                            *target = Target::Unit(enemy);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Recompute what ground this side sees, and learn any new obstacle in view.
    fn sweep_fog(&mut self, side: Side) {
        self.sweep_fog_profiled(side, &mut |_| {});
    }

    fn sweep_fog_profiled(&mut self, side: Side, completed: &mut impl FnMut(TickPhase)) {
        let mut field = self.occlusion.field();
        let mut candidates = Vec::new();
        let mut remembered = Vec::new();
        let mut views = Vec::new();
        for unit in self.units.iter().filter(|u| u.side == side && u.alive()) {
            let sight = sight::of(unit, &self.rules);
            let eyes = sensing::eyes(unit, &self.rules);
            for eye in &eyes {
                // A ray can step one cell past its reach, and the marked cell
                // can contain a footprint sample a diagonal farther away.
                views.push((eye.xy(), sight.max_range() + 3.0 * field.cell_m));
            }
            self.occlusion.sweep_observer(
                &self.world,
                &self.rules.sensors,
                unit.id,
                &eyes,
                &sight,
                &mut field,
            );
        }
        self.world.prop_ids_near_many(&views, &mut candidates);
        self.sides[side.index()].standing_ids_near_many(&views, &mut remembered);
        completed(TickPhase::Fog);
        // Enemy fallen in view are remembered, and the ground in view learned.
        let knowledge = &mut self.knowledge[side.index()];
        knowledge.learn_ground(&self.ground, &field);
        for f in self.fallen.values() {
            if field.visible(f.at[0], f.at[1]) {
                knowledge.learn_fallen(f.prop);
            }
        }
        for u in self.units.iter().filter(|u| u.side != side) {
            for s in &u.members {
                if let Some(fallen) = s.corpse.filter(|f| field.visible(f.at.x, f.at.y)) {
                    knowledge.note_corpse(s.id, fallen);
                }
            }
        }
        // Bodies in view are learned where they stand: new ones, and known
        // ones seen moved (L1, L2); trees seen fallen are gone (Q16).
        let known = &mut self.sides[side.index()];
        let revealed: std::collections::BTreeSet<_> = candidates
            .iter()
            .filter_map(|&id| self.world.prop(id))
            .chain(remembered.iter().filter_map(|id| known.standing().get(id)))
            .filter_map(|p| self.world.building_of(p.id).map(|owner| (owner, p)))
            .filter(|(_, p)| footprint_seen(&field, p))
            .map(|(owner, _)| owner)
            .collect();
        // Seeing one physical part reveals the entire aggregate, including
        // parts beyond the eye's reach. Keep learning in ascending prop order.
        candidates.extend(
            revealed
                .iter()
                .flat_map(|&owner| self.world.current_building_parts(owner).iter().copied()),
        );
        candidates.sort_unstable();
        candidates.dedup();
        remembered.extend(
            revealed
                .iter()
                .flat_map(|&owner| self.world.historical_building_parts(owner).iter().copied()),
        );
        remembered.sort_unstable();
        remembered.dedup();
        let fallen: Vec<PropId> = remembered
            .into_iter()
            .filter_map(|id| known.standing().get(&id))
            .filter(|p| {
                footprint_seen(&field, p)
                    || self
                        .world
                        .building_of(p.id)
                        .is_some_and(|id| revealed.contains(&id))
            })
            .map(|p| p.id)
            .collect();
        for id in fallen {
            known.saw_fallen(id);
        }
        let relearn = self.rules.pushing.relearn_m;
        for prop in candidates.into_iter().filter_map(|id| self.world.prop(id)) {
            if (prop.id >= self.authored_props || known.seen.contains_key(&prop.id))
                && (footprint_seen(&field, prop)
                    || self
                        .world
                        .building_of(prop.id)
                        .is_some_and(|id| revealed.contains(&id)))
            {
                let resting = self.world.resting(prop.id, self.tick);
                known.learn(prop, self.authored_props, relearn, resting);
            }
        }
        self.fog[side.index()] = field;
    }

    fn prepare(
        &mut self,
        command: CommandEnvelope,
        planning_tick: Tick,
    ) -> Result<PreparedCommand, OrderError> {
        self.validate(&command)?;
        if matches!(command.order, Order::Ready) {
            self.skirmish.as_mut().unwrap().ready[command.side.index()] = true;
        }
        if let Order::CancelPending { purchase } = command.order {
            self.skirmish
                .as_mut()
                .unwrap()
                .cancel(command.side, purchase);
        }
        if let Order::ConfirmPurchase {
            variant,
            destination,
        } = &command.order
        {
            let kind = self.preview_purchase(command.side, variant, *destination)?;
            let living = self
                .units
                .iter()
                .filter(|u| u.side == command.side && u.alive())
                .count() as u32;
            self.skirmish.as_mut().unwrap().reserve(
                command.side,
                kind,
                *destination,
                self.rules.catalog.get(kind).cost,
                planning_tick + 1,
                living,
            )?;
        }
        let building = if let Order::OccupyBuilding {
            units,
            building,
            facing,
            ..
        } = &command.order
        {
            Some(self.plan_building(
                command.side,
                &BuildingPreviewRequest {
                    units: units.clone(),
                    building: *building,
                    facing: *facing,
                    queued: command.queued,
                },
                planning_tick,
            )?)
        } else {
            None
        };
        let mut stops = BTreeMap::new();
        let placement = match &command.order {
            Order::Refund { units } => {
                let entry = self
                    .skirmish
                    .as_ref()
                    .unwrap()
                    .setup
                    .sites
                    .entries
                    .iter()
                    .find(|entry| entry.side == command.side)
                    .unwrap();
                Some(MovePlacement {
                    gesture: 0,
                    destinations: units
                        .iter()
                        .map(|unit| contract::command::MoveDestination {
                            unit: *unit,
                            goal: entry.center,
                            placed: true,
                            facing: entry.yaw,
                        })
                        .collect(),
                })
            }
            Order::OccupyBuilding { gesture, .. } => Some(MovePlacement {
                gesture: *gesture,
                destinations: building.as_ref().unwrap().placement.destinations.clone(),
            }),
            Order::Move {
                units,
                gesture,
                goal,
                facing,
                direction,
                route,
                ..
            } => {
                let (destinations, found) = self.place_move(
                    command.side,
                    &MovePreviewRequest {
                        units: units.clone(),
                        goal: *goal,
                        facing: *facing,
                        direction: *direction,
                        route: *route,
                        queued: command.queued,
                    },
                )?;
                stops = found;
                Some(MovePlacement {
                    gesture: *gesture,
                    destinations,
                })
            }
            Order::AttackMove {
                units,
                gesture,
                goal,
            } => {
                let (destinations, found) = self.place_move(
                    command.side,
                    &MovePreviewRequest {
                        units: units.clone(),
                        goal: *goal,
                        queued: command.queued,
                        ..Default::default()
                    },
                )?;
                stops = found;
                Some(MovePlacement {
                    gesture: *gesture,
                    destinations,
                })
            }
            _ => None,
        };
        Ok(PreparedCommand {
            command,
            placement,
            building,
            stops,
        })
    }

    fn apply(&mut self, prepared: PreparedCommand) {
        let mut units = std::mem::take(&mut self.units);
        self.apply_units(prepared, &mut units);
        self.units = units;
    }

    fn apply_units(&self, prepared: PreparedCommand, movers: &mut [Unit]) {
        let command = prepared.command;
        let side = command.side;
        let queued = command.queued;
        let push = |unit: &mut Unit, order: UnitOrder| unit.enqueue(order, queued);
        let replacing = match &command.order {
            Order::Move { units, .. }
            | Order::AttackMove { units, .. }
            | Order::Stop { units }
            | Order::Attack { units, .. }
            | Order::SetEngagement { units, .. }
            | Order::SetDeployment { units, .. }
            | Order::Garrison { units, .. }
            | Order::OccupyBuilding { units, .. }
            | Order::ExitBuilding { units } => Some(units),
            _ => None,
        };
        if let Some(units) = replacing {
            for id in units {
                movers[id.0 as usize].withdrawing = false;
            }
        }
        match command.order {
            Order::Refund { .. } => {
                for slot in prepared.placement.unwrap().destinations {
                    if !slot.placed {
                        continue;
                    }
                    let unit = &mut movers[slot.unit.0 as usize];
                    unit.enqueue(
                        UnitOrder::Move(MoveOrder {
                            destination: v2(slot.goal[0], slot.goal[1]),
                            policy: RoutePolicy::Fastest,
                            gesture: 0,
                            direction: Default::default(),
                            facing: None,
                            short: None,
                        }),
                        false,
                    );
                    unit.withdrawing = true;
                }
            }
            Order::Ready | Order::ConfirmPurchase { .. } | Order::CancelPending { .. } => {}
            Order::OccupyBuilding {
                gesture, facing, ..
            } => {
                let garrison::BuildingPlan {
                    placement: plan,
                    entry_action,
                } = prepared.building.unwrap();
                if let Some(entry) = plan.entrant {
                    let unit = &mut movers[entry.unit.0 as usize];
                    let approach = v2(entry.approach[0], entry.approach[1]);
                    if matches!(entry_action, garrison::EntryAction::Reassert) {
                        unit.enqueue(
                            UnitOrder::Garrison {
                                building: plan.building,
                                approach,
                            },
                            false,
                        );
                    } else if matches!(entry_action, garrison::EntryAction::Route) {
                        push(
                            unit,
                            UnitOrder::Move(MoveOrder {
                                destination: approach,
                                policy: RoutePolicy::Shortest,
                                gesture,
                                direction: Default::default(),
                                facing: None,
                                short: None,
                            }),
                        );
                        unit.enqueue(
                            UnitOrder::Garrison {
                                building: plan.building,
                                approach,
                            },
                            true,
                        );
                    }
                }
                for slot in plan.destinations {
                    let unit = &mut movers[slot.unit.0 as usize];
                    if slot.placed {
                        push(
                            unit,
                            UnitOrder::Move(MoveOrder {
                                destination: v2(slot.goal[0], slot.goal[1]),
                                policy: RoutePolicy::Shortest,
                                gesture,
                                direction: Default::default(),
                                facing,
                                short: None,
                            }),
                        );
                    } else if !queued {
                        Self::hold_position(unit);
                    }
                }
            }
            Order::Move {
                units: _,
                gesture,
                goal: _,
                route,
                direction,
                facing,
            } => {
                for slot in prepared.placement.unwrap().destinations {
                    let unit = &mut movers[slot.unit.0 as usize];
                    if !slot.placed {
                        if !queued {
                            Self::hold_position(unit);
                        }
                        continue;
                    }
                    let destination = v2(slot.goal[0], slot.goal[1]);
                    let order = UnitOrder::Move(MoveOrder {
                        destination,
                        policy: route,
                        gesture,
                        direction,
                        facing,
                        short: prepared.stops.get(&slot.unit).cloned(),
                    });
                    push(unit, order);
                }
            }
            Order::AttackMove { gesture, .. } => {
                for slot in prepared.placement.unwrap().destinations {
                    let unit = &mut movers[slot.unit.0 as usize];
                    if !slot.placed {
                        if !queued {
                            Self::hold_position(unit);
                        }
                        continue;
                    }
                    let destination = v2(slot.goal[0], slot.goal[1]);
                    unit.engagement = Engagement::FireAtWill; // W14
                    let order = UnitOrder::AttackMove(MoveOrder {
                        destination,
                        policy: contract::command::RoutePolicy::Shortest,
                        gesture,
                        direction: contract::command::MoveDirection::Forward,
                        facing: None,
                        short: prepared.stops.get(&slot.unit).cloned(),
                    });
                    push(unit, order);
                }
            }
            Order::Attack { units, target } => {
                // Resolved again at application; a target lost since the ack is dropped.
                let Some(target) = self.resolve_target(side, &target) else {
                    return;
                };
                for id in units {
                    let unit = &mut movers[id.0 as usize];
                    unit.engagement = Engagement::FireAtWill; // W14
                    push(
                        unit,
                        UnitOrder::Attack {
                            target,
                            last_known: None,
                        },
                    );
                }
            }
            Order::Garrison { units, building } => {
                let building = self.world.remembered_structure_owner(building);
                for id in units {
                    let unit = &mut movers[id.0 as usize];
                    let from = unit.position.xy();
                    let Some(approach) = garrison::approach(
                        &self.world,
                        &self.sides[side.index()],
                        self.authored_props,
                        building,
                        from,
                        self.rules.garrison.entry_distance_m / 2.0,
                    ) else {
                        continue;
                    };
                    push(unit, UnitOrder::Garrison { building, approach });
                }
            }
            Order::ExitBuilding { units } => {
                for id in units {
                    push(&mut movers[id.0 as usize], UnitOrder::Exit);
                }
            }
            Order::SetEngagement { units, policy } => {
                for id in units {
                    movers[id.0 as usize].engagement = policy;
                }
            }
            Order::Stop { units } => {
                // W15: clear the queue and cancel movement, aim, reload and
                // guidance once; policy stays; automatic fire may restart.
                for id in units {
                    let unit = &mut movers[id.0 as usize];
                    unit.orders.clear();
                    unit.route = None;
                    unit.planned_goal = None;
                    unit.pursuit = None;
                    unit.state = MoveState::Idle;
                    unit.drive_speed_mps = 0.0;
                    unit.blocker = None;
                    unit.turn_to = None;
                    for mount in &mut unit.mounts {
                        mount.stop();
                    }
                    // Stop never leaves a unit half-packed: it sets up where it is.
                    if let Some(d) = unit.deployment.as_mut() {
                        d.stationary = Posture::Deployed;
                    }
                }
            }
            Order::SetDeployment { units, deployed } => {
                for id in units {
                    let unit = &mut movers[id.0 as usize];
                    let Some(d) = unit.deployment.as_mut() else {
                        continue; // units that never set up ignore it
                    };
                    if deployed {
                        // Set up here: movement ends and progress reverses from where it stands.
                        d.stationary = Posture::Deployed;
                        unit.orders.clear();
                        unit.route = None;
                        unit.planned_goal = None;
                        unit.pursuit = None;
                        unit.state = MoveState::Idle;
                        unit.blocker = None;
                    } else {
                        // Pack and stay packed once any movement ends.
                        d.stationary = Posture::Packed;
                    }
                }
            }
            Order::UpgradeMove { gesture, route } => {
                for unit in movers.iter_mut().filter(|u| u.side == side) {
                    for (k, order) in unit.orders.iter_mut().enumerate() {
                        if let Some(m) = order.movement_mut() {
                            if m.gesture == gesture && m.policy != route {
                                m.policy = route;
                                unit.withdrawing = false;
                                if k == 0 {
                                    unit.route = None; // replan the active order
                                    unit.planned_goal = None;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn hold_position(unit: &mut Unit) {
        unit.orders.clear();
        unit.route = None;
        unit.planned_goal = None;
        unit.pursuit = None;
        unit.state = MoveState::Idle;
        unit.drive_speed_mps = 0.0;
        unit.blocker = None;
        unit.turn_to = None;
    }

    /// Preview placement on this side's known map without advancing time,
    /// recording a command, changing orders, or warming live navigation caches.
    /// Admission uses side knowledge, so an unseen enemy body cannot reject a ghost.
    pub fn preview_purchase(
        &mut self,
        side: Side,
        variant: &str,
        destination: [f64; 2],
    ) -> Result<contract::catalog::TypeIndex, OrderError> {
        let skirmish = self.skirmish.as_ref().ok_or(OrderError::NotSkirmish)?;
        let kind = self
            .rules
            .catalog
            .index(variant)
            .ok_or(OrderError::UnitUnavailable)?;
        let card = self
            .rules
            .catalog
            .card(variant)
            .ok_or(OrderError::UnitUnavailable)?;
        if !card
            .roster
            .factions
            .contains(&skirmish.setup.factions[side.index()])
        {
            return Err(OrderError::WrongFaction);
        }
        if !destination.iter().all(|n| n.is_finite())
            || self
                .world
                .height_at(destination[0], destination[1])
                .is_none()
        {
            return Err(OrderError::OutOfBounds);
        }
        let mobility = units::mobility(self.rules.catalog.get(kind), &self.rules);
        let mut known = self.sides[side.index()].clone();
        if !known
            .grid(&self.world, self.authored_props)
            .placement_fits(v2(destination[0], destination[1]), &mobility)
        {
            return Err(OrderError::NoValidDestination);
        }
        Ok(kind)
    }

    pub fn preview_move(
        &mut self,
        side: Side,
        request: &MovePreviewRequest,
    ) -> Result<Vec<contract::command::MoveDestination>, OrderError> {
        self.place_move(side, request)
            .map(|(destinations, _)| destinations)
    }

    /// Each unit's destination, and where a hull sent onto ground it has no
    /// room on stops short of it instead, if the room nearest is a long way
    /// round ([`crate::units::StopShort`]).
    fn place_move(
        &mut self,
        side: Side,
        request: &MovePreviewRequest,
    ) -> Result<
        (
            Vec<contract::command::MoveDestination>,
            BTreeMap<UnitId, crate::units::StopShort>,
        ),
        OrderError,
    > {
        let ids = &request.units;
        let goal = request.goal;
        let facing = request.facing;
        self.validate_units(side, ids)?;
        if self.world.height_at(goal[0], goal[1]).is_none()
            || facing.is_some_and(|f| !f.is_finite())
        {
            return Err(OrderError::OutOfBounds);
        }
        let members: Vec<_> = ids
            .iter()
            .map(|&id| {
                let u = &self.units[id.0 as usize];
                crate::formation::Member {
                    id,
                    position: u.position.xy(),
                    yaw: u.yaw,
                    radius: u.hull.map_or_else(
                        || cover::area_radius(&self.rules, u),
                        |half| half.xy().length(),
                    ),
                }
            })
            .collect();
        let mut known = self.sides[side.index()].clone();
        let grid = known.grid(&self.world, self.authored_props);
        let pockets = crate::navigation::Pockets::default();
        // Per unit, the last point placed for it and where it stops short.
        let shorts = std::cell::RefCell::new(BTreeMap::new());
        // A queued leg sets off from where the queue ends.
        let from = |u: &Unit| {
            u.orders
                .iter()
                .rev()
                .filter(|_| request.queued)
                .find_map(|o| o.movement())
                .map_or(u.position.xy(), |m| m.destination)
        };
        let plan = crate::formation::place(
            &members,
            v2(goal[0], goal[1]),
            facing,
            &self.rules.formation,
            libm::hypot(self.world.width(), self.world.depth()),
            |id, p| {
                let u = &self.units[id.0 as usize];
                // A hull that turns on the spot parks facing as ordered.
                let pivots = u.mobility.drive.is_some_and(|d| d.tracked);
                let hull = u.hull.map(|h| crate::navigation::Parking {
                    half: h.xy(),
                    facing: request.facing.filter(|_| pivots),
                });
                let placed = grid.destination_point(p, &u.mobility, hull, from(u), &pockets)?;
                let short = hull
                    .filter(|_| placed != p)
                    .and_then(|hull| grid.room_on_the_way(p, &u.mobility, hull, from(u), &pockets))
                    .filter(|&at| at != placed)
                    .map(|at| crate::units::StopShort {
                        at,
                        detour_m: self.rules.navigation.stop_short_detour_ratio
                            * ((at - p).length() - (placed - p).length()).max(0.0),
                        nearest: None,
                    });
                shorts.borrow_mut().insert(id, (placed, short));
                Some(placed)
            },
        );
        // A placed marker is a place to stand that its unit can reach on the
        // ground its side knows; the way there is found and kept on the move.
        let source = self.move_source(side);
        let shorts = shorts.into_inner();
        // Where each placed unit stops short, if it was placed where that
        // was worked out for.
        let stops = plan
            .slots
            .iter()
            .filter_map(|slot| {
                let (placed, short) = shorts.get(&slot.id)?;
                (slot.point == Some(*placed)).then(|| Some((slot.id, short.clone()?)))?
            })
            .collect();
        let destinations = plan
            .slots
            .into_iter()
            .map(|slot| {
                let u = &self.units[slot.id.0 as usize];
                // An attack has no end to set a queued leg off from.
                let after_attack = request.queued
                    && source[slot.id.0 as usize]
                        .orders
                        .iter()
                        .any(|o| matches!(o, UnitOrder::Attack { .. }));
                let slot_point = slot
                    .point
                    .filter(|p| !after_attack && grid.reaches(from(u), *p, &u.mobility, &pockets));
                let goal = slot_point.unwrap_or(u.position.xy());
                let facing =
                    movement::final_yaw(u, request.facing, from(u), goal, request.direction);
                contract::command::MoveDestination {
                    unit: slot.id,
                    goal: [goal.x, goal.y],
                    placed: slot_point.is_some(),
                    facing: facing.unwrap_or(u.yaw),
                }
            })
            .collect();
        Ok((destinations, stops))
    }

    /// Resolve entry and gathering using only this side's known, isolated state.
    pub fn preview_building(
        &self,
        side: Side,
        request: &BuildingPreviewRequest,
    ) -> Result<BuildingPlacement, OrderError> {
        self.plan_building(side, request, self.tick)
            .map(|p| p.placement)
    }

    fn plan_building(
        &self,
        side: Side,
        request: &BuildingPreviewRequest,
        planning_tick: Tick,
    ) -> Result<garrison::BuildingPlan, OrderError> {
        self.validate_units(side, &request.units)?;
        if request.facing.is_some_and(|f| !f.is_finite()) {
            return Err(OrderError::OutOfBounds);
        }
        let source = self.move_source(side);
        // Live admission precedes its application tick; replay and scripts
        // already entered that tick. Timed prefix prediction must start from
        // the same prior-state tick in all three paths.
        let mut ctx = self.movement_context();
        ctx.tick = planning_tick;
        garrison::occupy(&ctx, &self.sides[side.index()], &source, request)
    }

    fn move_source(&self, side: Side) -> Vec<Unit> {
        let mut source = self.units.clone();
        for unit in &mut source {
            if unit.side != side {
                let track = self.knowledge[side.index()].track(unit.id);
                for (k, soldier) in unit.members.iter_mut().enumerate() {
                    let seen =
                        track.is_some_and(|t| t.last_seen == self.tick && t.members.contains(&k));
                    soldier.hp = if seen { 1.0 } else { 0.0 };
                    if !seen {
                        soldier.position = track.map_or(Default::default(), |t| t.position);
                    }
                    soldier.path.clear();
                    soldier.spot = None;
                    soldier.post = None;
                    soldier.lean = None;
                    soldier.cover = None;
                }
                unit.cover = Default::default();
                unit.attackers.clear();
                unit.suppression = 0.0;
                for mount in &mut unit.mounts {
                    mount.stop();
                }
                if let Some(track) = track {
                    unit.position = track.position;
                    unit.yaw = track.yaw;
                    unit.hp = 1.0;
                } else {
                    unit.hp = 0.0;
                }
            }
        }
        // Admission sees earlier commands exactly as execution will apply them.
        for pending in &self.pending {
            if pending.command.side == side {
                self.apply_units(pending.clone(), &mut source);
            }
        }
        source
    }

    fn movement_context(&self) -> MovementContext<'_> {
        MovementContext {
            world: &self.world,
            roads: &self.roads,
            ground: &self.ground,
            ground_rules: &self.rules.ground,
            authored: self.authored_props,
            tick: self.tick,
            tick_hz: self.rules.tick_hz,
            infantry: &self.rules.infantry_movement,
            soldier_radius_m: self.rules.physics.soldier_radius_m,
            seed: self.seed,
            rules: &self.rules,
            knowledge: [&self.knowledge[0], &self.knowledge[1]],
            arsenal: &self.arsenal,
        }
    }

    /// A soldier's resolved place and cover (D2+): his spot while moving, his
    /// post while holding, else where he stands; the cover he has now against
    /// his squad's threat, in the true world as rounds meet it, and the tier
    /// his place was resolved to give. A garrison shelters instead (Q22).
    fn member_order(
        world: &WorldGeometry,
        ground: &GroundLayer,
        rules: &Rules,
        u: &Unit,
        s: &Soldier,
        hulls: &[cover::Body],
    ) -> MemberOrder {
        let here = s.position.xy();
        let spot = s.spot.or(s.post).unwrap_or(here);
        let sheltered = u.garrison.is_some();
        let cover_now = u
            .cover
            .threat
            .filter(|_| !sheltered)
            .and_then(|t| cover::at(world, ground, hulls, rules, here, t));
        MemberOrder {
            spot: [spot.x, spot.y],
            cover_now,
            cover_there: s.cover.filter(|_| !sheltered),
        }
    }

    fn observe_all(&mut self) {
        // Every live hull a soldier's current cover may lie behind (D2+).
        let hulls = cover::hull_bodies(&crate::lean::hulls(&self.units, &self.rules));
        for side in Side::ALL {
            let knowledge = &self.knowledge[side.index()];
            let spotted_by_visible_enemy: BTreeSet<UnitId> = self.sightings[1 - side.index()]
                .iter()
                .filter(|s| {
                    knowledge
                        .track(s.observer)
                        .is_some_and(|t| t.last_seen == self.tick)
                })
                .map(|s| s.target)
                .collect();
            let fog = &self.fog[side.index()];
            let frame = &mut self.observations[side.index()];
            frame.tick = self.tick;
            frame.identified.clear();
            frame
                .identified
                .extend(knowledge.identified(self.tick, &self.units, &self.rules));
            frame.ground_visibility.clone_from(fog);
            frame.contacts.clear();
            frame.contacts.extend(knowledge.contacts());
            frame.audible.clone_from(&self.audible[side.index()]);
            frame.known_props.clear();
            let known = &self.sides[side.index()];
            // Planning and publication share the complete remembered pose.
            let remembered = known.seen.keys().filter_map(|&id| {
                let p = known.prop(&self.world, self.authored_props, id)?;
                let replaces = if id < self.authored_props {
                    Some(id)
                } else {
                    self.structures.replaced_by(id)
                };
                Some((p, replaces, false))
            });
            // A learned removal without a replacement only hides its authored body.
            let removed = self
                .structures
                .removed()
                .filter(|p| p.id < self.authored_props && !known.standing().contains_key(&p.id))
                .map(|p| (p.clone(), Some(p.id), true));
            for (p, replaces, destroyed) in remembered.chain(removed) {
                let building = self.world.building_of(p.id);
                frame.known_props.push(KnownProp {
                    kind: p.kind,
                    center: [p.center.x, p.center.y],
                    yaw: p.yaw,
                    half_extents: [p.half.x, p.half.y, p.half.z],
                    base_z: p.base_z,
                    replaces,
                    destroyed,
                    id: p.id,
                    building,
                    structure_owner: building
                        .filter(|_| !destroyed)
                        .map(|_| self.world.remembered_structure_owner(p.id)),
                    authored_prop: self.world.authored_prop(p.id),
                    wreck_of: p.wreck_of,
                });
            }
            frame.projectiles.clear();
            for round in &self.segments {
                if round.side == side {
                    frame.projectiles.push(round.whole());
                } else {
                    frame.projectiles.extend(clip_to_seen(fog, round));
                }
            }
            frame.blasts.clear();
            frame.blasts.extend(
                self.blasts
                    .iter()
                    .filter(|(by, b)| *by == side || fog.visible(b.point[0], b.point[1]))
                    .map(|(_, b)| b.clone()),
            );
            let target_ref = |t: Target| -> Option<TargetRef> {
                match t {
                    Target::Unit(u) => knowledge
                        .track(u)
                        .map(|tr| TargetRef::Identified { id: tr.id }),
                    Target::Contact(c) => Some(TargetRef::Contact { id: c }),
                    Target::Ground(p) => Some(TargetRef::Ground {
                        point: [p.x, p.y, p.z],
                    }),
                }
            };
            frame.own.clear();
            frame.own.extend(
                self.units
                    .iter()
                    .filter(|u| u.side == side && u.alive())
                    .map(|u| OwnUnit {
                        withdrawing: u.withdrawing,
                        protection: u.protection.as_ref().map(|p| {
                            let definition = u
                                .unit_type(&self.rules)
                                .capabilities
                                .active_protection
                                .as_ref()
                                .unwrap();
                            let remaining = (p.ready_at_tick - self.tick as f64).max(0.0);
                            contract::observation::ProtectionReadiness {
                                charges: p.charges,
                                cooldown: (remaining > 0.0).then(|| {
                                    (1.0 - remaining
                                        / (definition.cooldown_s * f64::from(self.rules.tick_hz)))
                                    .clamp(0.0, 1.0)
                                }),
                            }
                        }),
                        id: u.id,
                        kind: u.kind,
                        position: [u.position.x, u.position.y, u.position.z],
                        yaw: u.yaw,
                        goal: u.movement_goal().map(|(g, _)| [g.x, g.y]),
                        policy: u.movement_goal().map(|(_, p)| p),
                        direction: u.movement_goal().map(|_| u.direction()),
                        reversing: u.reversing,
                        state: u.state,
                        // An enemy it waits for is named only if the side
                        // could name it anyway (Q14: contact reveals nothing).
                        blocker: u.blocker.filter(|b| self.units[b.0 as usize].side == side),
                        route: u.route.iter().flatten().map(|p| [p.x, p.y]).collect(),
                        queue: u
                            .orders
                            .iter()
                            .skip(1)
                            .filter_map(|o| {
                                o.movement().map(|m| [m.destination.x, m.destination.y])
                            })
                            .collect(),
                        members: u
                            .members
                            .iter()
                            .enumerate()
                            .filter(|(_, s)| s.alive())
                            .map(|(k, _)| {
                                let p = garrison::body_position(u, k, &self.rules);
                                [p.x, p.y, p.z]
                            })
                            .collect(),
                        member_ids: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| s.id)
                            .collect(),
                        member_slots: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| s.slot as u8)
                            .collect(),
                        member_active_mounts: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| s.active_mount.map(|m| m as u8))
                            .collect(),
                        member_orders: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| {
                                Self::member_order(
                                    &self.world,
                                    &self.ground,
                                    &self.rules,
                                    u,
                                    s,
                                    &hulls,
                                )
                            })
                            .collect(),
                        member_leans: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| s.leaning(self.tick).map(|l| l.published()))
                            .collect(),
                        area: u.anchor.map(|a| SquadArea {
                            anchor: [a.at.x, a.at.y],
                            radius: crate::cover::area_radius(&self.rules, u),
                        }),
                        final_facing: movement::final_facing(u),
                        sees: knowledge.own_sensor(u.id),
                        engagement: u.engagement,
                        mounts: u
                            .mounts
                            .iter()
                            .map(|m| {
                                weapons::readiness(
                                    &self.arsenal,
                                    u,
                                    m,
                                    m.lock.as_ref().and_then(|l| target_ref(l.target)),
                                )
                            })
                            .collect(),
                        weapon_poses: u.mounts.iter().map(weapons::pose).collect(),
                        deployment: deployment::state(u),
                        hp: u.hp,
                        member_hp: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| s.hp)
                            .collect(),
                        suppression: self.rules.suppression.tier(u.suppression),
                        concealed: sensing::concealed(&self.world, u, &self.rules)
                            && !spotted_by_visible_enemy.contains(&u.id)
                            && u.attackers.is_empty(),
                        stock: u.stock,
                        service: u.service,
                        garrison: garrison::state(&self.world, u, &self.rules),
                        sight: {
                            let s = sight::of(u, &self.rules);
                            UnitSight {
                                eyes: sensing::eyes(u, &self.rules)
                                    .into_iter()
                                    .map(|e| [e.x, e.y, e.z])
                                    .collect(),
                                forward: s.forward,
                                shape: s.shape,
                                range: s.range,
                            }
                        },
                    }),
            );
            frame.encounter = self.encounter;
            frame.skirmish = self.skirmish.as_ref().map(|s| {
                s.view(
                    side,
                    self.tick,
                    self.rules.tick_hz,
                    self.units
                        .iter()
                        .filter(|u| u.side == side && u.alive())
                        .count() as u32,
                )
            });
            frame.guided.clear();
            frame
                .guided
                .extend(self.projectiles.active().iter().filter_map(|p| {
                    let own = self.rounds.get(&p.id).is_some_and(|r| r.side == side);
                    p.guidance.filter(|_| own).map(|g| GuidedMissile {
                        id: p.id.0,
                        position: [p.position.x, p.position.y, p.position.z],
                        point: [g.point.x, g.point.y, g.point.z],
                        supported: g.supported,
                    })
                }));
            frame.corpses.clear();
            for u in &self.units {
                for s in &u.members {
                    let own = u.side == side;
                    let fallen = s.corpse.and_then(|actual| {
                        if own {
                            Some(actual)
                        } else {
                            knowledge.corpse(s.id, actual)
                        }
                    });
                    if let Some(f) = fallen {
                        frame.corpses.push(Corpse {
                            position: [f.at.x, f.at.y, f.at.z],
                            own,
                            soldier: s.id,
                            kind: u.kind,
                            slot: s.slot as u8,
                            yaw: f.yaw,
                        });
                    }
                }
            }
            // By place, so the list never groups enemy fallen by their hidden unit.
            frame.corpses.sort_by(|a, b| {
                (a.position[0], a.position[1])
                    .partial_cmp(&(b.position[0], b.position[1]))
                    .unwrap()
            });
            frame.fallen_bodies.clear();
            frame.fallen_bodies.extend(
                self.fallen
                    .values()
                    .filter(|f| knowledge.knows_fallen(f.prop))
                    .cloned(),
            );
        }
    }

    pub fn observe(&self, side: Side) -> &ObservationFrame {
        &self.observations[side.index()]
    }

    /// Digest of the complete authoritative state at the current tick.
    pub fn digest(&self) -> u64 {
        let mut d = Digest::default();
        d.u64(self.tick);
        if let Some(skirmish) = &self.skirmish {
            skirmish.digest(&mut d);
        }
        d.u64(self.units.len() as u64);
        for u in &self.units {
            u.digest(&mut d);
        }
        d.u64(self.last_soldier as u64);
        for knowledge in &self.knowledge {
            knowledge.digest(&mut d);
        }
        for sightings in &self.sightings {
            d.u64(sightings.len() as u64);
            for s in sightings {
                d.u64(s.observer.0 as u64)
                    .u64(s.target.0 as u64)
                    .u64(s.members.len() as u64);
                for &k in &s.members {
                    d.u64(k as u64);
                }
            }
        }
        d.u64(self.fired.len() as u64);
        for id in &self.fired {
            d.u64(id.0 as u64);
        }
        for side in &self.sides {
            side.digest(&mut d);
        }
        self.planner.digest(&mut d);
        d.u64(self.world.obstacle_revision());
        // Every body's pose (L3): shoves move them.
        self.world.digest_props(&mut d);
        for (id, t) in self.world.moved() {
            d.u64(id as u64).u64(t);
        }
        self.world.digest_cleared(&mut d);
        self.world.digest_buildings(&mut d);
        d.u64(self.expiries.len() as u64);
        for (id, t) in &self.expiries {
            d.u64(*id as u64).u64(*t);
        }
        self.structures.digest(&mut d);
        // An empty fall log folds nothing: a battle where nothing topples
        // digests as if there were no log.
        if !self.fallen.is_empty() {
            d.u64(self.fallen.len() as u64);
            for f in self.fallen.values() {
                d.u64(u64::from(f.prop))
                    .f64(f.at[0])
                    .f64(f.at[1])
                    .f64(f.toward[0])
                    .f64(f.toward[1])
                    .u64(f.tick);
            }
        }
        self.ground.digest(&mut d);
        self.projectiles.digest(&mut d);
        d.u64(self.rounds.len() as u64);
        for (id, r) in &self.rounds {
            d.u64(id.0)
                .u64(r.weapon as u64)
                .u64(r.unit.0 as u64)
                .u64(r.side.index() as u64);
        }
        d.u64(self.suppressed.len() as u64);
        for ((id, i), v) in &self.suppressed {
            d.u64(id.0).u64(*i as u64).f64(*v);
        }
        // Command sequencing (next_seq, accepted, pending) is input
        // bookkeeping, left out: live play accepts a command a tick before a
        // replay admits it, and its effect is the applied orders above.
        d.u64(self.combat_rng.state())
            .u64(self.damage_rng.state())
            .u64(self.ricochet_rng.state());
        // The defender's memory is a player's, not the battle's: its effect is
        // its accepted commands, and a replay runs without it.

        if let Some((_, referee)) = &self.referee {
            referee.digest(&mut d);
        }
        d.finish()
    }

    pub fn replay(&self) -> Replay {
        Replay {
            engine_build: ENGINE_BUILD_ID.to_owned(),
            scenario_digest: format!("{:016x}", self.scenario_digest),
            config_digest: format!("{:016x}", self.config_digest),
            seed: self.seed,
            accepted: self.accepted.clone(),
        }
    }
}

#[derive(Clone)]
struct PreparedCommand {
    command: CommandEnvelope,
    placement: Option<MovePlacement>,
    building: Option<garrison::BuildingPlan>,
    /// Where a placed unit stops short ([`crate::units::StopShort`]).
    stops: BTreeMap<UnitId, crate::units::StopShort>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A battle that changes what each side knows in every way a battle
    /// can: a tank shoves a crate and knocks a lane through a wood, shells
    /// fell trees, break a wall into rubble and a wreck into a lighter
    /// wreck, and a wreck appears; one side watches from close by, the
    /// other learns of each only as it comes into view. Whatever a side
    /// believes at any tick, its planning grid, kept up body by body, is
    /// the grid built whole from those beliefs.
    #[test]
    fn each_sides_grid_stays_the_whole_build_of_what_it_believes() {
        let burst = |tick: u64, at: [f64; 2]| serde_json::json!({ "tick": tick, "burst": { "point": at, "weapon": "tank_he" } });
        let mut events = vec![serde_json::json!({ "tick": 60, "add_prop": {
            "kind": "light_wreck", "center": [150, 30], "yaw": 0.4, "half_extents": [2.2, 1.0, 0.9],
            "wreck_of": "test_jeep" } })];
        for round in 0..12 {
            let tick = 20 + 25 * round;
            events.push(burst(tick, [150.0 + 4.0 * round as f64, 80.0]));
            events.push(burst(tick + 3, [232.0, 60.0]));
            if round < 5 {
                events.push(burst(tick + 6, [236.0, 140.0]));
            }
        }
        // A row of crates too close together for a tank to pass between.
        let mut props: Vec<serde_json::Value> = (0..13)
            .map(|k| {
                serde_json::json!({ "kind": "crate", "center": [70.0, 79.0 + 3.5 * k as f64],
                    "yaw": 0, "half_extents": [0.8, 0.8, 0.6] })
            })
            .collect();
        props.push(
            serde_json::json!({ "kind": "wall", "center": [232, 60], "yaw": 0,
            "half_extents": [0.4, 10, 1] }),
        );
        props.push(
            serde_json::json!({ "kind": "heavy_wreck", "center": [236, 140], "yaw": 0.3,
            "half_extents": [3, 1.5, 1], "wreck_of": "test_tank" }),
        );
        let (wall, wreck) = (13, 14);
        let drive = |side: &str, unit: u32, goal: [f64; 2]| {
            serde_json::json!({ "tick": 1, "side": side, "order": { "kind": "move",
                "units": [unit], "gesture": unit + 1, "goal": goal, "route": "shortest" } })
        };
        let setup: ScenarioDefinition = serde_json::from_value(serde_json::json!({
            "map": { "size": [900, 200], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                "forests": [{ "shape": { "kind": "polygon",
                    "ring": [[100, 50], [200, 50], [200, 150], [100, 150]] } }],
                "props": props },
            "rules": crate::fixtures::test_game(),
            "units": [
                { "side": "blue", "kind": "test_tank", "position": [40, 100], "yaw": 0.0,
                  "engagement": "return_fire_only" },
                { "side": "blue", "kind": "test_jeep", "position": [40, 30], "yaw": 0.0,
                  "engagement": "return_fire_only" },
                { "side": "red", "kind": "test_rifle", "position": [860, 100],
                  "engagement": "return_fire_only" },
                { "side": "red", "kind": "test_jeep", "position": [880, 180], "yaw": 3.1,
                  "engagement": "return_fire_only" }
            ],
            "events": events,
            "scripts": [
                drive("blue", 0, [290.0, 100.0]),
                drive("blue", 1, [300.0, 30.0]),
                drive("red", 3, [30.0, 180.0])
            ]
        }))
        .expect("a scenario");
        let mut battle = Battle::new(&setup, 3);
        let radius = battle.rules.physics.soldier_radius_m;
        for tick in 1..=1200 {
            battle.step();
            if tick % 20 != 0 {
                continue;
            }
            for side in Side::ALL {
                let (kept, whole) = battle.sides[side.index()].grid_beside_whole_build(
                    &battle.world,
                    battle.authored_props,
                    radius,
                );
                kept.assert_same(&format!("tick {tick}, {side:?}"), &whole);
            }
        }
        // The battle did change what the sides know, in each of the ways.
        let world = &battle.world;
        assert!(world.cleared_cells() > 0, "a lane was cleared");
        assert!(world.moved().next().is_some(), "a crate was shoved");
        assert!(world.prop(wall).is_none(), "the wall fell");
        assert!(world.prop(wreck).is_none(), "the wreck broke up");
        assert!(
            world.props().any(|p| p.known_to_all),
            "remains both sides plan with"
        );
        for side in &battle.sides {
            assert!(side.revision > 0);
            assert!(!side.seen.is_empty());
        }
        assert!(battle.load().grid_cells_relaid > 0);
    }

    #[test]
    fn an_enemy_round_is_clipped_to_seen_ground_leg_by_leg_keeping_its_ricochets() {
        // 8 m cells, row 0 seen except column 2 (x 16–24).
        let seen: u32 = (0..10).filter(|&i| i != 2).map(|i| 1 << i).sum();
        let fog = VisibilityField {
            cell_m: 8.0,
            nx: 10,
            ny: 2,
            bits: vec![seen],
        };
        let normal = v3(-1.0, 0.0, 0.0);
        let round = Flown {
            side: Side::Blue,
            from: v3(2.0, 4.0, 1.0),
            ricochets: vec![(v3(40.0, 4.0, 1.0), normal)],
            to: v3(60.0, 4.0, 1.0),
            weapon: 0,
            shooter: None,
            hit: Some((SegmentHit::Ground, v3(0.0, 0.0, 1.0))),
        };
        let pieces = clip_to_seen(&fog, &round);
        let paths: Vec<Vec<[f64; 3]>> = pieces.iter().map(|p| p.path.clone()).collect();
        // The first leg samples every 4.75 m: seen to 11.5, unseen at 16.25
        // and 21, seen again from 25.75 through the ricochet to the end.
        assert_eq!(
            paths,
            [
                vec![[2.0, 4.0, 1.0], [11.5, 4.0, 1.0]],
                vec![[25.75, 4.0, 1.0], [40.0, 4.0, 1.0], [60.0, 4.0, 1.0]],
            ]
        );
        assert!(pieces[0].ricochets.is_empty());
        assert_eq!(pieces[0].hit, SegmentHit::None);
        assert_eq!(
            pieces[1].ricochets,
            [SegmentRicochet {
                point: 1,
                normal: [-1.0, 0.0, 0.0]
            }]
        );
        assert_eq!(pieces[1].hit, SegmentHit::Ground, "its end is seen");
        assert!(pieces.iter().all(|p| !p.own && p.kind == 0));
    }
    #[test]
    fn seeing_one_remain_clears_far_historical_building_snapshots() {
        let mut descriptor: contract::templates::BuildingTemplateDescriptor = serde_json::from_str(
            include_str!("../../../fixtures/parity/templates/asymmetric.json"),
        )
        .unwrap();
        descriptor.parts[0].half_extents[0] = 50.0;
        descriptor.parts[1].center[0] = 52.0;
        for edge in &mut descriptor.edges {
            if edge.id == "main-north" || edge.id == "main-south" {
                edge.span_m = [-50.0, 50.0];
            }
        }
        let catalogue =
            contract::templates::TemplateGeometryCatalog::new(vec![descriptor.clone()]).unwrap();
        let geometry = descriptor
            .materialize(contract::templates::PlacementFrame {
                translation: [400.0, 300.0, 0.0],
                yaw: 0.0,
            })
            .unwrap();
        let mut rules = crate::fixtures::test_game();
        crate::fixtures::patch_catalog(
            &mut rules,
            "units",
            "test_rifle",
            serde_json::json!({"sensors":{"ground_m":30}}),
        );
        let setup = serde_json::from_value(serde_json::json!({
            "map":{"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
                "template_catalog_hash":catalogue.hash(),"regional_family":descriptor.regional_family,
                "buildings":[{"owner":0,"kind":"building","category":descriptor.category,"regional_family":descriptor.regional_family,"parts":[{"part":"main","prop":0},{"part":"wing","prop":1}],"geometry":geometry}]},
            "rules":rules,"units":[{"side":"blue","kind":"test_rifle","position":[250,100],"engagement":"return_fire_only"}],"events":[],"scripts":[]
        })).unwrap();
        let mut battle = Battle::new(&setup, 11);
        battle.destroy_prop(0, v2(1.0, 0.0));
        assert!(
            battle.sides[0].standing().contains_key(&1),
            "unseen wing remains remembered"
        );
        let delta = crate::math::v3(78.0, 200.0, 0.0);
        battle.units[0].position = battle.units[0].position + delta;
        for member in &mut battle.units[0].members {
            member.position = member.position + delta;
        }
        battle.sweep_fog(Side::Blue);
        assert!(
            !battle.fog[0].visible(452.0, 301.0),
            "far wing stays fogged"
        );
        assert!(!battle.sides[0].standing().contains_key(&0));
        assert!(
            !battle.sides[0].standing().contains_key(&1),
            "whole aggregate's historical snapshots are cleared"
        );
    }
    #[test]
    fn corpse_support_loss_is_physical_but_enemy_memory_waits_for_sight() {
        for whole_squad in [false, true] {
            let descriptor: contract::templates::BuildingTemplateDescriptor = serde_json::from_str(
                include_str!("../../../fixtures/parity/templates/asymmetric.json"),
            )
            .unwrap();
            let catalogue =
                contract::templates::TemplateGeometryCatalog::new(vec![descriptor.clone()])
                    .unwrap();
            let mut geometry = descriptor
                .materialize(contract::templates::PlacementFrame {
                    translation: [400.0, 300.0, 0.0],
                    yaw: 0.0,
                })
                .unwrap();
            geometry.floor_z = Some(vec![0.0, 3.0, 6.0]);
            let mut rules = crate::fixtures::test_game();
            crate::fixtures::patch_catalog(
                &mut rules,
                "units",
                "test_rifle",
                serde_json::json!({"sensors":{"ground_m":60}}),
            );
            rules["weapons"]["rifle"]["damage"] = serde_json::json!(1e6);
            rules["garrison"]["survival_probability_on_collapse"] = serde_json::json!(0);
            let setup = serde_json::from_value(serde_json::json!({
                "map":{"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
                    "template_catalog_hash":catalogue.hash(),"regional_family":descriptor.regional_family,
                    "buildings":[{"owner":0,"kind":"building","category":descriptor.category,"regional_family":descriptor.regional_family,
                        "parts":[{"part":"main","prop":0},{"part":"wing","prop":1}],"geometry":geometry}]},
                "rules":rules,"units":[
                    {"side":"red","kind":"test_rifle","position":[380,300],"engagement":"return_fire_only"},
                    {"side":"blue","kind":"test_rifle","position":[350,300],"engagement":"return_fire_only"}
                ],"events":[],"scripts":[]
            })).unwrap();
            let mut battle = Battle::new(&setup, 11);
            let ack = battle.accept(CommandEnvelope {
                side: Side::Red,
                seq: 1,
                queued: false,
                order: Order::Garrison {
                    units: vec![UnitId(0)],
                    building: 0,
                },
            });
            assert_eq!(ack.error, None);
            for _ in 0..300 {
                battle.step();
            }
            assert!(battle.units[0].garrisoned());
            let upper = battle.units[0]
                .members
                .iter()
                .find(|s| s.position.z == 6.0 && battle.fog[0].visible(s.position.x, s.position.y))
                .unwrap()
                .clone();
            let targets: Vec<_> = battle.units[0]
                .members
                .iter()
                .filter(|s| whole_squad || s.id == upper.id)
                .map(|s| (s.id, s.position))
                .collect();
            let weapon = battle
                .arsenal
                .weapons
                .iter()
                .position(|w| w.id == "rifle")
                .unwrap();
            // Feed physical hit events at damage's boundary: stochastic aiming
            // is not the support or knowledge contract under test.
            let mut rounds = BTreeMap::new();
            let events: Vec<_> = targets
                .iter()
                .enumerate()
                .map(|(i, (id, at))| {
                    let projectile = ProjectileId(i as u64);
                    rounds.insert(
                        projectile,
                        Round {
                            weapon,
                            unit: UnitId(1),
                            side: Side::Blue,
                        },
                    );
                    FlightEvent::Impact(flight::Impact {
                        projectile,
                        struck: flight::Struck::Body(flight::BodyId(*id)),
                        point: *at,
                        normal: v3(1.0, 0.0, 0.0),
                        velocity: v3(100.0, 0.0, 0.0),
                        time: 0.0,
                        bounces: 0,
                        pose: None,
                        detonated: false,
                    })
                })
                .collect();
            damage::resolve(
                &DamageContext {
                    world: &battle.world,
                    ground: &battle.ground,
                    arsenal: &battle.arsenal,
                    rules: &battle.rules,
                    tick: battle.tick,
                },
                &events,
                &rounds,
                &mut battle.suppressed,
                &mut battle.units,
                &mut battle.damage_rng,
            );
            assert_eq!(battle.units[0].alive(), !whole_squad);
            assert_eq!(battle.units[0].garrison.is_none(), whole_squad);
            battle.sweep_fog(Side::Blue);
            battle.observe_all();
            let remembered = battle
                .observe(Side::Blue)
                .corpses
                .iter()
                .find(|f| f.soldier == upper.id)
                .unwrap()
                .clone();
            assert_eq!(remembered.position[2], 6.0);
            let delta = v3(-300.0, -250.0, 0.0);
            battle.units[1].position = battle.units[1].position + delta;
            for member in &mut battle.units[1].members {
                member.position = member.position + delta;
            }
            battle.sweep_fog(Side::Blue);
            assert!(!battle.fog[0].visible(remembered.position[0], remembered.position[1]));
            battle.destroy_prop(0, v2(1.0, 0.0));
            battle.observe_all();
            let physical = battle
                .observe(Side::Red)
                .corpses
                .iter()
                .find(|f| f.soldier == upper.id)
                .unwrap();
            let z = battle
                .world
                .surface_at(remembered.position[0], remembered.position[1])
                .unwrap()
                .z;
            assert_eq!(
                physical.position,
                [remembered.position[0], remembered.position[1], z]
            );
            assert_eq!(
                battle
                    .observe(Side::Blue)
                    .corpses
                    .iter()
                    .find(|f| f.soldier == upper.id)
                    .unwrap()
                    .position,
                remembered.position,
                "an unseen collapse cannot move a remembered enemy corpse"
            );
            let mut unchanged = battle.damage_rng.clone();
            battle.destroy_prop(0, v2(1.0, 0.0));
            assert_eq!(
                battle.damage_rng.unit(),
                unchanged.unit(),
                "no second survival roll"
            );
            battle.units[1].position = battle.units[1].position - delta;
            for member in &mut battle.units[1].members {
                member.position = member.position - delta;
            }
            battle.sweep_fog(Side::Blue);
            assert!(battle.fog[0].visible(remembered.position[0], remembered.position[1]));
            battle.observe_all();
            assert_eq!(
                battle
                    .observe(Side::Blue)
                    .corpses
                    .iter()
                    .find(|f| f.soldier == upper.id)
                    .unwrap()
                    .position,
                [remembered.position[0], remembered.position[1], z]
            );
        }
    }
}

#[cfg(test)]
mod settlement_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn hostile_collapse_uses_first_destructive_source_and_original_receipt() {
        let mut rules = crate::fixtures::test_game();
        for kind in ["test_tank", "test_rifle"] {
            crate::fixtures::patch_catalog(
                &mut rules,
                "units",
                kind,
                json!({"sensors":{"ground_m":1}}),
            );
        }
        crate::fixtures::patch_catalog(&mut rules, "units", "test_rifle", json!({"cost":900}));
        rules["garrison"]["survival_probability_on_collapse"] = json!(0);
        let setup: ScenarioDefinition = serde_json::from_value(json!({
            "map":rules["map"],"rules":rules,"units":[
                {"side":"blue","kind":"test_tank","position":[50,50]},
                {"side":"red","kind":"test_rifle","position":[700,550]}
            ]
        }))
        .unwrap();
        let mut battle = Battle::new(&setup, 7);
        let building = battle.world.props().find(|p| p.body.garrison).unwrap().id;
        let center = battle.world.prop(building).unwrap().center;
        battle.units[1].position = center.with_z(0.0);
        for member in &mut battle.units[1].members {
            member.position = center.with_z(0.0);
        }
        battle.units[1].garrison = Some(garrison::Garrison {
            building,
            phase: garrison::Phase::Inside,
            slots: Vec::new(),
            seats: vec![None; battle.units[1].members.len()],
            entry: center,
        });
        battle.skirmish = Some(crate::skirmish::Skirmish::new(serde_json::from_value(json!({
            "factions":["us","eastern"],
            "rules":{"credits_per_minute":200,"starting_minutes":5,"preparation_s":60,"max_units":30,"dispatch_interval_s":1},
            "sites":{"entries":[{"side":"blue","center":[400,10],"yaw":0},{"side":"red","center":[400,590],"yaw":0}],
                "objectives":[{"id":"center","center":[400,300],"radius_m":50,"kind":"junction","counterpart":null},
                    {"id":"south","center":[200,200],"radius_m":50,"kind":"field","counterpart":"north"},
                    {"id":"north","center":[600,400],"radius_m":50,"kind":"field","counterpart":"south"}]}
        })).unwrap()));
        battle.skirmish.as_mut().unwrap().entered.insert(
            UnitId(1),
            crate::skirmish::EnteredUnit {
                price: 200,
                entered_tick: 0,
            },
        );
        battle.tick = 1;
        battle.observe_all();
        let before = battle.observe(Side::Blue).clone();
        let source = Some(damage::LethalSource {
            unit: UnitId(0),
            side: Side::Blue,
        });
        battle.consequences(damage::Outcome {
            destroyed: Vec::new(),
            attacked: Vec::new(),
            structural: vec![
                damage::StructuralHit {
                    prop: building,
                    amount: 1e12,
                    toward: v2(1.0, 0.0),
                    source,
                },
                damage::StructuralHit {
                    prop: building,
                    amount: 1e12,
                    toward: v2(1.0, 0.0),
                    source: None,
                },
            ],
        });
        assert!(!battle.units[1].alive());
        battle.observe_all();
        let after = battle.observe(Side::Blue);
        assert_eq!(after.skirmish.as_ref().unwrap().credits, 1050.0);
        assert_eq!(after.identified, before.identified);
        assert_eq!(after.contacts, before.contacts);
        assert_eq!(after.known_props, before.known_props);
        assert_eq!(after.corpses, before.corpses);
        assert_eq!(
            battle.skirmish.as_ref().unwrap().settlement.pools(),
            [100_000_000, 0]
        );
    }
}
