//! Derives [`PoseFrame`] from the published feed: one pose per living or
//! dying soldier, one articulation per vehicle, and the corpses.
//!
//! Everything is per soldier: his gait and facing come from his own
//! velocity, never a formation slot's or the squad's heading. The fallen
//! are the one population that can grow into the tens of thousands, so they
//! are reconciled only when the feed hands over a new list. At most
//! `corpses.max` lie drawn: past it the oldest sinks away and is gone,
//! though the simulation still lists him.
mod catalog;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use std::sync::Arc;

use contract::catalog::FlightModel;

use super::delta_angle;
use super::pose::{
    Articulation, ClipBlend, CorpsePose, FadingCorpse, Kick, PoseFrame, SoldierPose, VehiclePose,
};
use crate::seed::{side_key, Mulberry32};
use crate::static_world::ground::{water::StandingWater, Ground};
use crate::world::{Appearance, Clip, UnitType, UnitsSection, Vec3};

mod stance;
pub use stance::StanceFeel;
use stance::{fall_of, lies_after, stance_manner, Posture, Stance};

use crate::GRAVITY;

#[derive(Clone, Debug, Default)]
pub struct FeedSoldier {
    pub id: f64,
    /// His slot in his squad type: which soldier kind he is.
    pub slot: usize,
    /// The weapon he has in use, as published.
    pub active_mount: Option<usize>,
    pub position: Vec3,
    /// The rounds he has launched with his selected weapon (the launches the
    /// flashes and sounds read); a rise is his shot.
    pub shots: f64,
    /// Where his body stands while he leans out past his cover's edge to fire.
    pub lean: Option<[f64; 2]>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FeedMount {
    /// The soldier operating it, as published.
    pub operator: Option<f64>,
    /// World bearing, radians.
    pub bearing: f64,
    pub elevation: f64,
    /// Rounds launched so far; a rise is a shot.
    pub shots: f64,
}

#[derive(Clone, Debug, Default)]
pub struct FeedUnit {
    pub id: f64,
    pub kind: Arc<str>,
    pub side: Arc<str>,
    pub position: Vec3,
    /// Hull heading, radians counter-clockwise from +X.
    pub yaw: f64,
    pub soldiers: Vec<FeedSoldier>,
    pub mounts: Vec<FeedMount>,
    pub deployment: Option<f64>,
    /// The squad is pinned, by the published tier (false where the side cannot know).
    pub pinned: bool,
    /// A downed airframe's (pitch, roll) as it falls, radians: nose up and
    /// right side down. None: a live unit.
    pub attitude: Option<(f64, f64)>,
}

#[derive(Clone, Debug)]
pub struct FeedFallen {
    pub soldier: f64,
    pub position: Vec3,
    /// His squad's heading when he fell.
    pub yaw: f64,
    pub kind: Arc<str>,
    pub slot: usize,
    pub side: Arc<str>,
}

/// One published moment, in simulation seconds.
#[derive(Clone, Debug, Default)]
pub struct FeedFrame {
    pub time: f64,
    pub units: Vec<FeedUnit>,
    /// Everyone the side knows has fallen: the same list while unchanged,
    /// so the driver reconciles corpses only when it changes.
    pub fallen: Arc<Vec<FeedFallen>>,
}

use super::feel::*;

/// Each posture's gait (`INFANTRY_MOVES`), and its clip for each way he
/// goes relative to his facing (`INFANTRY_WAYS`: ahead, left, back, right).
const GAITS: [&str; 3] = ["walk_aim", "crouch", "crawl"];
const MOVES: [[&str; 4]; 3] = [
    ["walk_aim_f", "walk_aim_l", "walk_aim_b", "walk_aim_r"],
    ["crouch_f", "crouch_l", "crouch_b", "crouch_r"],
    ["crawl_f", "crawl_l", "crawl_b", "crawl_r"],
];
/// The ways' bearings from his facing, counter-clockwise.
const WAY_BEARINGS: [f64; 4] = [0.0, FRAC_PI_2, PI, -FRAC_PI_2];
const WAY_NAMES: [&str; 4] = ["f", "l", "b", "r"];
/// How far past the halfway bearing between two ways he keeps the way he is
/// going, so a diagonal does not flicker between two clips, radians.
const WAY_HOLD_RAD: f64 = 0.2;

/// The way (an index of `WAY_BEARINGS`) he goes at bearing `rel` from his
/// facing: the nearest, unless `current` is already a way of `gait` and
/// still near enough.
fn way_of(rel: f64, current: &str, gait: &str) -> usize {
    let held = current.strip_prefix(gait).and_then(|w| w.strip_prefix('_'));
    if let Some(k) = WAY_NAMES.iter().position(|&w| Some(w) == held) {
        if delta_angle(WAY_BEARINGS[k], rel).abs() <= FRAC_PI_4 + WAY_HOLD_RAD {
            return k;
        }
    }
    let mut best = 0;
    let mut off = f64::INFINITY;
    for (k, &at) in WAY_BEARINGS.iter().enumerate() {
        let d = delta_angle(at, rel).abs();
        if d < off {
            (best, off) = (k, d);
        }
    }
    best
}

/// The gait a clip moves in, or None for any other clip.
fn gait_of_clip(clip: &str) -> Option<&str> {
    let gait = clip.rfind('_').map_or("", |cut| &clip[..cut]);
    GAITS.contains(&gait).then_some(gait)
}

/// Where a soldier starts a looping clip: his own offset (the golden ratio
/// over his id), so a squad never moves in lockstep.
pub fn loop_start(soldier: f64) -> f64 {
    (soldier * 0.618_033_988_749_894_9) % 1.0
}

/// How a soldier stands at rest, his own for life (from his id): how far
/// his gaze strays from the squad's aim, how fast his idle plays, and
/// whether he stands watching, weapon up (one man in `watch_every`).
fn rest_manner(soldier: f64, rest: &Rest) -> (f64, f64, bool) {
    let mut state =
        Mulberry32::new(((soldier + 1.0) as i64 as u32).wrapping_mul(crate::seed::GOLDEN));
    let turn = (state.sample() * 2.0 - 1.0) * rest.turn_rad;
    let tempo = rest.tempo[0] + state.sample() * (rest.tempo[1] - rest.tempo[0]);
    (turn, tempo, soldier % rest.watch_every == 2.0)
}

/// The clip `name` as soldier `pose`'s appearance has it (resolved as the
/// blue side's), or None where his rig lacks it.
fn clip_of<'u>(units: &'u UnitsSection, pose: &SoldierPose, name: &str) -> Option<&'u Clip> {
    let a = units.resolve(
        &pose.kind,
        "blue",
        pose.soldier,
        pose.slot,
        pose.operator_mount,
        pose.active_mount,
    )?;
    match units.appearances.get(a)? {
        Appearance::Skinned { clips, .. } => clips.get(name),
        Appearance::Articulated { .. } => None,
    }
}

fn set_clip(pose: &mut SoldierPose, clip: &str) {
    if &*pose.clip != clip {
        pose.clip = clip.into();
    }
}

fn approach(from: f64, to: f64, step: f64) -> f64 {
    let d = to - from;
    if d.abs() <= step {
        to
    } else {
        from + d.signum() * step
    }
}

/// How much of a kick is left `since` seconds after it, recovering over
/// `over` seconds: all of it at once, then going, fast at first and settling.
fn kick_left(since: f64, over: f64) -> f64 {
    if since.is_nan() || since < 0.0 || since >= over {
        return 0.0;
    }
    let left = 1.0 - since / over;
    left * left
}

/// The gun's run-back `since` seconds after a shot: all the way back at
/// once, then out to battery, fast at first and settling.
fn recoil_at(since: f64, m: &MountFeel) -> f64 {
    m.recoil_m * kick_left(since, m.recoil_return_s)
}

/// A hull's springs (`HullKick`): their decay and ringing rates, and the
/// speed a full kick starts them at so that they swing out to 1.
struct Springs {
    decay: f64,
    ring: f64,
    start: f64,
}

impl Springs {
    fn of(kick: &HullKick) -> Self {
        let omega = 2.0 * PI * kick.hz;
        let decay = kick.damping * omega;
        let ring = omega * (1.0 - kick.damping * kick.damping).sqrt();
        // Kicked from rest at speed v, it swings out to (v/ring)·e^(−decay·t)·sin(ring·t),
        // furthest where tan(ring·t) = ring/decay.
        let peak = libm::atan2(ring, decay) / ring;
        let start = ring / (libm::exp(-decay * peak) * libm::sin(ring * peak));
        Self { decay, ring, start }
    }

    /// A damped spring's state (`x`, `v`) run on `dt` seconds, exactly: the
    /// same wherever the frames fall.
    fn ring_on(&self, x: f64, v: f64, dt: f64) -> (f64, f64) {
        let e = libm::exp(-self.decay * dt);
        let c = libm::cos(self.ring * dt);
        let n = libm::sin(self.ring * dt) / self.ring;
        (
            e * (x * c + (v + self.decay * x) * n),
            e * (v * c
                - ((self.ring * self.ring + self.decay * self.decay) * x + self.decay * v) * n),
        )
    }
}

/// A hull's swing below which it is at rest, in kicks.
const KICK_REST: f64 = 1e-4;

/// Ring a hull on the ground on its springs through `dt`, then kick it for
/// each new round its mounts fired since `shots`, by the round's kick, away
/// from where the round went: its motion, or None at rest. `k` is its roof's
/// throw along its own x and y, in kicks, and how fast. The roof swings
/// along its kick: the hull turns about its base, and slides with it.
fn kick_hull(
    k: &mut [f64; 4],
    shots: &[f64],
    unit: &FeedUnit,
    ty: &UnitType,
    feel: &HullKick,
    dt: f64,
) -> Option<[f64; 16]> {
    let springs = Springs::of(feel);
    let [x, y, vx, vy] = k;
    (*x, *vx) = springs.ring_on(*x, *vx, dt);
    (*y, *vy) = springs.ring_on(*y, *vy, dt);
    for (i, m) in unit.mounts.iter().enumerate() {
        let fired = m.shots - shots.get(i).copied().unwrap_or(m.shots);
        let kick = match ty.mounts.get(i) {
            Some(row) if fired > 0.0 => fired * row.kick.hull,
            _ => 0.0,
        };
        if kick <= 0.0 {
            continue;
        }
        let rel = m.bearing - unit.yaw;
        *vx -= libm::cos(rel) * kick * springs.start;
        *vy -= libm::sin(rel) * kick * springs.start;
    }
    let swing = libm::hypot(*x, *y);
    if swing < KICK_REST && libm::hypot(*vx, *vy) < KICK_REST * springs.ring {
        *k = [0.0; 4];
        return None;
    }
    // About the level axis across its swing, the roof thrown along it (just
    // kicked, it has not swung yet: no turn about any axis).
    let across = if swing == 0.0 { 1.0 } else { swing };
    let (s, c) = libm::sincos(feel.rock_deg * PI / 180.0 * swing / 2.0);
    let q = [-*y / across * s, *x / across * s, 0.0, c];
    Some(super::skin::trs(
        [feel.slide_m * *x, feel.slide_m * *y, 0.0],
        q,
        [1.0; 3],
    ))
}

fn sine_in(x: f64) -> f64 {
    1.0 - libm::cos(x * PI / 2.0)
}

fn sine_in_out(x: f64) -> f64 {
    -(libm::cos(PI * x) - 1.0) / 2.0
}

/// Nose up about +Y, then right side down about +X, column-major
/// (`mat4.fromYRotation(-pitch)` then `rotateX(roll)`).
fn tilt(pitch: f64, roll: f64) -> [f64; 16] {
    let (s, c) = libm::sincos(-pitch);
    let (sx, cx) = libm::sincos(roll);
    let y = [0.0, 1.0, 0.0];
    let z = [s, 0.0, c];
    let mut m = [
        c, 0.0, -s, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    for i in 0..3 {
        m[4 + i] = y[i] * cx + z[i] * sx;
        m[8 + i] = z[i] * cx - y[i] * sx;
    }
    m
}

struct SoldierState {
    pose: SoldierPose,
    /// Where the simulation last put him (tucked in): his gait reads this,
    /// never the drawn position, so a lean is never a walk.
    at: Vec3,
    /// His ground velocity, as last measured. Speed is ground over time, so
    /// with no time passed (a paused clock, a frame between ticks) it stands
    /// as it was and so does his gait.
    velocity: [f64; 2],
    /// How far out on his lean he is drawn (0 tucked in, 1 out), and the
    /// point he slides to, kept while he eases back from it.
    lean_t: f64,
    lean_at: Vec3,
    /// The posture he was last posed in: the one he falls from.
    posture: Posture,
    turn: f64,
    tempo: f64,
    watch: bool,
    stance: Stance,
    /// When his squad last fired.
    alert_at: f64,
    fade_left: f64,
    /// His squad's shot counter and his own count, as last seen.
    last_shots: f64,
    own_shots: f64,
    fired_at: f64,
    fell_at: Option<f64>,
    seen: u64,
}

impl SoldierState {
    /// Install `clip` at once, no fade.
    fn reset_clip(&mut self, clip: &str, units: &UnitsSection) {
        set_clip(&mut self.pose, clip);
        self.pose.phase = self.start(units);
        self.pose.blend = None;
        self.fade_left = 0.0;
    }

    /// Move to `clip`, fading out of the current one.
    fn switch_to(&mut self, clip: &str, feel: &PoseFeel, units: &UnitsSection) {
        let pose = &mut self.pose;
        pose.blend = Some(ClipBlend {
            clip: pose.clip.clone(),
            phase: pose.phase,
            weight: 1.0,
        });
        self.fade_left = feel.gait.fade_s;
        set_clip(pose, clip);
        self.pose.phase = self.start(units);
    }

    /// Where his current clip starts: his own offset if it loops.
    fn start(&self, units: &UnitsSection) -> f64 {
        let looping = clip_of(units, &self.pose, &self.pose.clip).is_some_and(|c| c.looping);
        if looping {
            loop_start(self.pose.soldier)
        } else {
            0.0
        }
    }

    /// Move to `clip` (fading from the current one) and advance its phase.
    fn advance(&mut self, clip: &str, moved: f64, dt: f64, feel: &PoseFeel, units: &UnitsSection) {
        if *self.pose.clip != *clip {
            // Turning his way within one gait keeps his step: its ways share a phase.
            let phase = self.pose.phase;
            self.switch_to(clip, feel, units);
            let from = self.pose.blend.as_ref().and_then(|b| gait_of_clip(&b.clip));
            if gait_of_clip(clip).is_some_and(|g| Some(g) == from) {
                self.pose.phase = phase;
            }
        }
        if let Some(facts) = clip_of(units, &self.pose, &self.pose.clip) {
            // Locomotion advances with ground covered, so feet do not slide;
            // everything else with time, the idle at his own tempo.
            let step = match facts.stride {
                Some(stride) if stride != 0.0 => moved / stride,
                _ => {
                    dt / facts.duration
                        * if &*self.pose.clip == "idle" {
                            self.tempo
                        } else {
                            1.0
                        }
                }
            };
            let phase = self.pose.phase + step;
            self.pose.phase = if facts.looping {
                phase % 1.0
            } else {
                phase.min(1.0)
            };
        }
        self.fade(dt, feel);
    }

    fn fade(&mut self, dt: f64, feel: &PoseFeel) {
        let Some(blend) = &mut self.pose.blend else {
            return;
        };
        self.fade_left = (self.fade_left - dt).max(0.0);
        blend.weight = self.fade_left / feel.gait.fade_s;
        if blend.weight <= 0.0 {
            self.pose.blend = None;
        }
    }
}

#[derive(Default)]
struct Flight {
    /// Its drawn ground velocity and acceleration, eased.
    velocity: [f64; 2],
    accel: [f64; 2],
    /// A fixed-wing aircraft's published 3D flight velocity, eased.
    path_velocity: [f64; 3],
    /// The (pitch, roll) they gave it last, held while the clock stands still.
    attitude: Option<(f64, f64)>,
}

struct VehicleState {
    pose: VehiclePose,
    /// Each mount's shot counter as last seen, and when the gun's last shot was.
    shots: Vec<f64>,
    fired_at: f64,
    /// Its hull on its springs (`kick_hull`).
    kick: [f64; 4],
    seen: u64,
    flight: Option<Flight>,
}

/// The static corpses in the order they came to lie, oldest first.
#[derive(Default)]
struct Corpses {
    lying: BTreeMap<u64, CorpsePose>,
    /// Each corpse's place in `lying`, by soldier id bits.
    order: HashMap<u64, u64>,
    next: u64,
    /// Laid since the list was last published: never drawn, so the cap
    /// drops them without a fade.
    unshown: HashSet<u64>,
    /// Soldiers the cap has taken away, fading or gone: never laid again.
    gone: HashSet<u64>,
}

impl Corpses {
    fn lay(&mut self, corpse: CorpsePose) {
        let key = corpse.soldier.to_bits();
        let next = &mut self.next;
        let at = *self.order.entry(key).or_insert_with(|| {
            *next += 1;
            *next
        });
        self.lying.insert(at, corpse);
        self.unshown.insert(key);
    }

    /// Hold the list to `max`, oldest out first: one that was drawn starts
    /// to sink; one never drawn just goes. Then publish it.
    fn publish(&mut self, max: usize, time: f64, out: &mut PoseFrame) {
        for _ in max..self.lying.len() {
            let (_, corpse) = self.lying.pop_first().expect("over the cap");
            let key = corpse.soldier.to_bits();
            self.order.remove(&key);
            self.gone.insert(key);
            if !self.unshown.contains(&key) {
                out.fading.push(FadingCorpse {
                    corpse,
                    sink: 0.0,
                    since: time,
                });
            }
        }
        self.unshown.clear();
        out.corpses.clear();
        out.corpses.extend(self.lying.values().cloned());
        out.corpses_version += 1;
    }
}

pub struct PoseDriver {
    feel: PoseFeel,
    /// Seconds a soldier stays in his firing pose after a shot (`cover.lean_hold_s`).
    lean_hold: f64,
    soldiers: HashMap<u64, SoldierState>,
    /// By side key: an identified enemy's handle can equal an own unit's id.
    vehicles: HashMap<u64, VehicleState>,
    /// Soldiers playing their death, in the order they fell.
    dying: Vec<f64>,
    corpses: Corpses,
    last_fallen: Option<Arc<Vec<FeedFallen>>>,
    time: Option<f64>,
    generation: u64,
    out: PoseFrame,
}

impl PoseDriver {
    /// A driver for `units`' pose feel and lean hold.
    pub fn new(units: &UnitsSection) -> Result<Self, String> {
        let feel = PoseFeel::from_value(&units.pose)?;
        Ok(Self {
            feel,
            lean_hold: units.lean_hold,
            soldiers: HashMap::new(),
            vehicles: HashMap::new(),
            dying: Vec::new(),
            corpses: Corpses::default(),
            last_fallen: None,
            time: None,
            generation: 0,
            out: PoseFrame::default(),
        })
    }

    /// The last frame posed.
    pub fn poses(&self) -> &PoseFrame {
        &self.out
    }

    /// Forget everything: the next frame starts fresh, as if first seen.
    pub fn reset(&mut self) {
        self.soldiers.clear();
        self.vehicles.clear();
        self.dying.clear();
        self.corpses = Corpses::default();
        self.last_fallen = None;
        self.time = None;
        self.out.corpses.clear();
        self.out.corpses_version += 1;
        self.out.fading.clear();
    }

    /// Advance to `frame` and pose everything in it, on `ground`.
    pub fn update(
        &mut self,
        frame: &FeedFrame,
        units: &UnitsSection,
        ground: Option<&Ground>,
    ) -> &PoseFrame {
        let water = ground.map(Ground::standing_water);
        // Time running backwards is a new battle (a reset or a reloaded replay).
        if self.time.is_some_and(|t| frame.time < t) {
            self.reset();
        }
        let dt = self.time.map_or(0.0, |t| (frame.time - t).max(0.0));
        self.time = Some(frame.time);
        self.generation += 1;
        let generation = self.generation;
        self.out.soldiers.clear();
        self.out.vehicles.clear();
        for unit in &frame.units {
            match units.types.get(&*unit.kind) {
                Some(ty) if ty.hull.is_some() => self.vehicle(unit, ty, frame.time, dt, generation),
                _ => self.squad(unit, units, water.as_ref(), frame.time, dt, generation),
            }
        }
        let mut relisted = false;
        if !self
            .last_fallen
            .as_ref()
            .is_some_and(|f| Arc::ptr_eq(f, &frame.fallen))
        {
            self.last_fallen = Some(frame.fallen.clone());
            relisted = self.reconcile_fallen(&frame.fallen, frame.time, units);
        }
        if self.advance_dying(frame.time, dt, generation, units) || relisted {
            self.corpses
                .publish(self.feel.corpses.max, frame.time, &mut self.out);
        }
        let cap = &self.feel.corpses;
        self.out.fading.retain_mut(|f| {
            let t = (frame.time - f.since) / cap.fade_s;
            f.sink = cap.sink_m * sine_in(t);
            t < 1.0
        });
        self.soldiers.retain(|_, s| s.seen == generation);
        let soldiers = &self.soldiers;
        self.dying.retain(|id| soldiers.contains_key(&id.to_bits()));
        self.vehicles.retain(|_, v| v.seen == generation);
        &self.out
    }

    fn squad(
        &mut self,
        unit: &FeedUnit,
        units: &UnitsSection,
        water: Option<&StandingWater>,
        time: f64,
        dt: f64,
        generation: u64,
    ) {
        let feel = &self.feel;
        let gait = &feel.gait;
        for soldier in &unit.soldiers {
            let selected = soldier.active_mount.and_then(|m| unit.mounts.get(m));
            let shots = selected.map_or(0.0, |m| m.shots);
            let aim = selected.map_or(unit.yaw, |m| m.bearing);
            let state = self
                .soldiers
                .entry(soldier.id.to_bits())
                .or_insert_with(|| {
                    let (turn, tempo, watch) = rest_manner(soldier.id, &feel.rest);
                    SoldierState {
                        pose: SoldierPose {
                            soldier: soldier.id,
                            unit: unit.id,
                            kind: unit.kind.clone(),
                            slot: soldier.slot,
                            operator_mount: None,
                            active_mount: None,
                            side: unit.side.clone(),
                            position: soldier.position,
                            facing: unit.yaw + turn,
                            clip: "idle".into(),
                            phase: loop_start(soldier.id),
                            blend: None,
                            kick: Kick::default(),
                        },
                        at: soldier.position,
                        velocity: [0.0; 2],
                        lean_t: 0.0,
                        lean_at: soldier.position,
                        posture: Posture::Stand,
                        turn,
                        tempo,
                        watch,
                        stance: stance_manner(soldier.id, &feel.stance),
                        alert_at: f64::NEG_INFINITY,
                        fade_left: 0.0,
                        // Rounds fired before he was first seen are not his shot.
                        last_shots: shots,
                        own_shots: soldier.shots,
                        fired_at: f64::NEG_INFINITY,
                        fell_at: None,
                        seen: generation,
                    }
                });
            state.seen = generation;
            let carried = unit
                .mounts
                .iter()
                .position(|m| m.operator == Some(soldier.id));
            let dx = soldier.position[0] - state.at[0];
            let dy = soldier.position[1] - state.at[1];
            let moved = libm::hypot(dx, dy);
            if dt > 0.0 {
                state.velocity = [dx / dt, dy / dt];
            }
            let [vx, vy] = state.velocity;
            let speed = libm::hypot(vx, vy);
            let authored_hold = carried
                .and_then(|c| {
                    units
                        .types
                        .get(&*unit.kind)?
                        .mounts
                        .get(c)?
                        .active_pose
                        .as_ref()
                })
                .filter(|_| soldier.active_mount == carried);
            let hold = authored_hold.filter(|_| speed < gait.walk_mps);
            // Presentation equipment only: the feed remains observed authority.
            let active_mount = if authored_hold.is_some() && hold.is_none() {
                None
            } else {
                soldier.active_mount
            };
            let pose = &mut state.pose;
            let equipment_changed = pose.operator_mount != carried
                || (carried.is_some()
                    && (pose.active_mount == carried) != (active_mount == carried));
            if pose.active_mount != active_mount {
                state.fired_at = f64::NEG_INFINITY;
                state.last_shots = shots;
                state.own_shots = soldier.shots;
            }
            pose.operator_mount = carried;
            pose.active_mount = active_mount;
            if soldier.shots > state.own_shots {
                state.fired_at = time;
            }
            if shots > state.last_shots {
                state.alert_at = time;
            }
            state.last_shots = shots;
            state.own_shots = soldier.shots;
            let firing = time - state.fired_at < self.lean_hold;
            // His last shot's kick, by the round his selected weapon fires.
            pose.kick = match (&feel.soldier_kick, soldier.active_mount) {
                (Some(kick), Some(m)) => {
                    let round = units.types.get(&*unit.kind).and_then(|t| t.mounts.get(m));
                    let left = round.map_or(0.0, |r| {
                        r.kick.soldier * kick_left(time - state.fired_at, kick.return_s)
                    });
                    Kick {
                        pitch: kick.pitch_deg * PI / 180.0 * left,
                        back: kick.back_m * left,
                    }
                }
                _ => Kick::default(),
            };

            let wet = || water?.under(soldier.position);
            let lean = soldier.lean.is_some();
            let fast = speed >= gait.crawl_mps;
            let posture = state
                .stance
                .posture(lean, unit.pinned, firing, fast, wet, &feel.stance);
            state.posture = posture;
            let running = speed >= gait.run_mps;
            // Down or shooting, he keeps his weapon toward the threat whichever
            // way he goes; otherwise, and at a run, he faces the way he goes.
            let gait_of =
                (!running && (posture != Posture::Stand || firing)).then_some(posture as usize);

            // Facing: his own velocity, else the weapon's aim (or the unit's
            // heading), strayed by his own manner once the squad has settled.
            let rest = &feel.rest;
            let settled =
                ((time - state.alert_at - rest.settle_s[0]) / rest.settle_s[1]).clamp(0.0, 1.0);
            let travel = libm::atan2(vy, vx);
            let target = if speed > gait.facing_mps && gait_of.is_none() {
                travel
            } else {
                aim + state.turn * settled
            };
            let turn = delta_angle(pose.facing, target);
            let step = gait.turn_rad_s * dt;
            if hold.is_some() {
                pose.facing = aim;
            } else {
                pose.facing += if turn.abs() <= step {
                    turn
                } else {
                    turn.signum() * step
                };
            }

            let clip = if let Some(h) = hold {
                h.clip.as_str()
            } else if running {
                "run"
            } else if speed >= gait.walk_mps {
                match gait_of {
                    Some(g) => {
                        MOVES[g][way_of(delta_angle(pose.facing, travel), &pose.clip, GAITS[g])]
                    }
                    None => "walk",
                }
            } else {
                match posture {
                    Posture::Prone => "prone_pinned",
                    Posture::Kneel => "kneel_fire",
                    Posture::Stand if state.watch || firing => "stand_aim",
                    Posture::Stand => "idle",
                }
            };
            if equipment_changed {
                // Install the new hold directly: never blend a pose sampled
                // from the former equipment.
                state.reset_clip(clip, units);
            }
            if let Some(h) = hold {
                // An active kit is authored against this exact hold.
                set_clip(&mut state.pose, &h.clip);
                state.pose.phase = h.phase;
                state.pose.blend = None;
                state.fade_left = 0.0;
            } else {
                state.advance(clip, moved, dt, feel, units);
            }
            state.at = soldier.position;
            // Out on his lean: slide to the lean point; tucked in: ease back.
            if let Some([x, y]) = soldier.lean {
                state.lean_at = [x, y, soldier.position[2]];
                state.lean_t = (state.lean_t + dt / feel.lean.out_s).min(1.0);
            } else {
                state.lean_t = (state.lean_t - dt / feel.lean.back_s).max(0.0);
            }
            state.pose.position =
                super::lerp3(soldier.position, state.lean_at, sine_in_out(state.lean_t));
            state.pose.unit = unit.id;
            self.out.soldiers.push(state.pose.clone());
        }
    }

    /// A new fallen list: soldiers seen alive start their death; the rest
    /// (and anyone first seen already down) lie as corpses at once. Whether
    /// the static corpses changed.
    fn reconcile_fallen(&mut self, fallen: &[FeedFallen], time: f64, units: &UnitsSection) -> bool {
        let mut changed = false;
        let listed: HashSet<u64> = fallen.iter().map(|f| f.soldier.to_bits()).collect();
        let corpses = &mut self.corpses;
        for f in fallen {
            let key = f.soldier.to_bits();
            if let Some(at) = corpses.order.get(&key) {
                let corpse = corpses.lying.get_mut(at).expect("ordered corpse");
                if corpse.position != f.position {
                    corpse.position = f.position;
                    changed = true;
                }
                continue;
            }
            if corpses.gone.contains(&key) {
                if let Some(c) = self
                    .out
                    .fading
                    .iter_mut()
                    .find(|c| c.corpse.soldier == f.soldier)
                {
                    c.corpse.position = f.position;
                }
                continue;
            }
            match self.soldiers.get_mut(&key) {
                Some(state) if state.fell_at.is_none() => {
                    state.fell_at = Some(time);
                    state.pose.position = f.position;
                    state.pose.unit = -1.0;
                    let changed_hold = state.pose.operator_mount.is_some();
                    state.pose.active_mount = None;
                    state.pose.kick = Kick::default();
                    // He falls from the posture he is in, never getting up to die.
                    let fall = fall_of(state.posture, state.stance);
                    if changed_hold {
                        state.reset_clip(fall, units);
                    } else {
                        state.switch_to(fall, &self.feel, units);
                    }
                    self.dying.push(f.soldier);
                }
                // Authority can move a body while the same death plays.
                Some(state) => state.pose.position = f.position,
                None => {
                    // Never seen fall: he lies as he would have fallen standing.
                    let stance = stance_manner(f.soldier, &self.feel.stance);
                    corpses.lay(CorpsePose {
                        lies: lies_after(fall_of(Posture::Stand, stance)),
                        operator_mount: None,
                        soldier: f.soldier,
                        kind: f.kind.clone(),
                        slot: f.slot,
                        side: f.side.clone(),
                        position: f.position,
                        yaw: f.yaw,
                    });
                    changed = true;
                }
            }
        }
        let before = corpses.lying.len();
        corpses
            .lying
            .retain(|_, c| listed.contains(&c.soldier.to_bits()));
        changed |= corpses.lying.len() != before;
        corpses.order.retain(|k, _| listed.contains(k));
        corpses.unshown.retain(|k| listed.contains(k));
        corpses.gone.retain(|k| listed.contains(k));
        self.out
            .fading
            .retain(|f| listed.contains(&f.corpse.soldier.to_bits()));
        let soldiers = &mut self.soldiers;
        self.dying.retain(|id| {
            let kept = listed.contains(&id.to_bits());
            if !kept {
                soldiers.remove(&id.to_bits());
            }
            kept
        });
        changed
    }

    /// Play each death on; a finished one becomes a static corpse. Whether any did.
    fn advance_dying(&mut self, time: f64, dt: f64, generation: u64, units: &UnitsSection) -> bool {
        let mut laid = false;
        let (soldiers, corpses, out, feel) = (
            &mut self.soldiers,
            &mut self.corpses,
            &mut self.out,
            &self.feel,
        );
        self.dying.retain(|&id| {
            let key = id.to_bits();
            let Some(state) = soldiers.get_mut(&key) else {
                return false;
            };
            state.seen = generation;
            state.pose.active_mount = None;
            let since = time - state.fell_at.unwrap_or(time);
            state.pose.phase = clip_of(units, &state.pose, &state.pose.clip)
                .map_or(1.0, |c| (since / c.duration).min(1.0));
            state.fade(dt, feel);
            if state.pose.phase >= 1.0 && state.pose.blend.is_none() {
                let pose = soldiers.remove(&key).expect("dying soldier").pose;
                corpses.lay(CorpsePose {
                    lies: lies_after(&pose.clip),
                    operator_mount: pose.operator_mount,
                    soldier: id,
                    kind: pose.kind,
                    slot: pose.slot,
                    side: pose.side,
                    position: pose.position,
                    yaw: pose.facing,
                });
                laid = true;
                return false;
            }
            out.soldiers.push(state.pose.clone());
            true
        });
        laid
    }

    fn vehicle(&mut self, unit: &FeedUnit, ty: &UnitType, time: f64, dt: f64, generation: u64) {
        let role = |name: &str| ty.roles.iter().position(|r| r == name);
        let (gun, hmg) = (role("gun"), role("hmg"));
        let gun_mount = gun.and_then(|g| unit.mounts.get(g));
        let hmg_mount = hmg.and_then(|h| unit.mounts.get(h));
        // The published elevations; each model's rig stops its guns at its
        // own pitch limits (`articulate`).
        let gun_target = gun_mount.map_or(0.0, |m| m.elevation);
        let hmg_target = hmg_mount.map_or(0.0, |m| m.elevation);
        let state = self
            .vehicles
            .entry(side_key(unit.id, &*unit.side == "blue").to_bits())
            .or_insert_with(|| {
                // First seen: posed as published, with no shot to recoil from.
                VehicleState {
                    shots: unit.mounts.iter().map(|m| m.shots).collect(),
                    fired_at: f64::NEG_INFINITY,
                    kick: [0.0; 4],
                    pose: VehiclePose {
                        unit: unit.id,
                        kind: unit.kind.clone(),
                        side: unit.side.clone(),
                        position: unit.position,
                        yaw: unit.yaw,
                        airborne: ty.airborne,
                        // Its rotors as a camera's shutter sees them turn.
                        articulation: Articulation {
                            gun_pitch: gun_target,
                            hmg_pitch: hmg_target,
                            rotor_blur: if ty.airborne { 1.0 } else { 0.0 },
                            ..Articulation::default()
                        },
                        motion: None,
                    },
                    seen: generation,
                    flight: None,
                }
            });
        state.seen = generation;
        let feel = &self.feel;
        let pose = &mut state.pose;
        let a = &mut pose.articulation;
        if pose.airborne {
            // In the air its rotors turn and nothing rolls.
            a.rotor += feel.rotor.tip_mps * dt;
        } else {
            // Ground covered along the hull, plus each side's share of the turn.
            let forward = (unit.position[0] - pose.position[0]) * libm::cos(unit.yaw)
                + (unit.position[1] - pose.position[1]) * libm::sin(unit.yaw);
            let turned = delta_angle(pose.yaw, unit.yaw);
            let half = ty.hull.map_or(0.0, |h| h.half_track);
            a.travel_l += forward - turned * half;
            a.travel_r += forward + turned * half;
        }
        let flying = if pose.airborne && unit.attitude.is_none() {
            let f = state.flight.get_or_insert_with(Flight::default);
            aircraft_attitude(f, &feel.attitude, (pose.position, pose.yaw), unit, ty, dt)
        } else {
            None
        };
        pose.position = unit.position;
        pose.yaw = unit.yaw;
        let attitude = unit.attitude.or(flying);
        let a = &mut pose.articulation;
        a.deploy = unit.deployment.unwrap_or(0.0);

        // The turret on the cannon's bearing, the HMG relative to what it
        // rides (a mount, else the hull); elevations eased (they change
        // only on a shot); a new cannon round recoils the gun.
        let carrier = hmg
            .and_then(|h| ty.mounts.get(h)?.on)
            .and_then(|on| unit.mounts.get(on));
        let carrier_bearing = carrier.map_or(unit.yaw, |m| m.bearing);
        a.turret_yaw = gun_mount.map_or(0.0, |m| delta_angle(unit.yaw, m.bearing));
        a.gun_pitch = approach(a.gun_pitch, gun_target, feel.mount.gun_elevation_rad_s * dt);
        a.hmg_yaw = hmg_mount.map_or(0.0, |m| delta_angle(carrier_bearing, m.bearing));
        a.hmg_pitch = approach(a.hmg_pitch, hmg_target, feel.mount.hmg_elevation_rad_s * dt);
        let last = |i: usize| state.shots.get(i).copied();
        if gun_mount.is_some_and(|m| m.shots > gun.and_then(last).unwrap_or(m.shots)) {
            state.fired_at = time;
        }
        a.recoil = recoil_at(time - state.fired_at, &feel.mount);

        // An aircraft leans; a hull on the ground rocks on its springs.
        pose.motion = match (attitude, &feel.hull_kick) {
            (Some((pitch, roll)), _) => Some(tilt(pitch, roll)),
            (None, Some(kick)) if !pose.airborne => {
                kick_hull(&mut state.kick, &state.shots, unit, ty, kick, dt)
            }
            _ => None,
        };
        state.shots.clear();
        state.shots.extend(unit.mounts.iter().map(|m| m.shots));
        self.out.vehicles.push(pose.clone());
    }
}

/// How a winged aircraft follows its published flight path: pitch is the
/// path angle and roll is the bank needed to turn at its published yaw rate.
fn fixed_wing_attitude(
    f: &mut Flight,
    feel: &TiltFeel,
    (was, was_yaw): (Vec3, f64),
    unit: &FeedUnit,
    dt: f64,
) -> Option<(f64, f64)> {
    let TiltFeel {
        gain,
        max_deg,
        smoothing_s,
    } = *feel;
    let k = 1.0 - libm::exp(-dt / smoothing_s);
    let measured = [
        (unit.position[0] - was[0]) / dt,
        (unit.position[1] - was[1]) / dt,
        (unit.position[2] - was[2]) / dt,
    ];
    for (velocity, next) in f.path_velocity.iter_mut().zip(measured) {
        *velocity += (next - *velocity) * k;
    }
    let horizontal = libm::hypot(f.path_velocity[0], f.path_velocity[1]);
    let max = max_deg * PI / 180.0;
    let limit = |angle: f64| (gain * angle).clamp(-max, max);
    let target_pitch = limit(libm::atan2(f.path_velocity[2], horizontal));
    let turn_rate = delta_angle(was_yaw, unit.yaw) / dt;
    let target_roll = limit(-libm::atan2(horizontal * turn_rate, GRAVITY));
    let (old_pitch, old_roll) = f.attitude.unwrap_or_default();
    let pitch = old_pitch + (target_pitch - old_pitch) * k;
    let roll = old_roll + (target_roll - old_roll) * k;
    f.attitude = (pitch.abs() + roll.abs() > 1e-9).then_some((pitch, roll));
    f.attitude
}

/// How a flying aircraft leans, from how its drawn pose (position and yaw)
/// moved since the last frame (`was`). A winged aircraft follows its flight
/// path (see `fixed_wing_attitude`). Any other aircraft leans by its
/// acceleration: nose down to speed up, up to brake, into a turn, by the
/// angle that tilts its lift into its acceleration. Held while the clock
/// stands still; level, it carries no tilt.
fn aircraft_attitude(
    f: &mut Flight,
    feel: &AttitudeFeel,
    was: (Vec3, f64),
    unit: &FeedUnit,
    ty: &UnitType,
    dt: f64,
) -> Option<(f64, f64)> {
    if dt == 0.0 {
        return f.attitude;
    }
    // The flight model, never the layer, decides how it tilts.
    let flight = ty.flight.unwrap_or(FlightModel::Rotor);
    let feel = feel.flight(flight);
    if flight == FlightModel::FixedWing {
        return fixed_wing_attitude(f, feel, was, unit, dt);
    }
    let (was, _) = was;
    let k = 1.0 - libm::exp(-dt / feel.smoothing_s);
    let v = [
        (unit.position[0] - was[0]) / dt,
        (unit.position[1] - was[1]) / dt,
    ];
    let [ox, oy] = f.velocity;
    f.velocity = [ox + (v[0] - ox) * k, oy + (v[1] - oy) * k];
    let a = [(f.velocity[0] - ox) / dt, (f.velocity[1] - oy) / dt];
    f.accel = [
        f.accel[0] + (a[0] - f.accel[0]) * k,
        f.accel[1] + (a[1] - f.accel[1]) * k,
    ];
    let max = feel.max_deg * PI / 180.0;
    let tilt_by = |a: f64| (feel.gain * libm::atan(a / GRAVITY)).clamp(-max, max);
    let (s, c) = libm::sincos(unit.yaw);
    let pitch = -tilt_by(f.accel[0] * c + f.accel[1] * s);
    let roll = -tilt_by(-f.accel[0] * s + f.accel[1] * c);
    f.attitude = (pitch.abs() + roll.abs() > 1e-9).then_some((pitch, roll));
    f.attitude
}
