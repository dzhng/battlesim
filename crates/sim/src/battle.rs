//! The one battle authority: commands in, fixed ticks, side observations out.
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use contract::catalog::Destroyed;
use contract::command::{CommandAck, CommandEnvelope, Engagement, Order, OrderError, TargetRef};
use contract::ids::{Side, Tick, UnitId};
use contract::map::{MoverClass, PropDefinition};
use contract::observation::{
    Blast, Corpse, EncounterStatus, GuidedMissile, KnownProp, MemberOrder, MoveState,
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
use crate::flight::{
    self, Body, BodyId, FlightEvent, Pose, ProjectileId, Projectiles, Shape, Struck,
};
use crate::garrison;
use crate::ground::{self, GroundLayer, KnownGround, Wear};
use crate::hearing;
use crate::knowledge::SideKnowledge;
use crate::math::{v2, v3, Obb2, V2, V3};
use crate::movement::{self, MovementContext, SideGeometry};
use crate::rng::Rng;
use crate::sensing::{self, Sighting};
use crate::sight;
use crate::structures::Structures;
use crate::supply;
use crate::units::{self, MoveOrder, Soldier, Unit, UnitOrder};
use crate::village::{Defender, Referee};
use crate::visibility::{self, OcclusionGrid};
use crate::weapons::{self, Arsenal, FireContext, Support, Target, VEHICLE_BODY_BASE};
use crate::world::{PropId, WorldGeometry};

/// Group offsets are compressed to fit within this radius of the goal.
const GROUP_SPREAD_M: f64 = 40.0;
/// How far a group member's destination may move to find standing room.
const DESTINATION_SNAP_M: f64 = 16.0;
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
    /// Bytes the ground layer holds.
    pub ground_bytes: usize,
    /// Bytes both sides' learned copies of it hold.
    pub known_ground_bytes: usize,
}

/// Everything needed to reproduce a battle in the same build: the setup
/// identity, the seed and every sequenced command from both sides.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Replay {
    pub scenario_digest: String,
    pub config_digest: String,
    pub seed: u64,
    /// `(applied_tick, command)` in acceptance order.
    pub accepted: Vec<(Tick, CommandEnvelope)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReplayError {
    ScenarioMismatch,
    ConfigMismatch,
}

pub struct Battle {
    world: WorldGeometry,
    rules: Rules,
    seed: u64,
    tick: Tick,
    units: Vec<Unit>,
    /// Props authored with the map (ids below this) are known to every side.
    authored_props: PropId,
    sides: [SideGeometry; 2],
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
    /// The observation-bound opponent and its memory (off while replaying).
    opponent: Option<(Opponent, Defender)>,
    /// The fixture's completion referee and its latest verdict.
    referee: Option<(EncounterRules, Referee)>,
    encounter: Option<EncounterStatus>,
    pending: Vec<CommandEnvelope>,
    accepted: Vec<(Tick, CommandEnvelope)>,
    /// Recorded commands still to apply when this battle is a replay.
    replaying: Option<VecDeque<(Tick, CommandEnvelope)>>,
    observations: [ObservationFrame; 2],
    scenario_digest: u64,
    config_digest: u64,
    /// Transient bodies (a row with a `lifetime_s`) and the tick each goes.
    expiries: BTreeMap<PropId, Tick>,
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
            let p = prop.center + local.rotated(prop.yaw);
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
    let n = unit.members.len();
    for k in n.saturating_sub(c.casualties as usize)..n {
        let at = unit.members[k].position;
        unit.members[k].fall(at, unit.yaw);
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

fn scenario_digest(setup: &ScenarioDefinition) -> u64 {
    let map = serde_json::to_string(&setup.map).expect("map serializes");
    let units = serde_json::to_string(&setup.units).expect("units serialize");
    let events = serde_json::to_string(&setup.events).expect("events serialize");
    let scripts = serde_json::to_string(&setup.scripts).expect("scripts serialize");
    // The opponent's policy and the encounter's rules change what a replay's
    // commands mean, so a replay is pinned to them too.
    let opponent = serde_json::to_string(&setup.opponent).expect("opponent serializes");
    let encounter = serde_json::to_string(&setup.encounter).expect("encounter serializes");
    digest::of_str(&(map + &units + &events + &scripts + &opponent + &encounter))
}

fn config_digest(setup: &ScenarioDefinition) -> u64 {
    digest::of_str(&serde_json::to_string(&setup.rules).expect("rules serialize"))
}

impl Battle {
    pub fn new(setup: &ScenarioDefinition, seed: u64) -> Self {
        let rules = setup.rules.clone();
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
        let world = WorldGeometry::new(&setup.map, &rules);
        let arsenal = Arsenal::new(&rules);
        supply::validate(&arsenal, &rules);
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
            .map(|(i, u)| {
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
                    &world,
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
                    soldier_ids += 1;
                    let z = world.surface_at(p.x, p.y).map_or(0.0, |s| s.z);
                    let hp = rules.catalog.soldier(&slots[slot]).hp;
                    Soldier::new(soldier_ids, slot, p.with_z(z), hp)
                })
                .collect();
                let mounts = arsenal.mounts_for(kind, u.yaw, &members);
                Unit {
                    id: UnitId(i as u32),
                    side: u.side,
                    kind,
                    position: xy.with_z(world.surface_at(xy.x, xy.y).map_or(0.0, |s| s.z)),
                    yaw: u.yaw,
                    mobility: units::mobility(t, &rules),
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
                    pursuit: None,
                    planned_goal: None,
                    engagement: u.engagement.unwrap_or(Engagement::FireAtWill),
                    mounts,
                    attackers: BTreeSet::new(),
                    reach: Default::default(),
                    hp: t.hull().map_or(0.0, |h| h.hp),
                    suppression: 0.0,
                    suppressed_at: 0,
                    deployment: deployment::initial(t, &rules),
                    garrison: None,
                    stock: t.capabilities.supply.map(|s| u.stock.unwrap_or(s.stock)),
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
            })
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
            authored_props: world.props().count() as PropId,
            world,
            rules,
            seed,
            tick: 0,
            units,
            sides: Default::default(),
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
            opponent: setup.opponent.clone().map(|o| (o, Defender::default())),
            referee: setup.encounter.clone().map(|e| (e, Referee::default())),
            encounter: None,
            pending: Vec::new(),
            accepted: Vec::new(),
            replaying: None,
            observations: Default::default(),
            scenario_digest: scenario_digest(setup),
            config_digest: config_digest(setup),
            expiries: BTreeMap::new(),
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
        if format!("{:016x}", scenario_digest(setup)) != replay.scenario_digest {
            return Err(ReplayError::ScenarioMismatch);
        }
        if format!("{:016x}", config_digest(setup)) != replay.config_digest {
            return Err(ReplayError::ConfigMismatch);
        }
        let mut battle = Battle::new(setup, replay.seed);
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
        let ack = |error| CommandAck {
            seq: command.seq,
            applied_tick,
            error,
        };
        if self.replaying.is_some() {
            return ack(Some(OrderError::ReplayInProgress));
        }
        self.admit(command.clone(), applied_tick)
            .map_or_else(|e| ack(Some(e)), |_| ack(None))
    }

    fn admit(&mut self, command: CommandEnvelope, applied_tick: Tick) -> Result<(), OrderError> {
        let side = command.side.index();
        if command.seq != self.next_seq[side] {
            return Err(OrderError::OutOfSequence {
                expected: self.next_seq[side],
            });
        }
        self.next_seq[side] += 1;
        self.accepted.push((applied_tick, command.clone()));
        self.validate(&command)?;
        self.pending.push(command);
        Ok(())
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

    fn validate(&self, command: &CommandEnvelope) -> Result<(), OrderError> {
        let units = match &command.order {
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
                    c.side == command.side
                        && matches!(c.order, Order::Garrison { building: b, .. } if self.world.remembered_structure_owner(b) == self.world.remembered_structure_owner(*building))
                });
                return garrison::validate(
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
        self.tick += 1;
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
            let script = self.scripts.pop_front().unwrap();
            let command = CommandEnvelope {
                side: script.side,
                seq: 0,
                order: script.order,
                queued: script.queued,
            };
            if self.validate(&command).is_ok() {
                self.pending.push(command);
            }
        }
        for command in std::mem::take(&mut self.pending) {
            self.apply(command);
        }
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
        let ctx = MovementContext {
            world: &self.world,
            ground: &self.ground,
            ground_rules: &self.rules.ground,
            authored: self.authored_props,
            tick: self.tick,
            tick_hz: self.rules.tick_hz,
            infantry: &self.rules.infantry_movement,
            soldier_radius_m: self.rules.physics.soldier_radius_m,
            seed: self.seed,
            rules: &self.rules,
            knowledge: &self.knowledge,
            arsenal: &self.arsenal,
        };
        let shoves = movement::advance(&ctx, &mut self.units, &mut self.sides);
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
        self.guide(&moved);
        self.fly(&before, &after);
        // Where each unit looks, before this tick's fire turns any turret.
        sight::snapshot(&mut self.units, &self.arsenal);
        for side in Side::ALL {
            self.sense(side, false);
            if self.tick % FOG_INTERVAL_TICKS == side.index() as u64 * FOG_INTERVAL_TICKS / 2 {
                self.sweep_fog(side);
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
        self.observe_all();
        self.opponent_turn();
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
                self.knock_down(prop, s.by);
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

    /// A vehicle of side `by` knocked a tree down (Q16): its body goes, and
    /// its side replans without it at once (contact). Every other side
    /// keeps it standing until it sees the ground where it stood (L1).
    fn knock_down(&mut self, prop: crate::world::Prop, by: Side) {
        self.world.knock_down(prop.id);
        self.keep_standing(&prop, Some(by));
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
    /// each sees it gone once it sees where it stood. Returns units destroyed.
    fn destroy_prop(&mut self, id: PropId) -> Vec<UnitId> {
        let Some(owner) = self.world.structure_owner(id) else {
            return Vec::new();
        };
        let parts = self.world.structure_parts(owner);
        let replacements: Vec<_> = parts
            .into_iter()
            .filter_map(|part| self.destroy_part(part).map(|new| (part, new)))
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

    fn destroy_part(&mut self, id: PropId) -> Option<PropId> {
        let prop = self.world.prop(id).cloned()?;
        let state = self.world.prop_type(prop.kind).destroyed.clone()?;
        self.keep_standing(&prop, None);
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
                height_m,
            } => {
                self.world.remove_prop(id);
                let remains = self.add_prop(&PropDefinition {
                    kind: remains,
                    center: [prop.center.x, prop.center.y],
                    yaw: prop.yaw,
                    half_extents: [prop.half.x, prop.half.y, height_m / 2.0],
                    base_z: Some(prop.base_z),
                });
                self.structures.note_replaced(remains, id);
                self.world.note_replacement(remains, id);
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
        damage::blast_props(&self.world, &w.def, at, None, &mut structural);
        self.ground
            .burst(&self.world, at, w.def.blast_radius_m, &self.rules.ground);
        for (prop, amount) in structural {
            if self.structures.damage(&self.world, prop, amount) {
                self.destroy_prop(prop);
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
        let Some((op, defender)) = self.opponent.as_mut() else {
            return;
        };
        let side = op.side;
        let orders = defender.decide(op, &self.observations[side.index()], &self.rules);
        for order in orders {
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
            units: &self.units,
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
    /// its track, while an unseen death discloses nothing.
    fn consequences(&mut self, outcome: damage::Outcome) {
        for (victim, shooter) in outcome.attacked {
            self.units[victim.0 as usize].attackers.insert(shooter);
        }
        let mut destroyed = outcome.destroyed;
        // Structural damage in event order; a prop is destroyed once (L10, Q17).
        for (prop, amount) in outcome.structural {
            if self.structures.damage(&self.world, prop, amount) {
                destroyed.extend(self.destroy_prop(prop));
            }
        }
        for id in destroyed {
            let unit = &self.units[id.0 as usize];
            let wreck = unit.unit_type(&self.rules).hull().map(|h| h.wreck.clone());
            if let (Some(half), Some(kind)) = (unit.hull, wreck) {
                let def = PropDefinition {
                    kind,
                    center: [unit.position.x, unit.position.y],
                    yaw: unit.yaw,
                    half_extents: [half.x, half.y, half.z],
                    base_z: Some(unit.position.z),
                };
                self.add_prop(&def);
            }
            let unit = &self.units[id.0 as usize];
            for side in Side::ALL.into_iter().filter(|&s| s != unit.side) {
                let knowledge = &mut self.knowledge[side.index()];
                if knowledge.identifies(id, self.tick - 1) {
                    knowledge.saw_destroyed(id);
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
                            unit.pursuit = Some(track.position.xy());
                        }
                        false
                    }
                    None => match *last_known {
                        Some(p) if (unit.position.xy() - p).length() > PURSUIT_ARRIVAL_M => {
                            unit.pursuit = Some(p);
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
                knowledge.track(a).is_some()
                    || knowledge.all_contacts().iter().any(|c| c.emitter == a)
            });
        }
    }

    /// Guided missiles (P05, P06): a launcher supports its missile while it
    /// stands still, has a living operator (including a survivor who takes
    /// over the launcher) and identifies the target with its own sensors, and
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
            for mount in &mut unit.mounts {
                let Some(s) = mount.support else { continue };
                let target = match s.target {
                    Target::Unit(t) => Some(t),
                    _ => None,
                };
                let sighting = target
                    .filter(|&t| alive[t.0 as usize] && knowledge.own_sees(unit.id, t))
                    .and_then(|t| knowledge.track(t).map(|tr| (t, tr.position)));
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
                match sighting.filter(|_| keep) {
                    Some((t, at)) => {
                        self.projectiles
                            .steer(s.projectile, at + v3(0.0, 0.0, aim_z[t.0 as usize]));
                        supported.insert(s.projectile);
                    }
                    None => mount.support = None,
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
            .filter_map(|s| sensing::kept(s, units))
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
        self.occlusion.refresh(&self.world);
        let mut field = self.occlusion.field();
        for unit in self.units.iter().filter(|u| u.side == side && u.alive()) {
            let sight = sight::of(unit, &self.rules);
            for eye in sensing::eyes(unit, &self.rules) {
                visibility::sweep(
                    &self.world,
                    &self.occlusion,
                    &self.rules.sensors,
                    eye,
                    &sight,
                    &mut field,
                );
            }
        }
        // Enemy fallen in view are remembered, and the ground in view learned.
        let knowledge = &mut self.knowledge[side.index()];
        knowledge.learn_ground(&self.ground, &field);
        for u in self.units.iter().filter(|u| u.side != side) {
            for s in &u.members {
                if s.corpse.is_some_and(|f| field.visible(f.at.x, f.at.y)) {
                    knowledge.note_corpse(s.id);
                }
            }
        }
        // Bodies in view are learned where they stand: new ones, and known
        // ones seen moved (L1, L2); trees seen fallen are gone (Q16).
        let known = &mut self.sides[side.index()];
        let revealed: std::collections::BTreeSet<_> = self
            .world
            .building_states()
            .filter(|(_, history, current)| {
                history.iter().any(|id| {
                    known
                        .standing
                        .get(id)
                        .is_some_and(|p| footprint_seen(&field, p))
                }) || current.iter().any(|&id| {
                    self.world
                        .prop(id)
                        .is_some_and(|p| footprint_seen(&field, p))
                })
            })
            .map(|(id, _, _)| id)
            .collect();
        let fallen: Vec<PropId> = known
            .standing
            .values()
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
        for prop in self.world.props() {
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

    fn apply(&mut self, command: CommandEnvelope) {
        let side = command.side;
        let queued = command.queued;
        let push = |unit: &mut Unit, order: UnitOrder| {
            // A new order ends any explicit Pack: once stationary again, set up.
            if let Some(d) = unit.deployment.as_mut() {
                d.stationary = Posture::Deployed;
            }
            if !queued {
                unit.orders.clear();
                unit.route = None;
                unit.planned_goal = None;
                unit.state = MoveState::Idle;
                unit.turn_to = None;
            }
            unit.orders.push_back(order);
        };
        match command.order {
            Order::Move {
                units,
                gesture,
                goal,
                route,
                direction,
                facing,
            } => {
                let destinations = self.group_destinations(side, &units, v2(goal[0], goal[1]));
                for (id, destination) in units.into_iter().zip(destinations) {
                    let order = UnitOrder::Move(MoveOrder {
                        destination,
                        policy: route,
                        gesture,
                        direction,
                        facing: facing.filter(|f| f.is_finite()),
                    });
                    push(&mut self.units[id.0 as usize], order);
                }
            }
            Order::AttackMove {
                units,
                gesture,
                goal,
            } => {
                let destinations = self.group_destinations(side, &units, v2(goal[0], goal[1]));
                for (id, destination) in units.into_iter().zip(destinations) {
                    let unit = &mut self.units[id.0 as usize];
                    unit.engagement = Engagement::FireAtWill; // W14
                    let order = UnitOrder::AttackMove(MoveOrder {
                        destination,
                        policy: contract::command::RoutePolicy::Shortest,
                        gesture,
                        direction: contract::command::MoveDirection::Forward,
                        facing: None,
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
                    let unit = &mut self.units[id.0 as usize];
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
                    let unit = &mut self.units[id.0 as usize];
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
                    push(&mut self.units[id.0 as usize], UnitOrder::Exit);
                }
            }
            Order::SetEngagement { units, policy } => {
                for id in units {
                    self.units[id.0 as usize].engagement = policy;
                }
            }
            Order::Stop { units } => {
                // W15: clear the queue and cancel movement, aim, reload and
                // guidance once; policy stays; automatic fire may restart.
                for id in units {
                    let unit = &mut self.units[id.0 as usize];
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
                    let unit = &mut self.units[id.0 as usize];
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
                for unit in self.units.iter_mut().filter(|u| u.side == side) {
                    for (k, order) in unit.orders.iter_mut().enumerate() {
                        if let Some(m) = order.movement_mut() {
                            if m.gesture == gesture && m.policy != route {
                                m.policy = route;
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

    /// Each unit keeps its place relative to the group where space permits:
    /// offsets from the group centre, compressed to a bounded spread and snapped
    /// to standing room on the side's known map.
    fn group_destinations(&mut self, side: Side, ids: &[UnitId], goal: V2) -> Vec<V2> {
        let positions: Vec<V2> = ids
            .iter()
            .map(|id| self.units[id.0 as usize].position.xy())
            .collect();
        let centre =
            positions.iter().fold(v2(0.0, 0.0), |a, &p| a + p) * (1.0 / positions.len() as f64);
        let spread = positions
            .iter()
            .map(|&p| (p - centre).length())
            .fold(0.0, f64::max);
        let scale = if spread > GROUP_SPREAD_M {
            GROUP_SPREAD_M / spread
        } else {
            1.0
        };
        let radius = self.rules.physics.soldier_radius_m;
        let grid = self.sides[side.index()].grid(&self.world, self.authored_props, radius);
        ids.iter()
            .zip(&positions)
            .map(|(id, &p)| {
                let wanted = goal + (p - centre) * scale;
                let mobility = self.units[id.0 as usize].mobility;
                grid.snap(wanted, &mobility, DESTINATION_SNAP_M)
                    .unwrap_or(goal)
            })
            .collect()
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

    /// The yaw a unit ends its current move at (D2): see `movement::final_yaw`.
    fn final_facing(u: &Unit) -> f64 {
        let Some((goal, _)) = u.movement_goal() else {
            return u.turn_to.unwrap_or(u.yaw);
        };
        let facing = u
            .orders
            .front()
            .and_then(|o| o.movement())
            .and_then(|m| m.facing);
        let route = u.route.as_deref().unwrap_or(&[]);
        let end = route.last().copied().unwrap_or(goal);
        let from = match route.len() {
            0 | 1 => u.position.xy(),
            n => route[n - 2],
        };
        movement::final_yaw(u, facing, from, end).unwrap_or(u.yaw)
    }

    fn observe_all(&mut self) {
        // Every live hull a soldier's current cover may lie behind (D2+).
        let hulls = cover::hull_bodies(&crate::lean::hulls(&self.units, &self.rules));
        for side in Side::ALL {
            let knowledge = &self.knowledge[side.index()];
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
                .filter(|p| p.id < self.authored_props && !known.standing.contains_key(&p.id))
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
                        members: u.member_positions().map(|p| [p.x, p.y, p.z]).collect(),
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
                        final_facing: Self::final_facing(u),
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
                    if let (Some(f), true) = (s.corpse, own || knowledge.knows_corpse(s.id)) {
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
        }
    }

    pub fn observe(&self, side: Side) -> &ObservationFrame {
        &self.observations[side.index()]
    }

    /// Digest of the complete authoritative state at the current tick.
    pub fn digest(&self) -> u64 {
        let mut d = Digest::default();
        d.u64(self.tick);
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
        d.u64(self.world.obstacle_revision());
        // Every body's pose (L3): shoves move them.
        for p in self.world.props() {
            d.u64(p.id as u64)
                .f64(p.center.x)
                .f64(p.center.y)
                .f64(p.yaw)
                .f64(p.base_z);
        }
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
            scenario_digest: format!("{:016x}", self.scenario_digest),
            config_digest: format!("{:016x}", self.config_digest),
            seed: self.seed,
            accepted: self.accepted.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
